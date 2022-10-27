use specs::Entity;

use crate::{engine::EngineStatus, prelude::EventStatus};

/// Enumeration of node statuses,
///
#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub enum NodeStatus {
    /// 
    Engine(EngineStatus),
    /// These are event nodes
    Event(EventStatus),
    /// This is a termination point for event nodes that are adhoc operations
    Profiler,
}

impl NodeStatus {
    /// Returns the entity,
    /// 
    pub fn entity(&self) -> Entity {
        match self {
            NodeStatus::Event(status) => status.entity(),
            NodeStatus::Engine(_) | NodeStatus::Profiler => panic!("Not implemented"),
        }
    }
}