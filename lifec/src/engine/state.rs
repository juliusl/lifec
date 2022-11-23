use std::{
    collections::{BTreeMap, HashMap},
    ops::Deref,
    sync::Arc,
};

use reality::Attribute;
use chrono::Utc;
use specs::{prelude::*, Entities, SystemData};
use tokio::sync::oneshot;

use super::{
    connection::ConnectionState, Adhoc, EngineStatus, Limit, Plugins, Profiler, TickControl,
    Transition, Yielding, NodeCommand,
};
use crate::{
    engine::Completion,
    prelude::*, appendix::Appendix,
};

cfg_editor! {
    use crate::editor::{Node, NodeStatus};
}

use tracing::{event, Level};

pub type NodeCommandHandler = fn(&mut State, Entity) -> bool;

/// Engine system data,
///
#[derive(SystemData)]
pub struct State<'a> {
    /// Lazy updates,
    ///
    lazy_update: Read<'a, LazyUpdate>,
    /// Runtime,
    ///
    runtime: Read<'a, Runtime>,
    /// Controls the event tick rate,
    ///
    tick_control: Write<'a, TickControl>,
    /// Appendix stores metadata on entities,
    ///
    appendix: Write<'a, Arc<Appendix>>,
    /// Channel to send error contexts,
    ///
    send_errors: Read<'a, tokio::sync::mpsc::Sender<ErrorContext>, EventRuntime>,
    /// Channel to broadcast completed plugin calls,
    ///
    send_completed: Read<'a, tokio::sync::broadcast::Sender<Entity>, EventRuntime>,
    /// Map of custom node command handlers,
    ///
    handlers: Read<'a, BTreeMap<String, NodeCommandHandler>>,
    /// Lookup entities by label,
    ///
    entity_map: Read<'a, HashMap<String, Entity>>,
    /// Workspace config,
    /// 
    workspace_config: WorkspaceConfig<'a>,
    /// Plugins system data,
    ///
    plugins: Plugins<'a>,
    /// Entity storage,
    ///
    entities: Entities<'a>,
    /// Event transition storage,
    ///
    transitions: ReadStorage<'a, Transition>,
    /// Profiler termination points for adhoc operations,
    ///
    profilers: ReadStorage<'a, Profiler>,
    /// Adhoc operation config,
    ///
    adhocs: ReadStorage<'a, Adhoc>,
    /// Block data,
    ///
    blocks: ReadStorage<'a, Block>,
    /// Thunk storage,
    ///
    thunks: ReadStorage<'a, Thunk>,
    /// Graph storage
    ///
    graphs: ReadStorage<'a, AttributeGraph>,
    /// Entity cursors,
    ///
    cursors: ReadStorage<'a, Cursor>,
    /// Sequences of entities,
    ///
    sequences: ReadStorage<'a, Sequence>,
    /// Event config,
    ///
    events: ReadStorage<'a, Event>,
    /// Connection state storage,
    ///
    connection_states: ReadStorage<'a, ConnectionState>,
    /// Engine storage,
    ///
    engines: ReadStorage<'a, Engine>,
    /// Execution limits,
    ///
    limits: WriteStorage<'a, Limit>,
    /// Connection storage,
    ///
    connections: WriteStorage<'a, Connection>,
    /// Operation storage,
    ///
    operations: WriteStorage<'a, Operation>,
    /// Yielding storage,
    ///
    yielding: WriteStorage<'a, Yielding>,
}

impl<'a> State<'a> {
    /// Returns a reference to lazy update resource,
    /// 
    pub fn lazy_updates(&self) -> &LazyUpdate {
        &self.lazy_update
    }

    /// Returns workspace config state,
    /// 
    pub fn workspace_config(&self) -> &WorkspaceConfig<'a> {
        &self.workspace_config
    }

    /// Returns plugins data,
    ///
    pub fn plugins(&self) -> &Plugins<'a> {
        &self.plugins
    }

    /// Returns a new appendix,
    ///
    pub fn appendix(&self) -> Arc<Appendix> {
        self.appendix.deref().clone()
    }

    /// Returns a list of adhoc operations,
    ///
    pub fn list_adhoc_operations(&self) -> Vec<(Entity, Adhoc, Sequence)> {
        let Self {
            entities,
            blocks,
            adhocs,
            sequences,
            ..
        } = self;

        let mut operations = vec![];

        let root_block = entities.entity(0);

        if let Some(block) = blocks.get(root_block) {
            for operation in block
                .index()
                .iter()
                .filter(|b| b.root().name().ends_with("operation"))
            {
                let operation_entity = operation.root().id();
                let operation_entity = entities.entity(operation_entity);

                if let Some((adhoc, operation)) =
                    (adhocs, sequences).join().get(operation_entity, entities)
                {
                    operations.push((operation_entity, adhoc.clone(), operation.clone()));
                }
            }
        }

        operations
    }

    /// Resets Completed/Cancelled events
    ///
    pub fn reset_all(&mut self) {
        let statuses = self.scan().collect::<Vec<_>>();
        for status in statuses {
            self.reset(status.entity());
        }
    }

    /// Resets a completed/cancelled event,
    ///
    pub fn reset(&mut self, event: Entity) -> bool {
        let status = self.status(event);

        let State {
            sequences,
            events,
            operations,
            ..
        } = self;

        match status {
            EventStatus::Completed(e) | EventStatus::Cancelled(e) => {
                operations.remove(e);

                if let (Some(sequence), Some(event)) = (sequences.get(e), events.get(e)) {
                    let mut next = event.clone();
                    next.reactivate(sequence.clone());
                    self.lazy_update.insert(e, next);
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Returns the event status for an event,
    ///
    pub fn status(&self, entity: Entity) -> EventStatus {
        let State {
            tick_control,
            entities,
            events,
            operations,
            ..
        } = self;

        if let Some((event, operation)) = (events, operations.maybe()).join().get(entity, entities)
        {
            if tick_control.is_paused(entity) {
                return EventStatus::Paused(entity);
            }

            if event.is_active() {
                if let Some(operation) = operation {
                    if operation.is_ready() {
                        EventStatus::Ready(entity)
                    } else if operation.is_completed() {
                        EventStatus::Completed(entity)
                    } else if operation.is_empty() {
                        EventStatus::Scheduled(entity)
                    } else if operation.is_cancelled() {
                        EventStatus::Cancelled(entity)
                    } else {
                        EventStatus::InProgress(entity)
                    }
                } else {
                    EventStatus::New(entity)
                }
            } else {
                EventStatus::Inactive(entity)
            }
        } else {
            EventStatus::Disposed(entity)
        }
    }

    /// Scans event status and returns a vector of entites w/ their status,
    ///
    pub fn scan(&self) -> impl Iterator<Item = EventStatus> + '_ {
        let State {
            entities, events, ..
        } = self;

        (entities, events).join().map(|(e, _)| self.status(e))
    }

    /// Returns a vec of cursors,
    ///
    pub fn scan_cursors(&self) -> Vec<Cursor> {
        let State {
            entities,
            cursors,
            events,
            ..
        } = self;

        let mut _cursors = vec![];

        for (_, cursor, _) in (entities, cursors, events).join() {
            _cursors.push(cursor.clone());
        }
        _cursors
    }

    /// Handles a vec of events,
    ///
    pub fn handle(&mut self) {
        for event in self.scan().collect::<Vec<_>>() {
            self.handle_event(&event)
        }
    }

    /// Handles a single event,
    ///
    pub fn handle_event(&mut self, event: &EventStatus) {
        match event {
            EventStatus::Scheduled(e) | EventStatus::New(e) => {
                event!(Level::DEBUG, "Starting event {}", e.id());
                self.set_scheduled_connection_state(*e);
                self.transition(None, *e);
            }
            EventStatus::Ready(ready) => {
                let mut result = self.get_result(*ready);

                let mut result = result.take();

                if let Some(Yielding(yielding, _)) = self.yielding.remove(*ready) {
                    if let Some(_result) = result.take() {
                        let clone = _result.clone();
                        match yielding.send(_result) {
                            Ok(_) => {}
                            Err(_) => {
                                event!(Level::ERROR, "Could not send to yielding channel");
                            }
                        }
                        result = Some(clone);
                    } else {
                        drop(yielding);
                    }
                }

                let next_entities = self.get_next_entities(*ready);

                for next in next_entities {
                    event!(
                        Level::DEBUG,
                        "\n\n\tEvent transition\n\t{} -> {}\n",
                        ready.id(),
                        next.id()
                    );
                    if let Some(error) = result.as_ref().and_then(ThunkContext::get_errors) {
                        self.set_error_connection_state(*ready, next, error.clone());
                        self.send_error_context(error);
                    } else {
                        self.set_completed_connection_state(*ready, next);
                        self.send_completed_event(*ready);
                        if !self.activate(next) {
                            event!(Level::DEBUG, "Repeating event");
                            let State { limits, .. } = self;
                            if let Some(limit) = limits.get_mut(next) {
                                event!(Level::DEBUG, "Remaining repeats {}", limit.0);
                                if !limit.take_one() {
                                    event!(Level::DEBUG, "Limit reached for {}", next.id());

                                    for n in self.get_next_entities(next) {
                                        self.set_completed_connection_state(next, n);
                                    }
                                    return;
                                }
                            }
                        }

                        // Signal to the events this event is connected to that this
                        // event is being processed
                        self.set_scheduled_connection_state(next);
                        self.transition(result.as_ref(), next);
                    }
                }
            }
            EventStatus::InProgress(_) => {
                // TODO make a better place to send these --
                // event!(Level::TRACE, "{} is in progress", in_progress.id());
            }
            EventStatus::Completed(_) => {
                // event!(Level::TRACE, "{} is complete", completed.id());
            }
            EventStatus::Cancelled(_) => {
                // event!(Level::TRACE, "{} is cancelled", cancelled.id());
            }
            _ => {}
        }

        let State { tick_control, .. } = self;
        tick_control.update_tick_rate();
    }

    /// Performs a serialized tick, waiting for any events in-progress before returning,
    ///
    /// This is similar to a debugger "step",
    ///
    pub fn serialized_tick(&mut self) {
        let event_state = self.scan();
        for event in event_state {
            match event {
                EventStatus::InProgress(e) => {
                    // | EventStatus::Paused(e) => {
                    self.wait_for_ready(e);
                }
                _ => {}
            }
        }

        self.handle();
    }

    /// Scans event data and handles any ready transitions, does not block,
    ///
    pub fn tick(&mut self) {
        self.handle();
    }

    /// Returns next entities this event points to,
    ///
    pub fn get_next_entities(&self, event: Entity) -> Vec<Entity> {
        let State { cursors, .. } = self;
        if let Some(cursor) = cursors.get(event) {
            match cursor {
                Cursor::Next(next) => {
                    vec![*next]
                }
                Cursor::Fork(forks) => forks.iter().cloned().collect::<Vec<_>>(),
            }
        } else {
            vec![]
        }
    }

    /// Returns a result for an event if the operation is ready,
    ///
    pub fn get_result(&mut self, event: Entity) -> Option<ThunkContext> {
        let State { operations, .. } = self;

        if let Some(operation) = operations.get_mut(event) {
            operation.wait_if_ready()
        } else {
            None
        }
    }

    /// Returns a result for an event if the operation is ready,
    ///
    pub fn wait_on(&mut self, event: Entity) -> Option<ThunkContext> {
        let State { operations, .. } = self;

        if let Some(operation) = operations.get_mut(event) {
            operation.wait()
        } else {
            None
        }
    }

    /// Waits for an event's operation to be ready w/o completing it,
    ///
    pub fn wait_for_ready(&self, event: Entity) {
        loop {
            if self.is_ready(event) {
                break;
            }
        }
    }

    /// Returns true if the event operation is in an is_ready state,
    /// 
    pub fn is_ready(&self, event: Entity) -> bool {
        let State { operations, .. } = self;

        if let Some(operation) = operations.get(event) {
            operation.is_ready()
        } else {
            false
        }
    }

    /// Cancels an event's operation
    ///
    pub fn cancel(&mut self, event: Entity) -> bool {
        let State { operations, .. } = self;

        if let Some(operation) = operations.get_mut(event) {
            event!(Level::TRACE, "Cancelling {}", event.id());
            operation.cancel()
        } else {
            false
        }
    }

    /// Activates an event, returns true if this is the initial activation
    ///
    pub fn activate(&mut self, event: Entity) -> bool {
        let event_entity = event;
        let State {
            sequences, events, ..
        } = self;
        if let Some(event) = events.get(event) {
            let mut next = event.clone();
            if let Some(sequence) = next.activate() {
                self.lazy_update.insert(event_entity, next);
                if !sequences.contains(event_entity) {
                    self.lazy_update.insert(event_entity, sequence);
                }
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Updates state,
    ///
    pub fn update_state(&self, graph: &AttributeGraph) -> bool {
        let Self { entities, .. } = self;
        let entity = entities.entity(graph.entity_id());
        self.lazy_update.insert(entity, graph.clone());
        true
    }

    /// Handles the transition of an event,
    ///
    pub fn transition(&mut self, previous: Option<&ThunkContext>, event: Entity) {
        let State { transitions, .. } = self;

        let transition = transitions.get(event).unwrap_or(&Transition::Start);
        match transition {
            Transition::Start => {
                self.start(event, previous);
            }
            Transition::Once => {
                self.once(event, previous);
            }
            Transition::Select => {
                if let Some(previous) = previous {
                    event!(Level::TRACE, "Selecting {}", previous.state().entity_id());
                    self.select(event, previous);
                    self.start(event, Some(previous));
                } else {
                    tracing::event!(Level::DEBUG, "Skipping");
                }
            }
            Transition::Spawn => {
                let spawned = self.spawn(event);
                self.start(spawned, previous);
            }
            Transition::Buffer => {
                // TODO: Buffers incoming events
                todo!()
            }
        }
    }

    /// Starts an event immediately, cancels any ongoing operations
    ///
    pub fn start(&mut self, event: Entity, previous: Option<&ThunkContext>) {
        if let Some(e) = self.events.get(event) {
            event!(Level::DEBUG, "\n\n\t{}\tstarted event {}\n", e, event.id());

            let sequence = self.sequences.get(event).expect("should have a sequence");

            let previous = if let None = previous {
                if let Some(Yielding(_, previous)) = self.yielding.get(event) {
                    Some(previous)
                } else {
                    None
                }
            } else {
                previous
            };

            let operation = self.start_sequence(event, sequence, previous);

            if let Some(existing) = self.operations.get_mut(event) {
                existing.set_task(operation);
            } else {
                self.operations
                    .insert(event, operation)
                    .expect("should be able to insert operation");
            }

            self.set_started_connection_state(event);
        } else {
            event!(
                Level::DEBUG,
                "Did not have an event to start for {}",
                event.id()
            );
        }
    }

    /// Skips if the event already has a result,
    ///
    pub fn once(&mut self, event: Entity, previous: Option<&ThunkContext>) {
        if self.get_result(event).is_none() {
            self.start(event, previous);
        } else {
            for next in self.get_next_entities(event) {
                self.transition(previous, next);
            }
        }
    }

    /// Selects an incoming event and cancels any others,
    ///
    pub fn select(&mut self, event: Entity, previous: &ThunkContext) {
        let State {
            entities,
            connections,
            ..
        } = self;

        let selected = previous
            .state()
            .find_int("event_id")
            .expect("should have event id");
        let selected = entities.entity(selected as u32);

        let connection = connections
            .get(event)
            .expect("should have a connection")
            .clone();
        for (from, _) in connection
            .connections()
            .filter(|(from, _)| **from != selected)
        {
            self.cancel(*from);
        }
    }

    /// Spawn takes an event and creates a new entity based off of the original event,
    ///
    /// In addition, it must handle connecting the spawned entity to the original event's cursors,
    /// When updating the connection, the connection state will be distinct from the original, however all perf. metrics will be
    /// recorded in the original's histogram, as the spawned entity will be ephemeral.
    ///
    pub fn spawn(&mut self, source: Entity) -> Entity {
        let spawned = self.entities.create();

        // Add spawned event to connections
        for e in self.get_next_entities(source).iter() {
            if let Some(connection) = self.connections.get_mut(*e) {
                connection.add_spawned(source, spawned);
            }
        }

        // Enable sequence on spawned
        let sequence = self.sequences.get(source).expect("should have a sequence");
        self.lazy_update.insert(spawned, sequence.clone());

        // Enable event on spawned
        let event = self.events.get(source).expect("should have event");
        let mut spawned_event = event.clone();
        spawned_event.activate();
        self.lazy_update.insert(spawned, spawned_event);

        // Enable cursor on spawned,
        if let Some(cursor) = self.cursors.get(source) {
            self.lazy_update.insert(spawned, cursor.clone());
        }

        // Remove yielding and add it to this spawned event,
        if let Some(yielding) = self.yielding.remove(source) {
            self.lazy_update.insert(spawned, yielding);
        }

        spawned
    }

    /// Deletes an entity,
    ///
    pub fn delete(&mut self, entity: Entity) -> bool {
        match self.entities.delete(entity) {
            Ok(_) => {
                event!(Level::DEBUG, "Deleted spawned entity, {}", entity.id());
                true
            }
            Err(err) => {
                event!(
                    Level::ERROR,
                    "Could not delete entity {}, {err}",
                    entity.id()
                );
                false
            }
        }
    }

    /// Cleans up an event connection,
    ///
    pub fn cleanup_connection(&mut self, event: Entity) -> bool {
        let mut disposed = vec![];
        if let Some(connection) = self.connections.get(event) {
            for (s, _, _) in connection.iter_spawned() {
                match self.status(*s) {
                    EventStatus::Disposed(_) => {
                        disposed.push(*s);
                    }
                    _ => {}
                }
            }
        }

        if let Some(connection) = self.connections.get_mut(event) {
            for d in disposed.iter() {
                connection.remove_spawned(d);
            }
        }

        disposed.len() > 0
    }

    /// Returns an iterator over spawned events,
    ///
    /// The tuple layout is, (spawned entity, source entity, owner)
    ///
    pub fn iter_spawned_events(&self) -> impl Iterator<Item = (&Entity, &Entity, &Entity)> {
        let Self {
            entities,
            connections,
            ..
        } = self;

        (entities, connections)
            .join()
            .map(|(_, c)| c.iter_spawned())
            .flatten()
    }

    /// Swaps the positions of two nodes in a sequence,
    ///
    pub fn swap(&mut self, owner: Entity, from: Entity, to: Entity) -> bool {
        let Self { sequences, .. } = self;

        if let Some(seq) = sequences.get(owner) {
            let mut next = seq.clone();
            if next.swap(from, to) {
                self.lazy_update.insert(owner, next);

                return true;
            }
        }

        false
    }
}

impl<'a> State<'a> {
    /// Returns true if there will no more activity,
    ///
    pub fn should_exit(&self) -> bool {
        self.scan().all(|e| match e {
            EventStatus::Inactive(_) | EventStatus::Completed(_) | EventStatus::Cancelled(_) => {
                true
            }
            _ => false,
        })
    }

    /// Returns true if the runtime can continue,
    ///
    pub fn can_continue(&self) -> bool {
        let State { tick_control, .. } = self;

        tick_control.can_tick()
    }

    /// Pauses the event runtime,
    ///
    pub fn pause(&mut self) {
        let State { tick_control, .. } = self;

        tick_control.pause()
    }

    /// Resumes the event runtime,
    ///
    pub fn resume(&mut self) {
        let State { tick_control, .. } = self;

        tick_control.resume()
    }

    /// Returns the tick frequency,
    ///
    pub fn tick_rate(&self) -> u64 {
        let State { tick_control, .. } = self;

        tick_control.tick_rate()
    }

    /// Handles any rate limits,
    ///
    pub fn handle_rate_limits(&mut self) {
        let State { tick_control, .. } = self;

        if tick_control.rate_limit().is_some() {
            tick_control.update_rate_limit();
        }
    }

    /// Set a rate limit on the tick control,
    ///
    pub fn set_rate_limit(&mut self, limit: u64) {
        let State { tick_control, .. } = self;

        tick_control.set_rate_limit(limit);
    }

    /// Clears any rate limits on the tick control,
    ///
    pub fn clear_rate_limit(&mut self) {
        let State { tick_control, .. } = self;

        tick_control.remove_rate_limit();
    }

    /// Pauses a specific event,
    ///
    /// This can also be used as a "breakpoint",
    ///
    pub fn pause_event(&mut self, event: Entity) -> bool {
        let State { tick_control, .. } = self;

        tick_control.pause_entity(event)
    }

    /// Resumes a specific event,
    ///
    pub fn resume_event(&mut self, event: Entity) -> bool {
        let State { tick_control, .. } = self;

        tick_control.resume_entity(event)
    }
}

/// Functions for sending messages,
///
impl<'a> State<'a> {
    /// Tries to send an error context,
    ///
    pub fn send_error_context(&self, error_context: ErrorContext) {
        let State { send_errors, .. } = self;

        send_errors.try_send(error_context).ok();
    }

    /// Returns the event entity that just completed,
    ///
    pub fn send_completed_event(&self, event: Entity) {
        let State { send_completed, .. } = self;

        send_completed.send(event).ok();
    }
}

/// Functions for handling connection state
///
impl<'a> State<'a> {
    /// Sets the scheduled connection state for the connections this event is connected to,
    ///
    pub fn set_scheduled_connection_state(&mut self, event: Entity) {
        let State {
            cursors,
            connections,
            ..
        } = self;

        if let Some(cursor) = &cursors.get(event) {
            match cursor {
                Cursor::Next(next) => {
                    if let Some(connection) = connections.get_mut(*next) {
                        if let Some(key) = connection.schedule(event) {
                            self.lazy_update
                                .insert(event, key);
                        }
                    }
                }
                Cursor::Fork(forks) => {
                    for fork in forks {
                        if let Some(connection) = connections.get_mut(*fork) {
                            if let Some(key) = connection.schedule(event) {
                                self.lazy_update
                                    .insert(event, key);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Sets the connection state to started for this event, on the connections it is connected to,
    ///
    pub fn set_started_connection_state(&mut self, event: Entity) {
        let State {
            cursors,
            connections,
            ..
        } = self;

        if let Some(cursor) = &cursors.get(event) {
            match cursor {
                Cursor::Next(next) => {
                    if let Some(connection) = connections.get_mut(*next) {
                        connection.start(event);
                    }
                }
                Cursor::Fork(forks) => {
                    for fork in forks {
                        if let Some(connection) = connections.get_mut(*fork) {
                            connection.start(event);
                        }
                    }
                }
            }
        }
    }

    /// Sets the connection state for an event on the connection it is connected to,
    ///
    pub fn set_error_connection_state(
        &mut self,
        incoming: Entity,
        event: Entity,
        error: ErrorContext,
    ) {
        let State { connections, .. } = self;
        if let Some(connection) = connections.get_mut(event) {
            connection.complete(incoming, Some(&error));
        }
    }

    /// Sets the connection state to completed for the incoming event, on the connected event
    ///
    pub fn set_completed_connection_state(&mut self, incoming: Entity, event: Entity) {
        let State { connections, .. } = self;

        if let Some(connection) = connections.get_mut(event) {
            connection.complete(incoming, None);
        }
    }
}

/// Editor related functions
///
impl<'a> State<'a> {
    /// Handles the node command,
    ///
    pub fn handle_node_command(&mut self, command: NodeCommand) -> bool {
        match command {
            NodeCommand::Activate(event) => {
                if self.activate(event) {
                    event!(Level::DEBUG, "Activating event {}", event.id());
                    true
                } else {
                    false
                }
            }
            NodeCommand::Reset(event) => {
                if self.reset(event) {
                    event!(Level::DEBUG, "Reseting event {}", event.id());
                    true
                } else {
                    false
                }
            }
            NodeCommand::Pause(event) => {
                if self.pause_event(event) {
                    event!(Level::DEBUG, "Pausing event {}", event.id());
                    true
                } else {
                    false
                }
            }
            NodeCommand::Resume(event) => {
                if self.resume_event(event) {
                    event!(Level::DEBUG, "Resuming event {}", event.id());
                    true
                } else {
                    false
                }
            }
            NodeCommand::Cancel(event) => {
                if self.cancel(event) {
                    event!(Level::DEBUG, "Cancelling event {}", event.id());
                    true
                } else {
                    false
                }
            }
            NodeCommand::Spawn(event) => {
                self.spawn(event);
                true
            }
            NodeCommand::Swap { owner, from, to } => {
                if self.swap(owner, from, to) {
                    event!(
                        Level::DEBUG,
                        "Swapping nodes {} -> {} in {}'s sequence",
                        from.id(),
                        to.id(),
                        owner.id()
                    );
                    true
                } else {
                    false
                }
            }
            NodeCommand::Update(graph) => {
                if self.update_state(&graph) {
                    event!(Level::DEBUG, "Updating state for {}", graph.entity_id());
                    true
                } else {
                    false
                }
            }
            NodeCommand::Custom(name, entity) => {
                event!(
                    Level::DEBUG,
                    "Handling custom command {name} for {}",
                    entity.id()
                );

                if let Some(handler) = self.handlers.get(&name) {
                    if handler(self, entity) {
                        event!(
                            Level::DEBUG,
                            "Executed custom handler, {name}({})",
                            entity.id()
                        );
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        }
    }
}

cfg_editor! {
    impl<'a> State<'a> {
        /// Returns connections from adhoc profilers as a vector of nodes,
    ///
    pub fn adhoc_nodes(&self) -> Vec<Node> {
        let State {
            entities,
            appendix,
            profilers,
            connections,
            ..
        } = self;

        (entities, profilers, connections)
            .join()
            .map(|(e, _, c)| Node {
                status: NodeStatus::Profiler(e),
                connection: Some(c.clone()),
                appendix: appendix.deref().clone(),
                ..Default::default()
            })
            .collect::<Vec<_>>()
    }

    /// Returns current event nodes,
    ///
    pub fn event_nodes(&'a self) -> Vec<Node> {
        self.scan()
            .filter_map(|e| self.event_node(e.entity()))
            .collect::<Vec<_>>()
    }

    /// Returns a node,
    ///
    pub fn event_node(&self, event: Entity) -> Option<Node> {
        let State {
            appendix,
            entities,
            cursors,
            events,
            connections,
            transitions,
            sequences,
            connection_states,
            adhocs,
            ..
        } = self;

        if let Some((_, connection, cursor, transition, sequence, connection_state, adhoc)) = (
            events,
            connections.maybe(),
            cursors.maybe(),
            transitions.maybe(),
            sequences.maybe(),
            connection_states.maybe(),
            adhocs.maybe(),
        )
            .join()
            .get(event, entities)
        {
            Some(Node {
                status: NodeStatus::Event(self.status(event)),
                transition: transition.cloned(),
                connection: connection.cloned(),
                cursor: cursor.cloned(),
                sequence: sequence.cloned(),
                connection_state: connection_state.cloned(),
                appendix: appendix.deref().clone(),
                adhoc: adhoc.cloned(),
                ..Default::default()
            })
        } else {
            None
        }
    }

    /// Scans engines for status,
    ///
    pub fn scan_engines(&'a self) -> impl Iterator<Item = EngineStatus> + '_ {
        let Self {
            entities,
            sequences,
            engines,
            ..
        } = self;

        (entities, sequences, engines)
            .join()
            .map(|(e, _, _)| self.engine_status(e))
    }

    /// Returns the status for an engine,
    ///
    pub fn engine_status(&self, engine: Entity) -> EngineStatus {
        let Self {
            entities,
            sequences,
            engines,
            ..
        } = self;

        match (sequences, engines).join().get(engine, entities) {
            Some((sequence, _)) => {
                let mut _events = sequence.iter_entities().map(|e| self.status(e));

                if _events.all(|e| match e {
                    super::EventStatus::Inactive(_) => true,
                    _ => false,
                }) {
                    EngineStatus::Inactive(engine)
                } else {
                    EngineStatus::Active(engine)
                }
            }
            None => EngineStatus::Disposed(engine),
        }
    }

    /// Returns a vector of engine nodes,
    ///
    pub fn engine_nodes(&self) -> Vec<Node> {
        self.scan_engines()
            .filter_map(|e| self.engine_node(e.entity()))
            .collect::<Vec<_>>()
    }

    /// Returns the current engine node for an entity,
    ///
    pub fn engine_node(&self, engine: Entity) -> Option<Node> {
        let State {
            appendix,
            entities,
            cursors,
            engines,
            sequences,
            ..
        } = self;

        if let Some((_, cursor, sequence)) = (engines, cursors.maybe(), sequences.maybe())
            .join()
            .get(engine, entities)
        {
            Some(Node {
                status: NodeStatus::Engine(self.engine_status(engine)),
                cursor: cursor.cloned(),
                sequence: sequence.cloned(),
                appendix: appendix.deref().clone(),
                ..Default::default()
            })
        } else {
            None
        }
    }
}
}

impl<'a> State<'a> {
    /// Returns the entity for a block by name,
    ///
    pub fn find_entity(&self, expression: impl AsRef<str>) -> Option<&Entity> {
        self.entity_map.get(expression.as_ref())
    }
}

impl<'a> State<'a> {
    /// Add's a plugin to an event,
    ///
    pub fn add_plugin<P>(&mut self, event_entity: Entity) -> bool
    where
        P: Plugin + BlockObject + Default,
    {
        if let Some(event) = self.events.get(event_entity) {
            let mut next = event.clone();

            if let Some(thunk_src) = self.runtime.thunk_source(format!("call::{}", P::symbol())) {
                let plugin_entity = self.entities.create();

                if next.add_thunk(thunk_src.thunk(), plugin_entity) {
                    if let Some(sequence) = next.sequence() {
                        self.lazy_update.insert(event_entity, sequence.clone());

                        if let Some(block) = self.blocks.get(event_entity) {
                            let mut graph = if let Some(default_settings) =
                                self.blocks.get(self.entities.entity(0)).and_then(|b| {
                                    b.index()
                                        .iter()
                                        .find(|i| i.root().name() == thunk_src.thunk().0)
                                        .cloned()
                                }) {
                                let mut root = default_settings.root().clone();
                                root.set_id(plugin_entity.id());

                                let mut block_index = BlockIndex::new(&root);
                                for (name, prop) in default_settings.properties().iter_properties()
                                {
                                    block_index.properties_mut().set(name, prop.clone());
                                }
                                AttributeGraph::new(block_index)
                            } else {
                                let root =
                                    Attribute::new(plugin_entity.id(), P::symbol(), Value::Empty);
                                let mut graph = AttributeGraph::new(BlockIndex::new(&root));
                                graph.with_symbol(P::symbol(), "");
                                graph
                            };

                            graph.with_int("event_id", event_entity.id() as i32);

                            let block_query = P::default().query();

                            for (name, prop) in block_query.iter_properties() {
                                match prop {
                                    BlockProperty::Single(value) => {
                                        graph.with(name, value.clone());
                                    }
                                    BlockProperty::List(values) => {
                                        for value in values {
                                            graph.with(name, value.clone());
                                        }
                                    }
                                    BlockProperty::Required(Some(value)) => {
                                        graph.with(name, value.clone());
                                    }
                                    BlockProperty::Optional(Some(value)) => {
                                        graph.with(name, value.clone());
                                    }
                                    _ => {}
                                }
                            }

                            let mut appendix = (*self.appendix).deref().clone();
                            appendix.insert_general(plugin_entity, &thunk_src.thunk());
                            appendix.insert_state(
                                plugin_entity,
                                crate::appendix::State {
                                    control_symbol: block.symbol().to_string(),
                                    graph: Some(graph.clone()),
                                },
                            );

                            *self.appendix = Arc::new(appendix);

                            self.lazy_update.insert(plugin_entity, block.clone());

                            self.lazy_update.insert(plugin_entity, graph);

                            self.lazy_update.insert(plugin_entity, thunk_src.thunk());

                            self.lazy_update.insert(event_entity, next);


                            return true;
                        } else {
                            event!(Level::DEBUG, "Didn't have a block");
                        }
                    }
                }
            }
        }

        false
    }

    /// Returns a new context,
    ///
    pub fn new_context(&self) -> ThunkContext {
        let Self { entities, .. } = self;

        let entity = entities.create();

        self.initialize_context(entity, None)
    }

    /// Returns an initialized context,
    ///
    pub fn initialize_context(
        &self,
        entity: Entity,
        initial_context: Option<&ThunkContext>,
    ) -> ThunkContext {
        let Self { blocks, graphs, .. } = self;

        let mut context = self
            .plugins
            .features()
            .enable(entity, initial_context.unwrap_or(&ThunkContext::default()));

        if let Some(block) = blocks.get(entity) {
            context = context.with_block(block);
        }

        if let Some(graph) = graphs.get(entity) {
            context = context.with_state(graph.clone());
        }

        context
    }

    /// Combines a sequence of plugin calls into an operation,
    ///
    pub fn start_sequence(
        &self,
        event: Entity,
        sequence: &Sequence,
        initial_context: Option<&ThunkContext>,
    ) -> Operation {
        let Self {
            plugins,
            thunks,
            blocks,
            graphs,
            ..
        } = self;

        let sequence = sequence.clone();
        let handle = plugins.features().handle();
        let entity = sequence.peek().expect("should have at least 1 entity");

        let thunk_context = self.initialize_context(entity, initial_context);

        let mut thunk_map = HashMap::<Entity, Thunk>::default();
        let mut graph_map = HashMap::<Entity, AttributeGraph>::default();
        let mut block_map = HashMap::<Entity, Block>::default();
        for call in sequence.iter_entities() {
            let thunk = thunks.get(call).expect("should have a thunk").clone();
            let graph = graphs.get(call).expect("should have a graph").clone();
            let block = blocks.get(call).expect("should have a block").clone();
            thunk_map.insert(call, thunk);
            graph_map.insert(call, graph);
            block_map.insert(call, block);
        }

        let (tx, rx) = tokio::sync::oneshot::channel::<()>();

        let task = handle.spawn(async move {
            let mut context = thunk_context.clone();
            let handle = context.handle().expect("should be a handle");
            let mut cancel_source = Some(rx);

            for e in sequence.iter_entities() {
                if let Some(mut _rx) = cancel_source.take() {
                    let (_tx, rx) = oneshot::channel::<()>();

                    context.set_entity(e);

                    let thunk = thunk_map.get(&e).expect("should have a thunk");
                    let graph = graph_map.get(&e).expect("should exist");
                    let block = block_map.get(&e).expect("should exist");

                    context.set_state(graph.clone());
                    context.set_block(block);

                    let mut operation =
                        Operation::empty(handle.clone()).start_with(thunk, &mut context);
                    {
                        let _rx = &mut _rx;
                        select! {
                            result = operation.task(rx) => {
                                match result {
                                    Some(mut result) => {
                                        let mut completion = Completion {
                                            timestamp: Utc::now(),
                                            event,
                                            thunk: e,
                                            control_values: context.control_values().clone(),
                                            query: context.properties().clone(),
                                            returns: None,
                                        };

                                        context = result.consume();

                                        completion.returns = Some(context.properties().clone());

                                        context.dispatch_completion(completion);
                                    }
                                    None => {
                                    }
                                }
                            },
                            _ = _rx => {
                                _tx.send(()).ok();
                                break;
                            }
                        }
                    }

                    event!(
                        Level::DEBUG,
                        "\n\n\t{:?}\n\tcompleted\n\tplugin {}\n",
                        thunk,
                        e.id(),
                    );

                    cancel_source = Some(_rx);
                } else {
                    break;
                }
            }

            context
        });

        Operation::empty(handle.clone()).with_task((task, tx))
    }
}
