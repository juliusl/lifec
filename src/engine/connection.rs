use std::collections::{HashMap, HashSet};

use specs::{Component, Entity, VecStorage};

use crate::prelude::ErrorContext;

use super::Activity;

/// Connection state for a connected entity,
///
/// Struct with the incoming entity. At runtime when the connection is being evaluated,
/// the incoming entity will have an operation with the result.
///
/// The connection state will always have a way to look up the original components.
///
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
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
#[derive(Component, Debug, Clone)]
#[storage(VecStorage)]
pub struct Connection {
    /// Set of entities of incoming connections,
    from: HashSet<Entity>,
    /// Owner of this connection,
    to: Entity,
    /// Map of the connection state,
    connection_state: HashMap<ConnectionState, Activity>,
}

impl Connection {
    /// Returns a new connection,
    /// 
    pub fn new(from: HashSet<Entity>, to: Entity) -> Self {
        Self {
            from,
            to,
            connection_state: HashMap::default(),
        }
    }

    /// Returns an iterator over each connection,
    ///
    pub fn connections<'a>(&'a self) -> impl Iterator<Item = (&'a Entity, &'a Entity)> {
        self.from.iter().map(|f| (f, &self.to))
    }

    /// Returns an iterator over the connection state,
    /// 
    pub fn connection_state<'a>(&'a self) -> impl Iterator<Item = (&'a ConnectionState, &'a Activity)> {
        self.connection_state.iter()
    }

    /// Schedules an incoming connection,
    ///
    pub fn schedule(&mut self, incoming: Entity) {
        if self.from.contains(&incoming) {
            self.connection_state
                .insert(ConnectionState::original(incoming), Activity::schedule());
        }
    }

    /// Starts a scheduled incoming connection,
    ///
    pub fn start(&mut self, incoming: Entity) {
        if self.from.contains(&incoming) {
            let connection_state = &ConnectionState::original(incoming);
            if let Some(activity) = self.connection_state.get(connection_state) {
                if let Some(start) = activity.start() {
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
                self.connection_state
                    .insert(connection_state.clone(), activity.complete(error));
            }
        }
    }
}
