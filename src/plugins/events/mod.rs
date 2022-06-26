use std::fmt::Display;

use atlier::system::Extension;
use atlier::system::WindowEvent;
use imgui::Ui;
use specs::{shred::SetupHandler, Component, Entities, Join, Read, System, WorldExt, WriteStorage, Entity};
use tokio::{
    runtime:: Runtime,
    task::JoinHandle, sync::{self, mpsc::{self, Sender}},
};

use super::thunks::StatusUpdate;
use super::{Plugin, Thunk, ThunkContext};
use specs::storage::VecStorage;
use specs::storage::HashMapStorage;

/// The event component allows an entity to spawn a task for thunks, w/ a tokio runtime instance
#[derive(Component)]
#[storage(VecStorage)]
pub struct Event(
    &'static str,
    Thunk,
    Option<ThunkContext>,
    Option<JoinHandle<ThunkContext>>,
);

impl Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_", self.0)?;
        write!(f, "{}", self.1.0)?;
        Ok(())
    }
}

impl Event {
    /// creates an event component, with a task created with on_event
    /// a handle to the tokio runtime is passed to this function to customize the task spawning
    pub fn from_plugin<P>(
        event_name: &'static str,
    ) -> Self
    where
        P: Plugin<ThunkContext> + Component + Default + Send,
    {
        Self(event_name, Thunk::from_plugin::<P>(), None, None)
    }

    /// "fire" the event, abort any previously running tasks
    pub fn fire(&mut self, thunk_context: ThunkContext) {
        self.2 = Some(thunk_context);

        if let Some(task) = self.3.as_mut() {
            eprintln!("aborting existing task");
            task.abort();
        }
    }

    /// returns true if task is running
    pub fn is_running(&self) -> bool {
        self.3.is_some()
    }
}
/// Event runtime handles various system related tasks, such as the progress system
/// and scheduling tasks on tokio
#[derive(Default)]
pub struct EventRuntime;

impl Extension for EventRuntime {
    fn configure_app_world(world: &mut specs::World) {
        world.register::<Event>();
        world.register::<ThunkContext>();
        world.register::<Progress>();
    }

    fn configure_app_systems(dispatcher: &mut specs::DispatcherBuilder) {
        dispatcher.add(EventRuntime::default(), "event_runtime", &[]);
    }

    fn on_ui(&'_ mut self, _: &specs::World, _: &'_ imgui::Ui<'_>) {
        // No-op
    }

    fn on_window_event(&'_ mut self, _: &specs::World, _: &'_ WindowEvent<'_>) {
        // No-op
    }

    fn on_run(&'_ mut self, app_world: &specs::World) {
        let mut rx = app_world.write_resource::<tokio::sync::mpsc::Receiver<StatusUpdate>>();
        let mut progress = app_world.write_storage::<Progress>();
 
        if let Some(msg) =  rx.try_recv().ok() {
             match progress.insert(msg.0, Progress(msg.1, msg.2)) {
                 Ok(_) => {},
                 Err(_) => {},
             }
        }
    }
}

impl SetupHandler<Runtime> for EventRuntime {
    fn setup(world: &mut specs::World) {
        world.insert(Runtime::new().unwrap());
    }
}

#[derive(Component, Clone)]
#[storage(HashMapStorage)]
pub struct Progress(f32, String);

impl Progress {
    pub fn show(&self, ui: &Ui) {
        imgui::ProgressBar::new(self.0) .overlay_text(self.1.to_string()).build(ui);
    }
}

pub struct ProgressBar(pub Sender<StatusUpdate>);

impl ProgressBar {
    pub async fn update(&self, entity: Entity, status: impl AsRef<str>, progress: f32) {
        let ProgressBar(sender) = self;

        match sender.send((entity, progress, status.as_ref().to_string())).await {
            Ok(_) => {},
            Err(_) => {},
        }
    }
}

impl SetupHandler<sync::mpsc::Sender<StatusUpdate>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = mpsc::channel::<StatusUpdate>(30);
        world.insert(tx);
        world.insert(rx);
    }
}

impl<'a> System<'a> for EventRuntime {
    type SystemData = (
        Read<'a, Runtime, EventRuntime>,
        Read<'a, Sender<StatusUpdate>, EventRuntime>,
        Entities<'a>,
        WriteStorage<'a, Event>,
        WriteStorage<'a, ThunkContext>,
    );

    fn run(&mut self, (runtime, status_sender, entities, mut events, mut contexts): Self::SystemData) {
        for (entity, event) in (&entities, &mut events).join() {
            let event_name = event.to_string();
            let Event(_, thunk, initial_context, task) = event;
            if let Some(current_task) = task.take() {
                if current_task.is_finished() {
                    if let Some(thunk_context) =
                        runtime.block_on(async move { current_task.await.ok() })
                    {
                        match contexts.insert(entity, thunk_context) {
                            Ok(_) => {
                                println!("completed: {}", &event_name);
                            }
                            Err(err) => {
                                eprintln!("error completing: {}, {}", &event_name, err);
                            }
                        }
                    }
                } else {
                    *task = Some(current_task);
                }
            } else if let Some(initial_context) = initial_context.take() {
                println!("begin event: {}, {}", &event_name, initial_context.as_ref().hash_code());
                let thunk = thunk.clone();
                let status_sender = status_sender.clone();
                let runtime_handle = runtime.handle().clone();
                *task = Some(runtime.spawn(async move { 
                    let Thunk(.., thunk) = thunk.clone();
                    let status_updates = status_sender.clone();
                    let mut initial_context = initial_context.clone();
                    initial_context.enable_async(entity, runtime_handle, Some(status_updates));

                    if let Some(handle) = thunk(&mut initial_context) {
                        match handle.await {
                            Ok(_) => {},
                            Err(_) => {},
                        }
                    } else {
                        println!("event completed: {}", &event_name);
                    }
                    initial_context
                }));
            }
        }
    }
}
