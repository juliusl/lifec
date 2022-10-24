use specs::{prelude::*, Entities, SystemData};

use super::{Limit, Plugins, Transition};
use crate::prelude::*;

use tracing::{event, Level};

/// Event system data,
///
#[derive(SystemData)]
pub struct Events<'a>(
    Read<'a, tokio::sync::mpsc::Sender<ErrorContext>, EventRuntime>,
    Read<'a, tokio::sync::broadcast::Sender<Entity>, EventRuntime>,
    Plugins<'a>,
    Entities<'a>,
    ReadStorage<'a, Cursor>,
    ReadStorage<'a, Transition>,
    WriteStorage<'a, Sequence>,
    WriteStorage<'a, Limit>,
    WriteStorage<'a, Event>,
    WriteStorage<'a, Connection>,
    WriteStorage<'a, Operation>,
);

/// Enumeration of event statuses,
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventStatus {
    /// Means that the operation is empty has no activity
    ///
    Scheduled(Entity),
    /// Means that a new operation is required
    ///
    New(Entity),
    /// Means that the operation is in progress
    ///
    InProgress(Entity),
    /// Means that the operation is ready to transition
    ///
    Ready(Entity),
    /// Means that the operation has already completed
    ///
    Completed(Entity),
    /// Means that the operation has already completed
    ///
    Cancelled(Entity),
    /// Means that the entity has not been activated yet
    ///
    Inactive(Entity),
}

impl<'a> Events<'a> {
    /// Scans event status and returns a vector of entites w/ their status,
    ///
    pub fn scan(&self) -> Vec<EventStatus> {
        let Events(_, _, _, entities, .., events, _, operations) = self;

        let mut status = vec![];

        for (entity, event, operation) in (entities, events, operations.maybe()).join() {
            if event.is_active() {
                if let Some(operation) = operation {
                    if operation.is_ready() {
                        status.push(EventStatus::Ready(entity));
                    } else if operation.is_completed() {
                        status.push(EventStatus::Completed(entity));
                    } else if operation.is_empty() {
                        status.push(EventStatus::Scheduled(entity));
                    } else if operation.is_cancelled() {
                        status.push(EventStatus::Cancelled(entity));
                    } else {
                        status.push(EventStatus::InProgress(entity));
                    }
                } else {
                    status.push(EventStatus::New(entity));
                }
            } else {
                status.push(EventStatus::Inactive(entity));
            }
        }

        status
    }

    /// Handles events,
    ///
    pub fn handle(&mut self, events: Vec<EventStatus>) {
        for event in events.iter() {
            match event {
                EventStatus::Scheduled(e) | EventStatus::New(e) => {
                    event!(Level::DEBUG, "Starting event {}", e.id());
                    self.set_scheduled_connection_state(*e);
                    self.transition(None, *e);
                }
                EventStatus::Ready(ready) => {
                    let result = self.get_result(*ready);

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
                                let Events(.., limits, _, _, _) = self;
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
                EventStatus::InProgress(in_progress) => {
                    event!(Level::TRACE, "{} is in progress", in_progress.id());
                }
                EventStatus::Completed(completed) => {
                    event!(Level::TRACE, "{} is complete", completed.id());
                }
                EventStatus::Cancelled(cancelled) => {
                    event!(Level::TRACE, "{} is cancelled", cancelled.id());
                }
                _ => {}
            }
        }
    }

    /// Performs a serialized tick, waiting for any events in-progress before returning,
    ///
    pub fn serialized_tick(&mut self) {
        let event_state = self.scan();
        for event in event_state {
            if let EventStatus::InProgress(in_progress) = event {
                self.wait_for_ready(in_progress);
            }
        }

        let event_state = self.scan();
        self.handle(event_state);
    }

    /// Scans event data and handles any ready transitions, does not block,
    ///
    pub fn tick(&mut self) {
        let event_state = self.scan();
        self.handle(event_state);
    }

    /// Returns next entities this event points to,
    ///
    pub fn get_next_entities(&mut self, event: Entity) -> Vec<Entity> {
        let Events(_, _, _, _, cursors, ..) = self;
        if let Some(cursor) = cursors.get(event) {
            match cursor {
                Cursor::Next(next) => {
                    vec![*next]
                }
                Cursor::Fork(forks) => forks.to_vec(),
            }
        } else {
            vec![]
        }
    }

    /// Returns a result for an event if the operation is ready,
    ///
    pub fn get_result(&mut self, event: Entity) -> Option<ThunkContext> {
        let Events(.., operations) = self;

        if let Some(operation) = operations.get_mut(event) {
            operation.wait_if_ready()
        } else {
            None
        }
    }

    /// Returns a result for an event if the operation is ready,
    ///
    pub fn wait_on(&mut self, event: Entity) -> Option<ThunkContext> {
        let Events(.., operations) = self;

        if let Some(operation) = operations.get_mut(event) {
            operation.wait()
        } else {
            None
        }
    }

    /// Waits for an event's operation to be ready w/o completing it,
    ///
    pub fn wait_for_ready(&mut self, event: Entity) {
        let Events(.., operations) = self;

        loop {
            if let Some(operation) = operations.get_mut(event) {
                if operation.is_ready() {
                    break;
                }
            }
        }
    }

    /// Cancels an event's operation
    ///
    pub fn cancel(&mut self, event: Entity) {
        let Events(.., operations) = self;

        if let Some(operation) = operations.get_mut(event) {
            event!(Level::TRACE, "Cancelling {}", event.id());
            operation.cancel();
        }
    }

    /// Activates an event, returns true if this is the initial activation
    ///
    pub fn activate(&mut self, event: Entity) -> bool {
        let event_entity = event;
        let Events(.., sequences, _, events, _, _) = self;
        if let Some(event) = events.get_mut(event) {
            if let Some(sequence) = event.activate() {
                if !sequences.contains(event_entity) {
                    sequences
                        .insert(event_entity, sequence)
                        .expect("should be able to insert sequence");
                }
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Handles the transition of an event,
    ///
    pub fn transition(&mut self, previous: Option<&ThunkContext>, event: Entity) {
        let Events(.., transitions, _, _, _, _, _) = self;

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
                // TODO: Duplicates the current event data under a new entity
                todo!()
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
        let Events(_, _, plugins, _, _, _, sequences, .., events, _, operations) = self;

        let e = events.get(event).expect("should have an event");
        event!(Level::DEBUG, "\n\n\t{}\tstarted event {}\n", e, event.id());

        let sequence = sequences.get(event).expect("should have a sequence");

        let operation = plugins.start_sequence(sequence, previous);

        if let Some(existing) = operations.get_mut(event) {
            existing.set_task(operation);
        } else {
            operations
                .insert(event, operation)
                .expect("should be able to insert operation");
        }

        self.set_started_connection_state(event);
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
        let Events(_, _, _, entities, .., connections, _) = self;

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
}

impl<'a> Events<'a> {
    /// Returns true if there will no more activity,
    ///
    pub fn should_exit(&self) -> bool {
        let event_data = self.scan();

        event_data.iter().all(|e| match e {
            EventStatus::Completed(_) | EventStatus::Cancelled(_) => true,
            _ => false,
        })
    }
}

/// Functions for sending messages,
///
impl<'a> Events<'a> {
    /// Tries to send an error context,
    ///
    pub fn send_error_context(&self, error_context: ErrorContext) {
        let Events(errors, ..) = self;

        errors.try_send(error_context).ok();
    }

    /// Returns the event entity that just completed,
    ///
    pub fn send_completed_event(&self, event: Entity) {
        let Events(_, completed, ..) = self;

        completed.send(event).ok();
    }
}

/// Functions for handling connection state
///
impl<'a> Events<'a> {
    /// Sets the scheduled connection state for the connections this event is connected to,
    ///
    pub fn set_scheduled_connection_state(&mut self, event: Entity) {
        let Events(.., cursors, _, _, _, _, connections, _) = self;

        if let Some(cursor) = &cursors.get(event) {
            match cursor {
                Cursor::Next(next) => {
                    if let Some(connection) = connections.get_mut(*next) {
                        connection.schedule(event);
                    }
                }
                Cursor::Fork(forks) => {
                    for fork in forks {
                        if let Some(connection) = connections.get_mut(*fork) {
                            connection.schedule(event);
                        }
                    }
                }
            }
        }
    }

    /// Sets the connection state to started for this event, on the connections it is connected to,
    ///
    pub fn set_started_connection_state(&mut self, event: Entity) {
        let Events(.., cursors, _, _, _, _, connections, _) = self;

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
        let Events(.., connections, _) = self;
        if let Some(connection) = connections.get_mut(event) {
            connection.complete(incoming, Some(&error));
        }
    }

    /// Sets the connection state to completed for the incoming event, on the connected event
    ///
    pub fn set_completed_connection_state(&mut self, incoming: Entity, event: Entity) {
        let Events(.., _, _, connections, _) = self;

        if let Some(connection) = connections.get_mut(event) {
            connection.complete(incoming, None);
        }
    }
}
