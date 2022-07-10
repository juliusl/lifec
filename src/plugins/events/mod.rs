use specs::Entity;
use specs::ReadStorage;
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

use crate::Extension;

use super::Project;
use super::thunks::CancelThunk;
use super::thunks::StatusUpdate;
use super::{Plugin, Thunk, ThunkContext};
use crate::plugins::thunks::Config;
use specs::storage::VecStorage;

mod listen;
pub use listen::Listen;

mod sequence;
pub use sequence::Sequence;
pub use sequence::Connection;

/// The event component allows an entity to spawn a task for thunks, w/ a tokio runtime instance
#[derive(Component)]
#[storage(VecStorage)]
pub struct Event(
    &'static str,
    Thunk,
    Option<Config>,
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
    /// Creates an event component, with a task created with on_event
    /// a handle to the tokio runtime is passed to this function to customize the task spawning
    pub fn from_plugin<P>(event_name: &'static str) -> Self
    where
        P: Plugin<ThunkContext> + Default + Send,
    {
        Self(event_name, Thunk::from_plugin::<P>(), None, None, None)
    }

    /// Sets the config to use w/ this event
    pub fn set_config(&mut self, config: Config) {
        self.2 = Some(config);
    }

    /// Prepares an event for the event runtime to start, cancel any previous join_handle
    pub fn fire(&mut self, thunk_context: ThunkContext) {
        self.3 = Some(thunk_context);

        // cancel any current task
        self.cancel();
    }

    /// Cancel the existing join handle, mainly used for housekeeping.
    /// Thunks must manage their own cancellation by using the cancel_source.
    pub fn cancel(&mut self) {
        if let Some(task) = self.4.as_mut() {
            eprintln!("cancelling existing join_handle");
            task.abort();
        }
    }

    /// returns true if task is running
    pub fn is_running(&self) -> bool {
        self.4
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

/// Event runtime drives the tokio::Runtime and schedules/monitors/orchestrates task entities
#[derive(Default)]
pub struct EventRuntime;

impl Extension for EventRuntime {
    fn configure_app_world(world: &mut specs::World) {
        world.register::<Event>();
        world.register::<ThunkContext>();
        world.register::<CancelThunk>();
    }

    fn configure_app_systems(dispatcher: &mut specs::DispatcherBuilder) {
        dispatcher.add(EventRuntime::default(), "event_runtime", &[]);
    }
}

/// Setup for tokio runtime, (Not to be confused with crate::Runtime)
impl SetupHandler<Runtime> for EventRuntime {
    fn setup(world: &mut specs::World) {
        world.insert(Runtime::new().unwrap());

        // TODO: setup shutdown hook
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
        Read<'a, Project>,
        Entities<'a>,
        ReadStorage<'a, Connection>,
        WriteStorage<'a, Event>,
        WriteStorage<'a, ThunkContext>,
        WriteStorage<'a, Sequence>,
        WriteStorage<'a, CancelThunk>,
    );

    fn run(
        &mut self,
        (
            runtime,
            status_update_channel,
            thunk_complete_channel,
            project,
            entities,
            connections,
            mut events,
            mut contexts,
            mut sequences,
            mut cancel_tokens,
        ): Self::SystemData,
    ) {
        let mut dispatch_queue = vec![];

        for (entity, _connection, event) in (&entities, connections.maybe(), &mut events).join() {
            let event_name = event.to_string();
            let Event(_, thunk, _, initial_context, task) = event;
            if let Some(current_task) = task.take() {
                if current_task.is_finished() {
                    if let Some(thunk_context) = runtime.block_on(async { current_task.await.ok() }) {
                        match contexts.insert(entity, thunk_context.clone()) {
                            Ok(_) => {
                                thunk_complete_channel.send(entity).ok();

                                // if the entity has a sequence, dispatch the next event
                                if let Some(sequence) = sequences.get(entity) {
                                    let mut next = sequence.clone();
                                    if let Some(next_event) = next.next() {
                                        match sequences.insert(next_event, next.clone()).ok() {
                                            Some(_) => {
                                                dispatch_queue.push((next_event, thunk_context));
                                            }
                                            None => {}
                                        }
                                    } else {
                                        eprintln!("-- seqeunce, completed");
                                        if let Some(cursor) = sequence.cursor() {
                                            eprintln!("-- found cursor {}", cursor.id());
                                            dispatch_queue.push((cursor, thunk_context));
                                        }
                                    }
                                }
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
                    "start event:\n\t{}\n\t{}\n\t{}\n\t{}",
                    entity.id(),
                    initial_context.block.block_name,
                    &event_name,
                    initial_context.as_ref().hash_code()
                );
                let thunk = thunk.clone();
                let status_sender = status_update_channel.clone();
                let runtime_handle = runtime.handle().clone();
                let mut context =
                    initial_context.enable_async(
                        entity, 
                        runtime_handle, 
                        Some(project.reload_source()), 
                        Some(status_sender)
                    );

                let Thunk(thunk_name, thunk) = thunk;
                // TODO it would be really helpful to add a macro for these status updates
                // OR could implement AsyncWrite, so you can do:
                // ``` writeln!(context, "# event received", &event_name, hash_code).await.ok();

                if let Some((handle, cancel_token)) = thunk(&mut context) {
                    match cancel_tokens.insert(entity, CancelThunk::from(cancel_token)) {
                        Ok(existing) => {
                            // If an existing cancel token existed, send a message now
                            if let Some(CancelThunk(cancel)) = existing {
                                eprintln!("swapping cancel token for: {:?}", entity);
                                cancel.send(()).ok();
                            }

                            let mut started = context.clone();                            
                            started.as_mut()
                                .with_text(
                                    "thunk_symbol", 
                                    format!("Running -> {}", thunk_name)
                                );

                            // Initializes and starts the task by spawning it on the runtime
                            *task = Some(runtime.spawn(async move {
                                context
                                    .update_status_only(format!(
                                        "# event received: {}, {}",
                                        &event_name,
                                        initial_context.as_ref().hash_code()
                                    ))
                                    .await;

                                match handle.await {
                                    Ok(mut updated_context) => {
                                        context
                                            .update_status_only(format!(
                                                "# completed: {}",
                                                &event_name
                                            ))
                                            .await;
                                        updated_context
                                            .as_mut()
                                            .add_text_attr("thunk_symbol", thunk_name);
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
                            }));

                            contexts.insert(entity, started).ok();
                        }
                        Err(_) => {}
                    }
                } else {
                    eprintln!("Task didn't start, which means the thunk has already completed");
                }
            }
        }

        // dispatch all queued messages
        loop {
            match dispatch_queue.pop() {
                Some((next, last)) => {
                    if let (Some(event), Some(context)) =
                        (events.get_mut(next), contexts.get_mut(next))
                    {
                        let last_id = last.as_ref().entity();
                        let previous = last.project
                                .and_then(|p| p.transpile_blocks().ok())
                                .unwrap_or_default()
                                .trim()
                                .to_string();

                        if !previous.trim().is_empty() {
                            context.as_mut().add_message(
                                event.to_string(),
                                "previous",
                                previous,
                            );
                        }

                        event.fire(context.clone());
                        println!(
                            "dispatch event:\n\t{} -> {}\n\t{}\n\t{}\n\t{}",
                            last_id,
                            next.id(),
                            context.block.block_name,
                            event,
                            context.as_ref().hash_code()
                        );
                    } else {
                        eprintln!("Next event does not exist");
                    }
                }
                None => break,
            }
        }
    }
}
