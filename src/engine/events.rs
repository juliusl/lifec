use specs::{prelude::*, Entities, SystemData};

use crate::{Connection, Cursor, Event, Operation, Sequence, ThunkContext};

use super::{Plugins, Transition};

use tracing::{event, Level};

/// Event system data,
///
#[derive(SystemData)]
pub struct Events<'a>(
    pub Plugins<'a>,
    pub Entities<'a>,
    pub ReadStorage<'a, Sequence>,
    pub ReadStorage<'a, Cursor>,
    pub ReadStorage<'a, Transition>,
    pub WriteStorage<'a, Event>,
    pub WriteStorage<'a, Connection>,
    pub WriteStorage<'a, Operation>,
);

/// Enumeration of event statuses,
/// 
#[derive(Debug, Clone)]
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
                } else {
                    status.push(EventStatus::InProgress(entity));
                }
            } else {
                status.push(EventStatus::New(entity));
            }
        }

        status
    }

    /// Handles event statuses,
    /// 
    pub fn handle(&mut self, events: Vec<EventStatus>) {
        for event in events.iter() {
            match event {
                EventStatus::Scheduled(e) |
                EventStatus::New(e) => {
                    event!(Level::DEBUG, "Starting event {}", e.id());
                    self.transition(None, *e);
                },
                EventStatus::InProgress(in_progress) => {
                    event!(Level::DEBUG, "{} is in progress", in_progress.id());
                },
                EventStatus::Ready(ready) => {
                    let result = {
                        let Events(.., operations) = self;
        
                        if let Some(operation) = operations.get_mut(*ready) {
                            operation.wait_if_ready()
                        } else {
                            None
                        }
                    };

                    // TODO: Handle errors
                    let next_entities = {
                        let Events(_, _, _, cursors, ..) = self;
                        if let Some(cursor) = cursors.get(*ready) {
                            match cursor {
                                Cursor::Next(next) => {
                                    vec![*next]
                                },
                                Cursor::Fork(forks) => {
                                    forks.to_vec()
                                },
                            }
                        } else {
                            vec![]
                        }
                    };

                    for next in next_entities {
                        {
                            let Events(.., events, _, _) = self;
                            if let Some(event) = events.get_mut(next) {
                                event.activate();
                            }
                        }
                        self.transition(result.as_ref(), next);
                    }
                },
                EventStatus::Completed(completed) => {
                    event!(Level::DEBUG, "{} is completed", completed.id());
                },
            }
        }
    }

    /// Handles the transition of an event,
    ///
    pub fn transition(&mut self, previous: Option<&ThunkContext>, event: Entity) {
        let Events(.., cursors, transitions, _, connections, _) = self;

        // if let (Some(incoming), Some(connection)) = (incoming, connections.get_mut(event)) {
        //     connection.complete(incoming, None);
        // }

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

        let transition = transitions.get(event).unwrap_or(&Transition::Start);
        match transition {
            Transition::Start => {
                self.start(event, previous);
            }
            Transition::Once => {
                // TODO: If a result already exists
                todo!()
            }
            Transition::Spawn => {
                // TODO: Duplicates the current event data under a new entity
                todo!()
            }
            Transition::Select => {
                // TODO: Cancels any connected events in-progress except for the
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
        let Events(plugins, _, sequences, cursors, _, _, connections, operations) = self;

        let sequence = sequences.get(event).expect("should have a sequence");

        let operation = plugins.start_sequence(sequence, previous);

        if let Some(existing) = operations.get_mut(event) {
            existing.set_task(operation);
        } else {
            operations
                .insert(event, operation)
                .expect("should be able to insert operation");
        }

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

    // TODO: Add a way to restore an event's sequence
}
