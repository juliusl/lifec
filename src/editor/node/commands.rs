use specs::Entity;

use crate::state::AttributeGraph;

use super::Node;

/// Enumeration of node commands,
///
#[derive(PartialEq, Eq, PartialOrd, Hash)]
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
    /// Command to update state,
    ///
    Update(AttributeGraph),
    /// Custom command for this node,
    ///
    /// This allows for extending capabilities of the node,
    ///
    Custom(&'static str, Entity),
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

    /// Dispatch a command to update a graph,
    /// 
    fn update(&mut self, graph: AttributeGraph);

    /// Dispatch a custom command,
    /// 
    fn custom(&mut self, name: &'static str, entity: Entity);
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

    fn update(&mut self, graph: AttributeGraph) {
        self.command = Some(NodeCommand::Update(graph));
    }

    fn custom(&mut self, name: &'static str, entity: Entity) {
        self.command = Some(NodeCommand::Custom(name, entity));
    }
}