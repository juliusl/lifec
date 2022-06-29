use atlier::system::Extension;
use specs::Entity;
use specs::World;
use specs::{shred::SetupHandler, Component, Entities, Join, Read, System, WorldExt, WriteStorage};
use std::fmt::Display;
use tokio::sync::broadcast;
use tokio::{
    runtime::Runtime,
    sync::{
        self,
        mpsc::{self, Sender},
    },
    task::JoinHandle,
};

use super::thunks::StatusUpdate;
use super::{Plugin, Thunk, ThunkContext};
use specs::storage::VecStorage;

mod listen;
pub use listen::Listen;

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
        write!(f, "{} ", self.0)?;
        write!(f, "{}", self.1 .0)?;
        Ok(())
    }
}

impl Event {
    /// creates an event component, with a task created with on_event
    /// a handle to the tokio runtime is passed to this function to customize the task spawning
    pub fn from_plugin<P>(event_name: &'static str) -> Self
    where
        P: Plugin<ThunkContext> + Component + Default + Send,
    {
        Self(event_name, Thunk::from_plugin::<P>(), None, None)
    }

    /// "fire" the event, abort any previously running tasks
    pub fn fire(&mut self, thunk_context: ThunkContext) {
        self.2 = Some(thunk_context);

        // cancel any current task
        self.cancel();
    }

    /// cancel any ongoing task spawned by this event
    pub fn cancel(&mut self) {
        if let Some(task) = self.3.as_mut() {
            eprintln!("aborting existing task");
            task.abort();
        }
    }

    /// returns true if task is running
    pub fn is_running(&self) -> bool {
        self.3
            .as_ref()
            .and_then(|j| Some(!j.is_finished()))
            .unwrap_or_default()
    }

    /// subscribe to get a notification when the runtime editor has updated an entity
    pub fn subscribe(world: &World) -> sync::broadcast::Receiver<Entity> {
        let sender = world.write_resource::<sync::broadcast::Sender<Entity>>();

        sender.subscribe()
    }

    /// receive the next updated thunkcontext from world,
    /// Note: this receives from the world's global receiver, to receive full broadcast, use Event::subscribe to get a receiver
    pub fn receive(world: &World) -> Option<ThunkContext> {
        let mut receiver = world.write_resource::<sync::broadcast::Receiver<Entity>>();

        match receiver.try_recv() {
            Ok(entity) => {
                let contexts = world.read_component::<ThunkContext>();
                contexts.get(entity).and_then(|c| Some(c.clone()))
            }
            Err(_) => None,
        }
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
    }

    fn configure_app_systems(dispatcher: &mut specs::DispatcherBuilder) {
        dispatcher.add(EventRuntime::default(), "event_runtime", &[]);
    }
}

/// Setup for tokio runtime, (Not to be confused with crate::Runtime)
impl SetupHandler<Runtime> for EventRuntime {
    fn setup(world: &mut specs::World) {
        world.insert(Runtime::new().unwrap());
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for status updates
impl SetupHandler<sync::mpsc::Sender<StatusUpdate>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = mpsc::channel::<StatusUpdate>(30);
        world.insert(tx);
        world.insert(rx);
    }
}

/// Setup for tokio-broadcast channel for entity updates
impl SetupHandler<sync::broadcast::Sender<Entity>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = broadcast::channel::<Entity>(100);
        world.insert(rx);
        world.insert(tx);
    }
}

impl<'a> System<'a> for EventRuntime {
    type SystemData = (
        Read<'a, Runtime, EventRuntime>,
        Read<'a, Sender<StatusUpdate>, EventRuntime>,
        Read<'a, sync::broadcast::Sender<Entity>, EventRuntime>,
        Entities<'a>,
        WriteStorage<'a, Event>,
        WriteStorage<'a, ThunkContext>,
    );

    fn run(
        &mut self,
        (runtime, status_sender, updated_sender, entities, mut events, mut contexts): Self::SystemData,
    ) {
        for (entity, event) in (&entities, &mut events).join() {
            let event_name = event.to_string();
            let Event(_, thunk, initial_context, task) = event;
            if let Some(current_task) = task.take() {
                if current_task.is_finished() {
                    if let Some(thunk_context) = runtime.block_on(async { current_task.await.ok() })
                    {
                        match contexts.insert(entity, thunk_context) {
                            Ok(_) => {
                                updated_sender.send(entity).ok();
                            }
                            Err(err) => {
                                eprintln!("# error completing: {}, {}", &event_name, err);
                            }
                        }
                    }
                } else {
                    *task = Some(current_task);
                }
            } else if let Some(initial_context) = initial_context.take() {
                println!(
                    "# start event: {}, {}",
                    &event_name,
                    initial_context.as_ref().hash_code()
                );
                let thunk = thunk.clone();
                let status_sender = status_sender.clone();
                let runtime_handle = runtime.handle().clone();
                let mut context =
                    initial_context.enable_async(entity, runtime_handle, Some(status_sender));
                *task = Some(runtime.spawn(async move {
                    let Thunk(thunk_name, thunk) = thunk;
                    context
                        .update_status_only(format!(
                            "# event received: {}, {}",
                            &event_name,
                            initial_context.as_ref().hash_code()
                        ))
                        .await;
                    if let Some(handle) = thunk(&mut context) {
                        context
                            .update_status_only(format!("# {} is being called", thunk_name))
                            .await;

                        match handle.await {
                            Ok(mut updated_context) => {
                                context
                                    .update_status_only(format!("# completed: {}", &event_name))
                                    .await;
                                updated_context.as_mut().add_text_attr("thunk_symbol", thunk_name);
                                updated_context
                            }
                            Err(err) => {
                                context.error(|g| {
                                    g.with_text("event_runtime", format!("{}", err));
                                });
                                context
                                    .update_status_only(format!(
                                        "# event error: {}, {}",
                                        &event_name, err
                                    ))
                                    .await;
                                context
                            }
                        }
                    } else {
                        // This means that the event completed without spawning any tasks
                        context
                            .update_status_only(format!("# completed: {}", &event_name))
                            .await;
                        context
                    }
                }));
            }
        }
    }
}
