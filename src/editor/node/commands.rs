use std::fmt::Display;

use specs::{Component, Entity, HashMapStorage};

use crate::state::{AttributeGraph, AttributeIndex};

use super::Node;

/// Enumeration of node commands,
///
#[derive(Component, Debug, Clone, PartialEq, Eq, PartialOrd, Hash)]
#[storage(HashMapStorage)]
pub enum NodeCommand {
    /// Command to activate this node,
    ///
    Activate(Entity),
    /// Command to reset this node,
    ///
    Reset(Entity),
    /// Command to pause this node,
    ///
    Pause(Entity),
    /// Command to resume a paused node,
    ///
    Resume(Entity),
    /// Command to cancel this node,
    ///
    Cancel(Entity),
    /// Command to spawn this node,
    ///
    Spawn(Entity),
    /// Command to update state,
    ///
    Update(AttributeGraph),
    /// Custom command for this node,
    ///
    /// This allows for extending capabilities of the node,
    ///
    Custom(String, Entity),
}

impl NodeCommand {
    /// Returns a custom node command,
    /// 
    pub fn custom(name: impl AsRef<str>, node: Entity) -> Self {
        NodeCommand::Custom(name.as_ref().to_string(), node)
    }
}

impl Display for NodeCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeCommand::Activate(e) => write!(f, "activate {}", e.id()),
            NodeCommand::Reset(e) => write!(f, "reset {}", e.id()),
            NodeCommand::Pause(e) => write!(f, "pause {}", e.id()),
            NodeCommand::Resume(e) => write!(f, "resume {}", e.id()),
            NodeCommand::Cancel(e) => write!(f, "cancel {}", e.id()),
            NodeCommand::Spawn(e) => write!(f, "spawn {}", e.id()),
            NodeCommand::Update(g) => write!(f, "update {}", g.entity_id()),
            NodeCommand::Custom(name, e) => write!(f, "custom.{name} {}", e.id()),
        }
    }
}

/// Extension for Node struct to dispatch commands,
///
pub trait CommandDispatcher {
    /// Dispatch a command to activate entity,
    ///
    fn activate(&mut self, entity: Entity);

    /// Dispatch a command to pause entity,
    ///
    fn pause(&mut self, entity: Entity);

    /// Dispatch a command to reset entity,
    ///
    fn reset(&mut self, entity: Entity);

    /// Dispatch a command to resume entity,
    ///
    fn resume(&mut self, entity: Entity);

    /// Dispatch a command to cancel entity,
    ///
    fn cancel(&mut self, entity: Entity);

    /// Dispatches a command to spawn an entity,
    ///
    fn spawn(&mut self, source: Entity);

    /// Dispatch a command to update a graph,
    ///
    fn update(&mut self, graph: AttributeGraph);

    /// Dispatch a custom command,
    ///
    fn custom(&mut self, name: impl AsRef<str>, entity: Entity);
}

impl CommandDispatcher for Node {
    fn activate(&mut self, entity: Entity) {
        self.command = Some(NodeCommand::Activate(entity));
    }

    fn pause(&mut self, entity: Entity) {
        self.command = Some(NodeCommand::Pause(entity));
    }

    fn reset(&mut self, entity: Entity) {
        self.command = Some(NodeCommand::Reset(entity));
    }

    fn resume(&mut self, entity: Entity) {
        self.command = Some(NodeCommand::Resume(entity));
    }

    fn cancel(&mut self, entity: Entity) {
        self.command = Some(NodeCommand::Cancel(entity));
    }

    fn spawn(&mut self, entity: Entity) {
        self.command = Some(NodeCommand::Spawn(entity));
    }

    fn update(&mut self, graph: AttributeGraph) {
        self.command = Some(NodeCommand::Update(graph));
    }

    fn custom(&mut self, name: impl AsRef<str>, entity: Entity) {
        self.command = Some(NodeCommand::Custom(name.as_ref().to_string(), entity));
    }
}
