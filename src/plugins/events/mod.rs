use atlier::system::Extension;
use specs::{
    shred::SetupHandler, Component, Entities, Join, Read, System, WorldExt, WriteStorage,
};
use tokio::{
    runtime::{Handle, Runtime},
    task::JoinHandle,
};

use super::{Plugin, Thunk, ThunkContext};
use specs::storage::VecStorage;

/// The event component allows an entity to spawn a task for thunks, w/ a tokio runtime instance
#[derive(Component)]
#[storage(VecStorage)]
pub struct Event(
    &'static str,
    fn(&Thunk, &ThunkContext, &Handle) -> JoinHandle<ThunkContext>,
    Thunk,
    Option<ThunkContext>,
    Option<JoinHandle<ThunkContext>>,
);

impl Event {
    /// creates an event component, wrapping the thunk call in a tokio task
    pub fn from_plugin<P>(event_name: &'static str) -> Self
    where
        P: Plugin<ThunkContext> + Component + Default + Send,
        <P as Component>::Storage: Default
    {
        Self::from_plugin_with::<P>(event_name, |thunk, initial_context, handle| {
            let thunk = thunk.clone();
            let initial_context = initial_context.clone();
            handle.spawn(async move {
                let mut context = initial_context;
                thunk.call(&mut context);
                context
            })
        })
    }

    /// creates an event component, with a task created with on_event
    /// a handle to the tokio runtime is passed to this function to customize the task spawning
    pub fn from_plugin_with<P>(
        event_name: &'static str,
        on_event: fn(&Thunk, &ThunkContext, &Handle) -> JoinHandle<ThunkContext>,
    ) -> Self
    where      
        P: Plugin<ThunkContext> + Component + Default + Send,
        <P as Component>::Storage: Default
    {
        Self(event_name, on_event, Thunk::from_plugin::<P>(), None, None)
    }

    /// "fire" the event, abort any previous tasks
    pub fn fire(&mut self, thunk_context: ThunkContext) {
        self.3 = Some(thunk_context);

        if let Some(task) = self.4.as_mut() {
            task.abort();
        }
    }
}

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

    fn on_ui(&'_ mut self, _: &specs::World, ui: &'_ imgui::Ui<'_>) {
    }
}

impl SetupHandler<Runtime> for EventRuntime {
    fn setup(world: &mut specs::World) {
        world.insert(Runtime::new().unwrap());
    }
}

impl<'a> System<'a> for EventRuntime {
    type SystemData = (
        Read<'a, Runtime, EventRuntime>,
        Entities<'a>,
        WriteStorage<'a, Event>,
        WriteStorage<'a, ThunkContext>,
    );

    fn run(&mut self, (runtime, entities, mut events, mut contexts): Self::SystemData) {
        for (entity, event) in (&entities, &mut events).join() {
            let Event(event_name, on_event, thunk, initial_context, task) = event;
            if let Some(current_task) = task.take() {
                if current_task.is_finished() {
                    if let Some(thunk_context) =
                        runtime.block_on(async move { current_task.await.ok() })
                    {
                        match contexts.insert(entity, thunk_context) {
                            Ok(_) => {
                                println!("completed {}", event_name);
                            }
                            Err(err) => {
                                eprintln!("error completing: {}, {}", event_name, err);
                            }
                        }
                    }
                } else {
                    *task = Some(current_task);
                }
            } else if let Some(initial_context) = initial_context.take() {
                println!("event started {}", event_name);
                let handle = on_event(&thunk, &initial_context, runtime.handle());
                *task = Some(handle);
            }
        }
    }
}
