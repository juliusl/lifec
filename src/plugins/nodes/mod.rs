use super::{Display, Edit, Plugin, Render};
use crate::AttributeGraph;
use atlier::system::Extension;
use imgui::Window;
use imnodes::{editor, AttributeId, InputPinId, NodeId, OutputPinId};
use specs::storage::DenseVecStorage;
use specs::{
    Builder, Component, Entities, Entity, EntityBuilder, Join, ReadStorage, RunNow, System, World,
    WorldExt, WriteStorage,
};
use std::collections::{HashMap, HashSet};

pub mod demo;

/// This component renders a graph to an editor node
#[derive(Component, Clone, Default, Hash, PartialEq)]
#[storage(DenseVecStorage)]
pub struct NodeContext {
    graph: AttributeGraph,
    node_id: Option<NodeId>,
    attribute_id: Option<AttributeId>,
    input_pin_id: Option<InputPinId>,
    output_pin_id: Option<OutputPinId>,
}

impl Eq for NodeContext {}

impl NodeContext {
    pub fn enable_input(&mut self) {
        self.as_mut().with_bool("enable_input", true);
    }

    pub fn enable_output(&mut self) {
        self.as_mut().with_bool("enable_output", true);
    }

    pub fn enable_attribute(&mut self) {
        self.as_mut().with_bool("enable_attribute", true);
    }

    pub fn node_title(&self) -> Option<String> {
        self.as_ref().find_text("node_title")
    }

    pub fn input_label(&self) -> Option<String> {
        self.as_ref().find_text("input_label")
    }

    pub fn output_label(&self) -> Option<String> {
        self.as_ref().find_text("output_label")
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

pub struct Node {
    _context: imnodes::Context,
    editor_context: imnodes::EditorContext,
    idgen: imnodes::IdentifierGenerator,
    contexts: HashSet<NodeContext>,
    edit: HashMap<NodeContext, Edit<NodeContext>>,
    display: HashMap<NodeContext, Display<NodeContext>>,
}

impl Node {
    pub fn add_node_from(path: impl AsRef<str>, world: &mut World, init: impl Fn(EntityBuilder) -> Entity) -> Option<Entity> {
        if let Some(node) = AttributeGraph::load_from_file(path) {
            let context = NodeContext::from(node);

            let entity = world.create_entity().with(context);

            Some(init(entity))
        } else {
            None
        }
    }
}

impl Default for Node {
    fn default() -> Self {
        Self::new()
    }
}

impl Node {
    pub fn new() -> Self {
        Self::from(imnodes::Context::new())
    }
}

impl Plugin<NodeContext> for Node {
    fn symbol() -> &'static str {
        "node"
    }

    fn call_with_context(_: &mut NodeContext) {
        //
    }
}

impl From<imnodes::Context> for Node {
    fn from(context: imnodes::Context) -> Self {
        let editor_context = context.create_editor();
        let idgen = editor_context.new_identifier_generator();
        Self {
            _context: context,
            editor_context,
            idgen,
            contexts: HashSet::new(),
            edit: HashMap::new(),
            display: HashMap::new(),
        }
    }
}

impl Extension for Node {
    fn configure_app_world(world: &mut World) {
        world.register::<NodeContext>();
        world.register::<Edit<NodeContext>>();
        world.register::<Display<NodeContext>>();
    }

    fn configure_app_systems(_: &mut specs::DispatcherBuilder) {
        // NO-OP
    }

    fn on_ui(&mut self, app_world: &World, ui: &imgui::Ui) {
        let mut frame = Render::<NodeContext>::next_frame(ui);

        self.run_now(app_world);

        Window::new("node_editor").build(ui, || {
            ui.text(format!("count: {}", self.contexts.len()));

            editor(&mut self.editor_context, |mut editor_scope| {
                for mut context in self.contexts.iter().cloned() {
                    let edit = self.edit.get(&context).and_then(|e| Some(e.to_owned()));
                    let display = self.display.get(&context).and_then(|d| Some(d.to_owned()));
                    if let Some(node_id) = &context.node_id {
                        editor_scope.add_node(*node_id, |mut node_scope| {
                            node_scope.add_titlebar(|| {
                                let config = context.clone();
                                if let Some(node_title) = config.node_title() {
                                    ui.text(node_title);
                                }
                            });

                            if let Some(input_pin_id) = &context.input_pin_id {
                                node_scope.add_input(
                                    *input_pin_id,
                                    imnodes::PinShape::Circle,
                                    || {
                                        let config = context.clone();
                                        if let Some(input_label) = config.input_label() {
                                            ui.text(input_label);
                                        }
                                    },
                                );
                            }

                            if let Some(attribute_id) = &context.attribute_id {
                                node_scope.attribute(*attribute_id, || {
                                    let config = context.clone();
                                    let graph = context.as_mut();

                                    frame.render_graph(
                                        graph,
                                        config,
                                        edit.clone(),
                                        display.clone(),
                                    );
                                });
                            }

                            if let Some(output_pin_id) = &context.output_pin_id {
                                node_scope.add_output(
                                    *output_pin_id,
                                    imnodes::PinShape::Triangle,
                                    || {
                                        let config = context.clone();
                                        if let Some(output_label) = config.output_label() {
                                            ui.text(output_label);
                                        }
                                    },
                                );
                            }
                        });
                    }
                }
            });
        });
    }
}

impl<'a> System<'a> for Node {
    type SystemData = (
        WriteStorage<'a, NodeContext>,
        ReadStorage<'a, Edit<NodeContext>>,
        ReadStorage<'a, Display<NodeContext>>,
    );

    fn run(&mut self, (mut contexts, edit_node, display_node): Self::SystemData) {
        for (context, edit_node, display_node) in
            (&mut contexts, edit_node.maybe(), display_node.maybe()).join()
        {
            if edit_node.is_some() || display_node.is_some() {
                if let None = context.node_id {
                    context.node_id = Some(self.idgen.next_node());

                    if let None = context.input_pin_id {
                        if let Some(true) = context.as_ref().is_enabled("enable_input") {
                            context.input_pin_id = Some(self.idgen.next_input_pin());
                        }
                    }

                    if let None = context.attribute_id {
                        if let Some(true) = context.as_ref().is_enabled("enable_attribute") {
                            context.attribute_id = Some(self.idgen.next_attribute());
                        }
                    }

                    if let None = context.output_pin_id {
                        if let Some(true) = context.as_ref().is_enabled("enable_output") {
                            context.output_pin_id = Some(self.idgen.next_output_pin());
                        }
                    }

                    if let Some(edit_node) = edit_node {
                        println!("found edit node for {:?}", context.node_id);
                        self.edit.insert(context.clone(), edit_node.clone());
                    }

                    if let Some(display_node) = display_node {
                        println!("found display node for {:?}", context.node_id);
                        self.display.insert(context.clone(), display_node.clone());
                    }

                    self.contexts.insert(context.clone());
                }
            }
        }
    }
}
