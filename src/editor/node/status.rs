use specs::{Entity, Component, DenseVecStorage};

use crate::{engine::EngineStatus, prelude::EventStatus};

/// Enumeration of node statuses,
///
#[derive(Component, Debug, Default, Hash, PartialEq, Eq, Clone, Copy)]
#[storage(DenseVecStorage)]
pub enum NodeStatus {
    /// Engine status,
    Engine(EngineStatus),
    /// Event status,
    Event(EventStatus),
    /// This is a termination point for event nodes that are adhoc operations
    Profiler(Entity),
    #[default]
    Empty,
}

impl NodeStatus {
    /// Returns the entity,
    /// 
    pub fn entity(&self) -> Entity {
        match self {
            NodeStatus::Event(status) => status.entity(),
            NodeStatus::Profiler(entity) => *entity, 
            NodeStatus::Engine(status) => status.entity(),
            NodeStatus::Empty => unimplemented!("empty node has no status")
        }
    }
}