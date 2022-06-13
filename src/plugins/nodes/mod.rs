use atlier::system::Extension;
use imgui::ChildWindow;
use imnodes::{editor, AttributeId, InputPinId, NodeId, OutputPinId};
use specs::storage::DenseVecStorage;
use specs::{Component, Join, ReadStorage, RunNow, System, World, WorldExt, WriteStorage};

use crate::{AttributeGraph, RuntimeDispatcher};

use super::{Display, Edit, Render};

/// This component renders a graph to an editor node
#[derive(Component, Clone, Default)]
#[storage(DenseVecStorage)]
pub struct NodeContext {
    graph: AttributeGraph,
    node_id: Option<NodeId>,
    attribute_id: Option<AttributeId>,
    input_pin_id: Option<InputPinId>,
    output_pin_id: Option<OutputPinId>,
}

impl NodeContext {
    pub fn render_title(&self) -> Self {
        self.as_ref()
            .dispatch("define node_title render")
            .ok()
            .unwrap_or_default()
            .into()
    }

    pub fn render_attribute(&self) -> Self {
        self.as_ref()
            .dispatch("define node_attribute render")
            .ok()
            .unwrap_or_default()
            .into()
    }

    pub fn render_input(&self) -> Self {
        self.as_ref()
            .dispatch("define node_input render")
            .ok()
            .unwrap_or_default()
            .into()
    }

    pub fn render_output(&self) -> Self {
        self.as_ref()
            .dispatch("define node_output render")
            .ok()
            .unwrap_or_default()
            .into()
    }
}

impl AsRef<AttributeGraph> for NodeContext {
    fn as_ref(&self) -> &AttributeGraph {
        &self.graph
    }
}

impl AsMut<AttributeGraph> for NodeContext {
    fn as_mut(&mut self) -> &mut AttributeGraph {
        &mut self.graph
    }
}

impl From<AttributeGraph> for NodeContext {
    fn from(graph: AttributeGraph) -> Self {
        Self {
            graph,
            ..Default::default()
        }
    }
}

pub struct Node<'a, 'ui> {
    editor_context: imnodes::EditorContext,
    idgen: imnodes::IdentifierGenerator,
    render_node: Option<Render<'a, 'ui, NodeContext>>,
}

impl<'a, 'ui> Default for Node<'a, 'ui> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, 'ui> Node<'a, 'ui> {
    pub fn new() -> Self {
        Self::from(imnodes::Context::new())
    }
}

impl From<imnodes::Context> for Node<'_, '_> {
    fn from(context: imnodes::Context) -> Self {
        let editor_context = context.create_editor();
        let idgen = editor_context.new_identifier_generator();
        Self { editor_context, idgen, render_node: None }
    }
}

impl<'a, 'ui> Extension<'a, 'ui> for Node<'a, 'ui> {
    fn configure_app_world(world: &mut World) {
        world.register::<NodeContext>();
        world.register::<Edit<NodeContext>>();
        world.register::<Display<NodeContext>>();
    }

    fn configure_app_systems(_: &mut specs::DispatcherBuilder) {
        // NO-OP
    }

    fn on_ui(&'a mut self, app_world: &'a World, ui: &'a imgui::Ui<'ui>) {
        ChildWindow::new("node_editor").build(ui, move || {

            let render = Render::next_frame(ui);

            self.render_node = Some(render);

            self.run_now(app_world);
        });
    }


}

impl<'a> System<'a> for Node<'_, '_> {
    type SystemData = (
        WriteStorage<'a, NodeContext>,
        ReadStorage<'a, Edit<NodeContext>>,
        ReadStorage<'a, Display<NodeContext>>,
    );

    fn run(&mut self, (mut contexts, edit_node, display_node): Self::SystemData) {
        editor(&mut self.editor_context, |mut editor_scope| {
            for (context, edit_node, display_node) in
                (&mut contexts, edit_node.maybe(), display_node.maybe()).join()
            {
                let edit_node = &edit_node.and_then(|e| Some(e.to_owned()));
                let display_node = &display_node.and_then(|e| Some(e.to_owned()));

                if let (Some(node_id), Some(render_node)) = (context.node_id, self.render_node.as_mut()) {
                    editor_scope.add_node(node_id, |mut node_scope| {
                        node_scope.add_titlebar(|| {
                            let config = context.render_title();
                            render_node.render_graph(
                                context.as_mut(),
                                config,
                                edit_node.clone(),
                                display_node.clone(),
                            );
                        });

                        if let Some(input_pin_id) = context.input_pin_id {
                            node_scope.add_input(input_pin_id, imnodes::PinShape::Circle, || {
                                let config = context.render_input();
                                render_node.render_graph(
                                    context.as_mut(),
                                    config,
                                    edit_node.clone(),
                                    display_node.clone(),
                                );
                            });
                        } else {
                            context.input_pin_id = Some(self.idgen.next_input_pin());
                        }

                        if let Some(attrid) = context.attribute_id {
                            node_scope.attribute(attrid, || {
                                let config = context.render_attribute();
                                render_node.render_graph(
                                    context.as_mut(),
                                    config,
                                    edit_node.clone(),
                                    display_node.clone(),
                                );
                            });
                        } else {
                            context.attribute_id = Some(self.idgen.next_attribute());
                        }

                        if let Some(output_pin_id) = context.output_pin_id {
                            node_scope.add_output(
                                output_pin_id,
                                imnodes::PinShape::Triangle,
                                || {
                                    let config = context.render_output();
                                    render_node.render_graph(
                                        context.as_mut(),
                                        config,
                                        edit_node.clone(),
                                        display_node.clone(),
                                    );
                                },
                            );
                        } else {
                            context.output_pin_id = Some(self.idgen.next_output_pin());
                        }
                    })
                } else {
                    context.node_id = Some(self.idgen.next_node());
                }
            }
        });
    }
}
