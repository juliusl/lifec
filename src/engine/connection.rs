use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use hdrhistogram::Histogram;
use specs::{Component, Entity, VecStorage};
use tracing::{event, Level};

use crate::prelude::ErrorContext;

use super::Activity;

/// Connection state for a connected entity,
///
/// Struct with the incoming entity. At runtime when the connection is being evaluated,
/// the incoming entity will have an operation with the result.
///
/// The connection state will always have a way to look up the original components.
///
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct ConnectionState {
    /// Incoming entity,
    ///
    /// Since events can be duplicated, this could be different than the actual source,
    ///
    incoming: Entity,
    /// Source of the incoming connection, if None, then incoming is the source,
    ///
    source: Option<Entity>,
}

impl ConnectionState {
    /// Returns a connection state of the original entity,
    ///
    pub fn original(incoming: Entity) -> Self {
        Self {
            incoming,
            source: None,
        }
    }

    /// Returns the connection state for a duplicated entity,
    ///
    pub fn duplicate(incoming: Entity, source: Entity) -> Self {
        Self {
            incoming,
            source: Some(source),
        }
    }

    /// Returns the incoming entity,
    ///
    pub fn incoming(&self) -> Entity {
        self.incoming
    }

    /// Returns the source of this connection state,
    ///
    pub fn source(&self) -> Entity {
        self.source.unwrap_or(self.incoming)
    }
}

/// This component configures the Sequence cursor to point at the sequence it is connected to
///
#[derive(Component, Debug, Clone, PartialEq)]
#[storage(VecStorage)]
pub struct Connection {
    /// Set of entities of incoming connections,
    from: HashSet<Entity>,
    /// Owner of this connection,
    to: Entity,
    /// Map of the connection state,
    connection_state: HashMap<ConnectionState, Activity>,
    /// Histogram of performance per connection,
    performance: HashMap<Entity, Histogram<u32>>,
}

impl Hash for Connection {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for f in self.from.iter() {
            f.hash(state);
        }
        self.to.hash(state);
        for (c, a) in self.connection_state.iter() {
            c.hash(state);
            a.hash(state);
        }
    }
}

impl Connection {
    /// Returns a new connection,
    ///
    pub fn new(from: HashSet<Entity>, to: Entity) -> Self {
        Self {
            from,
            to,
            connection_state: HashMap::default(),
            performance: HashMap::default(),
        }
    }

    /// Returns the entity this connection points to,
    ///
    pub fn entity(&self) -> Entity {
        self.to
    }

    /// Add an incoming entity,
    /// 
    pub fn add_incoming(&mut self, incoming: Entity) {
        self.from.insert(incoming);
    }

    /// Returns an iterator over each connection,
    ///
    pub fn connections<'a>(&'a self) -> impl Iterator<Item = (&'a Entity, &'a Entity)> {
        self.from.iter().map(|f| (f, &self.to))
    }

    /// Returns an iterator over the connection state,
    ///
    pub fn connection_state<'a>(
        &'a self,
    ) -> impl Iterator<Item = (&'a ConnectionState, &'a Activity)> {
        self.connection_state.iter()
    }

    /// Returns an iterator over performance of connections,
    ///
    pub fn performance<'a>(&'a self) -> impl Iterator<Item = (&'a Entity, &'a Histogram<u32>)> {
        self.performance.iter()
    }

    /// Schedules an incoming connection,
    ///
    pub fn schedule(&mut self, incoming: Entity) {
        if self.from.contains(&incoming) {
            let key = ConnectionState::original(incoming);
            self.connection_state.insert(key, Activity::schedule());

            if !self.performance.contains_key(&key.source()) {
                self.performance.insert(
                    key.source(),
                    Histogram::<u32>::new(3).expect("should be able to create histogram"),
                );
            }
        }
    }

    /// Starts a scheduled incoming connection,
    ///
    pub fn start(&mut self, incoming: Entity) {
        if self.from.contains(&incoming) {
            let connection_state = &ConnectionState::original(incoming);
            if let Some(activity) = self.connection_state.get(connection_state) {
                if let Some(start) = activity.start() {
                    event!(
                        Level::DEBUG,
                        "\n\n\tConnection update\n\tincoming event {}\n\tto       event {}\n\t{}\n",
                        incoming.id(),
                        self.to.id(),
                        start
                    );
                    self.connection_state
                        .insert(connection_state.clone(), start);
                }
            }
        }
    }

    /// Completes an active connection,
    ///
    pub fn complete(&mut self, incoming: Entity, error: Option<&ErrorContext>) {
        if self.from.contains(&incoming) {
            let connection_state = &ConnectionState::original(incoming);
            if let Some(activity) = self.connection_state.get(connection_state) {
                let completed = activity.complete(error);
                event!(
                    Level::DEBUG,
                    "\n\n\tConnection update\n\tincoming event {}\n\tto       event {}\n\t{}\n",
                    incoming.id(),
                    self.to.id(),
                    completed
                );
                self.connection_state
                    .insert(*connection_state, completed.clone());

                if let (Some(duration), Some(perf)) = (
                    completed.duration_ms(),
                    self.performance.get_mut(&connection_state.source()),
                ) {
                    match perf.record(duration) {
                        Ok(_) => {}
                        Err(err) => {
                            event!(Level::ERROR, "Could not record connection perf, {err}")
                        }
                    }
                }
            }
        }
    }
}
