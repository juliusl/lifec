use super::{
    thunks::{CancelThunk, ErrorContext, SecureClient, StatusUpdate},
    Archive, BlockAddress, Thunk, ThunkContext,
};
use crate::{
    engine::{Connection, Sequence},
    AttributeGraph, AttributeIndex, Engine, Event, Extension, LifecycleOptions, Operation, Runtime, Start,
};
use hyper::Client;
use hyper_tls::HttpsConnector;
use reality::Block;
use specs::{
    shred::SetupHandler, Entities, Entity, Join, Read, ReadStorage, System, World, WorldExt,
    WriteStorage,
};
use std::sync::Arc;
use tokio::sync::{
    self, broadcast,
    mpsc::{self, Sender},
};
use tracing::event;
use tracing::Level;

mod event_listener;
pub use event_listener::EventListener;

/// Event runtime drives the tokio::Runtime and schedules/monitors/orchestrates plugin events
///
#[derive(Default)]
pub struct EventRuntime;

impl Extension for EventRuntime {
    fn configure_app_world(world: &mut specs::World) {
        world.register::<Event>();
        world.register::<ThunkContext>();
        world.register::<CancelThunk>();
        world.register::<ErrorContext>();
        world.register::<crate::Runtime>();
        world.register::<Operation>();
    }

    fn configure_app_systems(dispatcher: &mut specs::DispatcherBuilder) {
        dispatcher.add(EventRuntime::default(), "event_runtime", &[]);
    }
}

/// Setup for tokio runtime, (Not to be confused with crate::Runtime)
impl SetupHandler<tokio::runtime::Runtime> for EventRuntime {
    fn setup(world: &mut specs::World) {
        world.insert(tokio::runtime::Runtime::new().unwrap());

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

/// Setup for tokio-mulitple-producers single-consumer channel for status updates
impl SetupHandler<sync::mpsc::Receiver<StatusUpdate>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = mpsc::channel::<StatusUpdate>(30);
        world.insert(tx);
        world.insert(rx);
    }
}

/// Setup for tokio-broadcast channel for entity updates
impl SetupHandler<sync::broadcast::Receiver<Entity>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = broadcast::channel::<Entity>(100);
        world.insert(rx);
        world.insert(tx);
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
impl SetupHandler<sync::mpsc::Sender<String>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = mpsc::channel::<String>(10);
        world.insert(tx);
        world.insert(rx);
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for status updates
impl SetupHandler<sync::mpsc::Receiver<String>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = mpsc::channel::<String>(10);
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

/// Setup for tokio-mulitple-producers single-consumer channel for status updates
impl SetupHandler<sync::mpsc::Receiver<ErrorContext>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = mpsc::channel::<ErrorContext>(10);
        world.insert(tx);
        world.insert(rx);
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for status updates
impl SetupHandler<sync::mpsc::Receiver<Operation>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = mpsc::channel::<Operation>(10);
        world.insert(tx);
        world.insert(rx);
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for status updates
impl SetupHandler<sync::mpsc::Sender<Operation>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = mpsc::channel::<Operation>(10);
        world.insert(tx);
        world.insert(rx);
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for host start command
impl SetupHandler<sync::mpsc::Receiver<Start>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = mpsc::channel::<Operation>(10);
        world.insert(tx);
        world.insert(rx);
    }
}

/// Setup for tokio-mulitple-producers single-consumer channel for host start command
impl SetupHandler<sync::mpsc::Sender<Start>> for EventRuntime {
    fn setup(world: &mut specs::World) {
        let (tx, rx) = mpsc::channel::<Operation>(10);
        world.insert(tx);
        world.insert(rx);
    }
}

/// Setup for a built-in runtime for the world
impl SetupHandler<super::Runtime> for EventRuntime {
    fn setup(world: &mut World) {
        world.insert(super::Runtime::default());
    }
}

/// Setup for a shared https client
impl SetupHandler<SecureClient> for EventRuntime {
    fn setup(world: &mut World) {
        let https = HttpsConnector::new();
        let client = Client::builder().build::<_, hyper::Body>(https);
        world.insert(client);
    }
}

impl<'a> System<'a> for EventRuntime {
    type SystemData = (
        Read<'a, tokio::runtime::Runtime, EventRuntime>,
        Read<'a, SecureClient, EventRuntime>,
        Read<'a, Sender<StatusUpdate>, EventRuntime>,
        Read<'a, Sender<String>, EventRuntime>,
        Read<'a, Sender<Operation>, EventRuntime>,
        Read<'a, Sender<ErrorContext>, EventRuntime>,
        Read<'a, sync::broadcast::Sender<Entity>, EventRuntime>,
        Entities<'a>,
        ReadStorage<'a, Connection>,
        ReadStorage<'a, Runtime>,
        ReadStorage<'a, AttributeGraph>,
        ReadStorage<'a, Engine>,
        ReadStorage<'a, Block>,
        WriteStorage<'a, Event>,
        WriteStorage<'a, ThunkContext>,
        WriteStorage<'a, Sequence>,
        WriteStorage<'a, CancelThunk>,
        WriteStorage<'a, ErrorContext>,
        WriteStorage<'a, Archive>,
        WriteStorage<'a, BlockAddress>,
        WriteStorage<'a, LifecycleOptions>,
    );

    fn run(
        &mut self,
        (
            runtime,
            https_client,
            status_update_channel,
            dispatcher,
            operation_dispatcher,
            error_dispatcher,
            thunk_complete_channel,
            entities,
            connections,
            lifec_runtimes,
            attribute_graphs,
            engines,
            blocks,
            mut events,
            mut contexts,
            mut sequences,
            mut cancel_tokens,
            mut error_contexts,
            mut archives,
            mut block_addresses,
            mut lifecycle_options,
        ): Self::SystemData,
    ) {
        let mut dispatch_queue = vec![];

        for (entity, _connection, _lifec_runtime, attribute_graph, block, event) in (
            &entities,
            connections.maybe(),
            lifec_runtimes.maybe(),
            attribute_graphs.maybe(),
            blocks.maybe(),
            &mut events,
        )
            .join()
        {
            let event_name = event.to_string();

            // Nit: there is probably a cleaner way to handle this
            let event_ref = Arc::new(event.duplicate());

            let Event(_, thunk, _, initial_context, task) = event;
            if let Some(current_task) = task.take() {
                if current_task.is_finished() {
                    if let Some(thunk_context) = runtime.block_on(async { current_task.await.ok() })
                    {
                        // Commit the current state to previous,
                        let thunk_context = thunk_context.commit();

                        // If the context enabled it's address, add the block address to world storage
                        if thunk_context.socket_address().is_some()
                            && !block_addresses.contains(entity)
                        {
                            if let Some(block_address) = thunk_context.to_block_address() {
                                match block_addresses.insert(entity, block_address) {
                                    Ok(_) => {
                                        event!(
                                            Level::DEBUG,
                                            "inserted new block address for {:?}",
                                            entity
                                        );
                                    }
                                    Err(err) => {
                                        event!(Level::ERROR, "Error inserting block address {err}");
                                    }
                                }
                            }
                        }

                        if let Some(error_context) = thunk_context.get_errors() {
                            event!(Level::ERROR, "plugin error context generated");
                            let thunk_context = thunk_context.clone();

                            if let Some(previous) =
                                error_contexts.insert(entity, error_context.clone()).ok()
                            {
                                if let Some(previous) = previous.and_then(|p| p.fixer()) {
                                    match archives.get_mut(entity) {
                                        Some(archive) => {
                                            if let Some(previous) = archive.0.take() {
                                                let previous_id = previous.id();
                                                match entities.delete(previous) {
                                                    Ok(_) => {
                                                        event!(Level::WARN, "deleting previous fix attempt {previous_id}");
                                                    }
                                                    Err(err) => {
                                                        event!(
                                                            Level::ERROR,
                                                            "error deleting previous entity {err}"
                                                        );
                                                    }
                                                }
                                            }
                                            archive.0 = Some(previous);
                                        }
                                        None => {
                                            archives.insert(entity, Archive(Some(previous))).ok();

                                            if let Some(archiving) = contexts.get(previous) {
                                                runtime.block_on(async {
                                                    archiving.update_status_only("Archived").await
                                                });
                                            }
                                        }
                                    }
                                }

                                runtime
                                    .block_on(async {
                                        let error_dispatcher = error_dispatcher.clone();
                                        error_dispatcher.send(error_context.clone()).await
                                    })
                                    .ok();

                                if error_context.stop_on_error() {
                                    event!(Level::ERROR, "Error detected, and `stop_on_error` is enabled, stopping at {}", entity.id());
                                    let mut clone = thunk_context.clone();

                                    clone.state_mut().with_text(
                                        "thunk_symbol",
                                        format!("Stopped -> {}", thunk.0),
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
                                                event!(
                                                    Level::TRACE,
                                                    "Initialized sequence for {}",
                                                    next_event.id()
                                                );
                                            }
                                        }
                                    } else {
                                        event!(Level::DEBUG, "seqeunce, completed");
                                        if let Some(cursor) = sequence.cursor() {
                                            event!(Level::TRACE, "found cursor {}", cursor.id());
                                            dispatch_queue.push((cursor, thunk_context));
                                        } else {
                                            // Since there isn't a cursor, the lifecycle option decides what should happen next
                                            // Finds the start of an engine
                                            let find_engine =
                                                |start: Entity, engines: &ReadStorage<Engine>| {
                                                    event!(
                                                        Level::TRACE,
                                                        "looking for engine for {:?}",
                                                        start
                                                    );
                                                    if let Some(engine) = engines.get(start) {
                                                        event!(Level::TRACE, "found engine");
                                                        if let Some(start) = engine.start() {
                                                            event!(
                                                                Level::TRACE,
                                                                "found start -> {}",
                                                                start.id()
                                                            );
                                                            return Some(start);
                                                        }
                                                    }
                                                    None
                                                };

                                            if let Some(lifecycle_option) =
                                                lifecycle_options.get_mut(entity)
                                            {
                                                event!(
                                                    Level::DEBUG,
                                                    "found lifecycle option {:?}",
                                                    lifecycle_option
                                                );
                                                match lifecycle_option {
                                                    LifecycleOptions::Repeat {
                                                        remaining,
                                                        start,
                                                    } if *remaining > 0 => {
                                                        *remaining -= 1;
                                                        if let Some(engine) =
                                                            find_engine(*start, &engines)
                                                        {
                                                            dispatch_queue.push((
                                                                engine,
                                                                thunk_context.clone(),
                                                            ));
                                                        }
                                                    }
                                                    LifecycleOptions::Fork(forks) => {
                                                        while let Some(fork) = forks.pop() {
                                                            if let Some(engine) =
                                                                find_engine(fork, &engines)
                                                            {
                                                                dispatch_queue.push((
                                                                    engine,
                                                                    thunk_context.clone(),
                                                                ));
                                                            }
                                                        }

                                                        lifecycle_options
                                                            .insert(
                                                                entity,
                                                                LifecycleOptions::exited(),
                                                            )
                                                            .expect("Should be able to insert");
                                                    }
                                                    LifecycleOptions::Next(next) => {
                                                        if let Some(engine) =
                                                            find_engine(*next, &engines)
                                                        {
                                                            dispatch_queue.push((
                                                                engine,
                                                                thunk_context.clone(),
                                                            ));
                                                        }

                                                        lifecycle_options
                                                            .insert(
                                                                entity,
                                                                LifecycleOptions::exited(),
                                                            )
                                                            .expect("Should be able to insert");
                                                    }
                                                    LifecycleOptions::Loop(next) => {
                                                        if let Some(engine) =
                                                            find_engine(*next, &engines)
                                                        {
                                                            dispatch_queue.push((
                                                                engine,
                                                                thunk_context.clone(),
                                                            ));
                                                        }
                                                    }
                                                    _ => {
                                                        event!(
                                                            Level::DEBUG,
                                                            "exit event:\n\t{} -> exit\n\t{}\n\t{}\n\t{}",
                                                            entity.id(),
                                                            &event_name,
                                                            format!(
                                                                "parent - {}",
                                                                attribute_graph
                                                                    .and_then(|g| Some(g.unscope().entity_id()))
                                                                    .unwrap_or_default()
                                                            ),
                                                            thunk_context.state().hash_code()
                                                        );
                                                        lifecycle_options
                                                            .insert(
                                                                entity,
                                                                LifecycleOptions::exited(),
                                                            )
                                                            .expect("Should be able to insert");
                                                    }
                                                }
                                            }
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
            // An event starts by passing an initial_context to the event
            // the runtime takes this context and configures it before calling the thunk
            } else if let Some(initial_context) = initial_context.take() {
                event!(
                    Level::DEBUG,
                    "start event:\n\t{}\n\t{}\n\t{}\n\t{}",
                    entity.id(),
                    &event_name,
                    format!(
                        "parent - {}",
                        attribute_graph
                            .and_then(|g| Some(g.unscope().entity_id()))
                            .unwrap_or_default()
                    ),
                    initial_context.state().hash_code()
                );
                let thunk = thunk.clone();
                let runtime_handle = runtime.handle().clone();

                let mut context = initial_context
                    .enable_async(entity, runtime_handle)
                    .enable_https_client(https_client.clone())
                    .enable_dispatcher(dispatcher.clone())
                    .enable_operation_dispatcher(operation_dispatcher.clone())
                    .enable_status_updates(status_update_channel.clone())
                    .to_owned();

                if let Some(graph) = attribute_graph {
                    event!(Level::TRACE, "Adding attribute graph to context for {}", entity.id());
                    context = context.with_state(graph.clone());
                }

                if let Some(block) = block {
                    event!(Level::TRACE, "Adding block to context for {}", entity.id());
                    context = context.with_block(block);
                }

                // TODO: This might be a good place to refactor w/ v2 operation
                let Thunk(thunk_name, thunk, ..) = thunk;

                event_ref.setup(&mut context);

                if let Some((handle, cancel_token)) = thunk(&mut context) {
                    match cancel_tokens.insert(entity, CancelThunk::from(cancel_token)) {
                        Ok(existing) => {
                            // If an existing cancel token existed, send a message now
                            if let Some(CancelThunk(cancel)) = existing {
                                event!(Level::TRACE, "swapping cancel token for: {:?}", entity);
                                cancel.send(()).ok();
                            }

                            let mut started = context.clone();
                            started
                                .state_mut()
                                .with_text("thunk_symbol", format!("Running -> {}", thunk_name));

                            // Initializes and starts the task by spawning it on the runtime
                            *task = Some(runtime.spawn(async move {
                                context
                                    .update_status_only(format!(
                                        "# event received: {}, {}",
                                        &event_name,
                                        initial_context.state().hash_code()
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
                                            .state_mut()
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
                    event!(
                        Level::TRACE,
                        "Task didn't start, which means the thunk has already completed"
                    );
                }
            }
        }

        // dispatch all queued messages
        while let Some((next, last)) = dispatch_queue.pop() {
            if let Some(event) = events.get_mut(next) {
                let last_id = last.state().entity_id();

                // Is this an intentional loop?
                if last_id == next.id()
                    && lifecycle_options
                        .get(next)
                        .and_then(|o| match o {
                            LifecycleOptions::Loop(_) => Some(false),
                            LifecycleOptions::Repeat { remaining, .. } if *remaining > 0 => {
                                Some(false)
                            }
                            LifecycleOptions::Repeat { remaining, .. } if *remaining == 0 => {
                                Some(false)
                            }
                            LifecycleOptions::Once => Some(false),
                            _ => Some(true),
                        })
                        .unwrap_or_default()
                {
                    event!(Level::WARN, "Loop detected, setting lifecycle to exit");
                    lifecycle_options
                        .insert(next, LifecycleOptions::exited())
                        .expect("should be able to insert");
                    continue;
                }

                event.fire(last.clone());
                event!(
                    tracing::Level::DEBUG,
                    "dispatch event:\n\t{} -> {}\n\t{}\n\t{}",
                    last_id,
                    next.id(),
                    event,
                    last.state().hash_code()
                );
            } else {
                event!(Level::WARN, "Next event does not exist");
            }
        }
    }
}
