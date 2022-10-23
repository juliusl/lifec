use specs::{prelude::*, Entities, SystemData};

use crate::{
    prelude::ErrorContext, AttributeIndex, Connection, Cursor, Event, Operation, Sequence,
    ThunkContext,
};

use super::{Plugins, Transition};

use tracing::{event, Level};

/// Event system data,
///
#[derive(SystemData)]
pub struct Events<'a>(
    Plugins<'a>,
    Entities<'a>,
    ReadStorage<'a, Sequence>,
    ReadStorage<'a, Cursor>,
    ReadStorage<'a, Transition>,
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
}

impl<'a> Events<'a> {
    /// Scans event status and returns a vector of entites w/ their status,
    ///
    pub fn scan(&self) -> Vec<EventStatus> {
        let Events(_, entities, .., events, _, operations) = self;

        let mut status = vec![];

        for (entity, _, operation) in (entities, events, operations.maybe())
            .join()
            .filter(|(_, e, _)| e.is_active())
        {
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
                    self.transition(None, *e);
                }
                EventStatus::Ready(ready) => {
                    let result = self.get_result(*ready);

                    let next_entities = self.get_next_entities(*ready);

                    for next in next_entities {
                        event!(Level::DEBUG, "{} -> {}", ready.id(), next.id());
                        if let Some(error) = result.as_ref().and_then(ThunkContext::get_errors) {
                            self.set_error_connection_state(*ready, next, error);
                        } else {
                            self.activate(next);
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
            }
        }
    }

    /// Returns next entities this event points to,
    ///
    pub fn get_next_entities(&mut self, event: Entity) -> Vec<Entity> {
        let Events(_, _, _, cursors, ..) = self;
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

    /// Activates an event,
    ///
    pub fn activate(&mut self, event: Entity) {
        let Events(.., events, _, _) = self;
        if let Some(event) = events.get_mut(event) {
            event.activate();
        } else {
            event!(Level::DEBUG, "Skipped activating {}", event.id());
        }
    }

    /// Handles the transition of an event,
    ///
    pub fn transition(&mut self, previous: Option<&ThunkContext>, event: Entity) {
        {
            // Signal to the events this event is connected to that this
            // event is being processed
            self.set_scheduled_connection_state(event);
        }

        let Events(.., transitions, _, _, _) = self;

        let transition = transitions.get(event).unwrap_or(&Transition::Start);
        match transition {
            Transition::Start => {
                self.start(event, previous);
            }
            Transition::Once => {
                if self.get_result(event).is_none() {
                    self.start(event, previous);
                } else {
                    // TODO - Hamdle an existing result
                    todo!()
                }
            }
            Transition::Spawn => {
                // TODO: Duplicates the current event data under a new entity
                todo!()
            }
            Transition::Select => {
                if let Some(previous) = previous {
                    event!(Level::TRACE, "Selecting {}", previous.state().entity_id());
                    self.select(event, previous);
                } else {
                    self.start(event, None);
                }
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
        let Events(plugins, _, sequences, .., operations) = self;

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

    /// Selects an incoming event and cancels any others,
    ///
    pub fn select(&mut self, event: Entity, previous: &ThunkContext) {
        let Events(_, entities, .., connections, _) = self;

        let selected = previous.state().find_int("event_id").expect("should have event id");
        let selected = entities.entity(selected as u32);

        let connection = connections
            .get_mut(event)
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

/// Functions for handling connection state
///
impl<'a> Events<'a> {
    /// Sets the scheduled connection state for the connections this event is connected to,
    ///
    pub fn set_scheduled_connection_state(&mut self, event: Entity) {
        let Events(.., cursors, _, _, connections, _) = self;

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
        let Events(.., cursors, _, _, connections, _) = self;

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
