use imnodes::{AttributeId, InputPinId, NodeId, OutputPinId};
use specs::storage::{DenseVecStorage, HashMapStorage};
use specs::{Component, Entities, Join, ReadStorage, RunNow, System, World, WriteStorage};

use crate::AttributeGraph;

use super::Plugin;

pub struct Node {
    editor_context: imnodes::EditorContext,
    idgen: imnodes::IdentifierGenerator,
    graph: AttributeGraph,
}

impl From<AttributeGraph> for Node {
    fn from(graph: AttributeGraph) -> Self {
        let context = imnodes::Context::new();
        let editor_context = context.create_editor();
        let idgen = editor_context.new_identifier_generator();
        Self {
            editor_context,
            idgen,
            graph,
        }
    }
}

impl AsRef<AttributeGraph> for Node {
    fn as_ref(&self) -> &AttributeGraph {
        &self.graph
    }
}

impl AsMut<AttributeGraph> for Node {
    fn as_mut(&mut self) -> &mut AttributeGraph {
        &mut self.graph
    }
}

#[derive(Component)]
#[storage(DenseVecStorage)]
pub struct NodeContext<T>
where
    T: Send + Sync + Sized + 'static,
{
    property: T,
}

impl<'a> System<'a> for Node {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, NodeContext<()>>,
        WriteStorage<'a, NodeContext<(NodeId, AttributeId, InputPinId, OutputPinId)>>,
    );

    fn run(
        &mut self, (
            entities,
            contexts,
            mut node_context,
        ): Self::SystemData,
    ) {
        for (e, context) in (&entities, contexts.maybe()).join() {
            match context {
                Some(_) => {
                    if !node_context.contains(e) {
                        let property = (
                            self.idgen.next_node(),
                            self.idgen.next_attribute(),
                            self.idgen.next_input_pin(),
                            self.idgen.next_output_pin()
                        );
                        match node_context.insert(e, NodeContext { property }) {
                            Ok(_) => todo!(),
                            Err(_) => todo!(),
                        }
                    }
                }
                None => {
                    node_context.remove(e);
                }
            }
        }
    }
}
