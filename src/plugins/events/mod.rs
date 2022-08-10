use hyper::Client;
use hyper_tls::HttpsConnector;
use specs::Entity;
use specs::ReadStorage;
use specs::World;
use specs::{shred::SetupHandler, Component, Entities, Join, Read, System, WorldExt, WriteStorage};
use tracing::Level;
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
use tracing::event;

use crate::AttributeGraph;
use crate::Extension;

use super::Archive;
use super::BlockAddress;
use super::Project;
use super::thunks::CancelThunk;
use super::thunks::ErrorContext;
use super::thunks::StatusUpdate;
use super::{Plugin, Thunk, ThunkContext};
use crate::plugins::thunks::Config;
use specs::storage::VecStorage;

mod proxy_dispatcher;
pub use proxy_dispatcher::ProxyDispatcher;

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
    /// Returns the a clone of the inner thunk
    /// 
    pub fn thunk(&self) -> Thunk {
        self.1.clone()
    }

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
    /// 
    /// Caveats: If the event has a config set, it will configure the context, before setting it
    /// 
    pub fn fire(&mut self, mut thunk_context: ThunkContext) {
        if let Some(Config(name, config)) = self.2 {
            event!(Level::TRACE, "detected config {name} for event: {}", self.0);
            config(&mut thunk_context);
        }

        self.3 = Some(thunk_context);

        // cancel any current task
        self.cancel();
    }

    /// Cancel the existing join handle, mainly used for housekeeping.
    /// Thunks must manage their own cancellation by using the cancel_source.
    pub fn cancel(&mut self) {
        if let Some(task) = self.4.as_mut() {
            event!(Level::TRACE, "cancelling existing join_handle");
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

    /// Creates a duplicate of this event
    pub fn duplicate(&self) -> Self {
        Self(self.0, self.1.clone(), self.2.clone(), None, None)
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
        world.register::<ErrorContext>();
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

/// Setup for tokio-mulitple-producers single-consumer channel for status updates
impl SetupHandler<sync::mpsc::Sender<AttributeGraph>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = mpsc::channel::<AttributeGraph>(10);
        world.insert(tx);
        world.insert(rx);
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for status updates
impl SetupHandler<sync::mpsc::Receiver<AttributeGraph>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = mpsc::channel::<AttributeGraph>(10);
        world.insert(tx);
        world.insert(rx);
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for status updates
impl SetupHandler<sync::mpsc::Sender<ErrorContext>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = mpsc::channel::<ErrorContext>(10);
        world.insert(tx);
        world.insert(rx);
    }
}

/// Setup for a built-in runtime for the world
/// 
/// TODO: Trying to move more things to this runtime-space
/// 
impl SetupHandler<super::Runtime> for EventRuntime {
    fn setup(world: &mut World) {
        world.insert(super::Runtime::default());
    }
}

impl<'a> System<'a> for EventRuntime {
    type SystemData = (
        Read<'a, Runtime, EventRuntime>,
        Read<'a, Sender<StatusUpdate>, EventRuntime>,
        Read<'a, Sender<AttributeGraph>, EventRuntime>,
        Read<'a, Sender<ErrorContext>, EventRuntime>,
        Read<'a, sync::broadcast::Sender<Entity>, EventRuntime>,
        Read<'a, Project>,
        Entities<'a>,
        ReadStorage<'a, Connection>,
        WriteStorage<'a, Event>,
        WriteStorage<'a, ThunkContext>,
        WriteStorage<'a, Sequence>,
        WriteStorage<'a, CancelThunk>,
        WriteStorage<'a, ErrorContext>,
        WriteStorage<'a, Archive>,
        WriteStorage<'a, BlockAddress>,
    );

    fn run(
        &mut self,
        (
            runtime,
            status_update_channel,
            dispatcher,
            error_dispatcher,
            thunk_complete_channel,
            project,
            entities,
            connections,
            mut events,
            mut contexts,
            mut sequences,
            mut cancel_tokens,
            mut error_contexts,
            mut archives,
            mut block_addresses,
        ): Self::SystemData,
    ) {
        let mut dispatch_queue = vec![];

        for (entity, _connection, event) in (&entities, connections.maybe(), &mut events).join() {
            let event_name = event.to_string();
            let Event(_, thunk, _, initial_context, task) = event;
            if let Some(current_task) = task.take() {
                if current_task.is_finished() {
                    if let Some(thunk_context) = runtime.block_on(async { current_task.await.ok() }) {
                        // If the context enabled it's address, add the block address to world storage
                        if thunk_context.socket_address().is_some() && !block_addresses.contains(entity) {
                            if let Some(block_address) = thunk_context.to_block_address() {
                                match block_addresses.insert(entity, block_address) {
                                    Ok(_) => {
                                        event!(Level::DEBUG, "inserted new block address for {:?}", entity);
                                    },
                                    Err(err) => {
                                        event!(Level::ERROR, "Error inserting block address {err}");
                                    },
                                }
                            }
                        }

                        if let Some(error_context) = thunk_context.get_errors() {
                            event!(Level::ERROR, "plugin error context generated");
                            let thunk_context = thunk_context.clone();

                            if let Some(previous) = error_contexts.insert(entity, error_context.clone()).ok() {
                                if let Some(previous) = previous.and_then(|p| p.fixer()) {
                                    match archives.get_mut(entity) {
                                        Some(archive) =>{
                                            if let Some(previous) = archive.0.take() {
                                                let previous_id = previous.id();
                                                match entities.delete(previous) {
                                                    Ok(_) => {
                                                        event!(Level::WARN, "deleting previous fix attempt {previous_id}");
                                                    },
                                                    Err(err) => {
                                                        event!(Level::ERROR, "error deleting previous entity {err}");
                                                    },
                                                }
                                            }
                                            archive.0 = Some(previous);
                                        }
                                        None => {
                                            archives.insert(entity, Archive(Some(previous))).ok();

                                            if let Some(archiving) = contexts.get(previous) {
                                                runtime.block_on( async { archiving.update_status_only("Archived").await });
                                            }
                                        },
                                    }
                                }

                                runtime.block_on(async { error_dispatcher.send(error_context.clone()).await }).ok();

                                if error_context.stop_on_error() {
                                    event!(Level::ERROR, "Error detected, and `stop_on_error` is enabled, stopping at {}", entity.id());
                                    let mut clone = thunk_context.clone();

                                    clone.as_mut().with_text(
                                        "thunk_symbol", 
                                        format!("Stopped -> {}", thunk.0)
                                    );

                                    contexts.insert(entity, clone).ok();
                                    continue;
                                }
                            }
                        }

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
                                            None => {
                                                event!(Level::TRACE, "Initialized sequence for {}", next_event.id());
                                            }
                                        }
                                    } else {
                                        event!(Level::DEBUG, "seqeunce, completed");
                                        if let Some(cursor) = sequence.cursor() {
                                            event!(Level::DEBUG, "found cursor {}", cursor.id());
                                            dispatch_queue.push((cursor, thunk_context));
                                        }
                                    }
                                }
                            }
                            Err(err) => {
                                event!(Level::ERROR, "error completing: {}, {err}", &event_name);
                            }
                        }
                    }
                } else {
                    *task = Some(current_task);
                }
            } else if let Some(initial_context) = initial_context.take() {
                event!(
                    Level::DEBUG,
                    "start event:\n\t{}\n\t{}\n\t{}\n\t{}",
                    entity.id(),
                    initial_context.block.block_name,
                    &event_name,
                    initial_context.as_ref().hash_code()
                );
                let thunk = thunk.clone();
                let runtime_handle = runtime.handle().clone();
                let https = HttpsConnector::new();
                let client = Client::builder().build::<_, hyper::Body>(https);
                
                let mut context =
                    initial_context.enable_async(
                        entity, 
                        runtime_handle,
                        Some(client),
                        Some(project.reload_source()), 
                        Some(status_update_channel.clone()),
                        Some(dispatcher.clone()),
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
                                event!(Level::TRACE, "swapping cancel token for: {:?}", entity);
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
                    event!(Level::TRACE, "Task didn't start, which means the thunk has already completed");
                }
            }
        }

        // dispatch all queued messages
        loop {
            match dispatch_queue.pop() {
                Some((mut next, last)) => {
                    if let Some(true) = connections.get(next).and_then(|c| Some(c.fork_enabled())) {
                        let forked_event = events.get(next).and_then(|e| Some(e.duplicate()));
                        let forked_context = contexts.get(next).and_then(|c| Some(c.clone()));

                        if let (Some(event), Some(context)) = (forked_event, forked_context) {
                            let fork = entities.create();
                            let fork_id = fork.id();
                            let log = format!("Forking, {fork_id}, {event}");
                            if events.insert(fork, event).is_ok() {
                                if contexts.insert(fork, context).is_ok() {
                                    event!(Level::TRACE, "{}", log);
                                    next = fork;
                                }
                            }
                        }
                    }

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
                        event!(
                            tracing::Level::DEBUG,
                            "dispatch event:\n\t{} -> {}\n\t{}\n\t{}\n\t{}",
                            last_id,
                            next.id(),
                            context.block.block_name,
                            event,
                            context.as_ref().hash_code()
                        );
                    } else {
                        event!(Level::WARN, "Next event does not exist");
                    }
                }
                None => break,
            }
        }
    }
}
