use specs::Entity;

use crate::{
    engine::{CommandDispatcher, NodeCommand},
    state::AttributeGraph,
};

use super::Node;

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

    fn swap(&mut self, owner: Entity, from: Entity, to: Entity) {
        self.command = Some(NodeCommand::Swap { owner, from, to });
    }
}
