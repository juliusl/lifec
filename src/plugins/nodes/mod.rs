use super::{Display, Edit, Plugin, Render};
use crate::AttributeGraph;
use atlier::system::{Extension, Value};
use imgui::{Condition, Window};
use imnodes::{
    editor, AttributeFlag, AttributeId, CoordinateSystem, InputPinId, Link, LinkId, NodeId,
    OutputPinId,
};
use specs::storage::DenseVecStorage;
use specs::{
    Component, Entities, Entity, Join, ReadStorage, RunNow, System, World, WorldExt, WriteStorage,
};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

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
    /// Enable input for this node.
    /// In the UI this is the input pin.
    pub fn enable_input(&mut self) {
        self.as_mut().with_bool("enable_input", true);
    }

    /// Enable output component for this node.
    /// In the UI this is the output pin.
    pub fn enable_output(&mut self) {
        self.as_mut().with_bool("enable_output", true);
    }

    /// Enable attribute component for this node.
    /// In the UI this will enable rendering the attribute render component.
    pub fn enable_attribute(&mut self) {
        self.as_mut().with_bool("enable_attribute", true);
    }

    /// Returns the current title of the node.
    pub fn node_title(&self) -> Option<String> {
        self.as_ref().find_text("node_title")
    }

    /// Returns the current input label for this node.
    pub fn input_label(&self) -> Option<String> {
        self.as_ref().find_text("input_label")
    }

    /// Returns the current output label for this node.
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

/// Add's a node editor using imnodes
/// Reads/Initializes from node.runmd to modify editor settings
pub struct Node {
    editor_context: imnodes::EditorContext,
    idgen: imnodes::IdentifierGenerator,
    contexts: HashSet<NodeContext>,
    edit: HashMap<NodeContext, Edit<NodeContext>>,
    display: HashMap<NodeContext, Display<NodeContext>>,
    link_index: HashMap<LinkId, Link>,
    graph: AttributeGraph,
    // TODO: Need to hold a context to this, because if it leaves scope it will drop
    // But that means we can only have 1 node editor window open
    _context: imnodes::Context,
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
        // No-OP
    }
}

impl From<imnodes::Context> for Node {
    fn from(context: imnodes::Context) -> Self {
        let editor_context = context.create_editor();
        let idgen = editor_context.new_identifier_generator();

        if let Some(config) = AttributeGraph::load_from_file("node.runmd") {
            Self {
                _context: context,
                editor_context,
                idgen,
                graph: config,
                contexts: HashSet::new(),
                edit: HashMap::new(),
                display: HashMap::new(),
                link_index: HashMap::new(),
            }
        } else {
            Self {
                _context: context,
                editor_context,
                idgen,
                graph: AttributeGraph::default(),
                contexts: HashSet::new(),
                edit: HashMap::new(),
                display: HashMap::new(),
                link_index: HashMap::new(),
            }
        }
    }
}

impl Extension for Node {
    fn configure_app_world(world: &mut World) {
        world.register::<NodeContext>();
        world.register::<Edit<NodeContext>>();
        world.register::<Display<NodeContext>>();
    }

    fn configure_app_systems(builder: &mut specs::DispatcherBuilder) {
        builder.add(NodeSync::default(), "node_sync", &[]);
    }

    fn on_ui(&mut self, app_world: &World, ui: &imgui::Ui) {
        let mut frame = Render::<NodeContext>::next_frame(ui);
        self.run_now(app_world);

        let mut size = [800.0, 600.0];
        if let Some(Value::FloatPair(width, height)) = self.graph.find_attr_value("size") {
            size[0] = *width as f32;
            size[1] = *height as f32;
        }

        let mut node_editor_window_title = format!("node_editor");
        if let Some(window_title) = self.graph.find_text("window_title") {
            node_editor_window_title = window_title;
        }

        Window::new(format!(
            "{} - {}",
            node_editor_window_title,
            self.graph.hash_code()
        ))
        .menu_bar(true)
        .size(size, Condition::Appearing)
        .build(ui, || {
            let detatch = self
                .editor_context
                .push(AttributeFlag::EnableLinkDetachWithDragClick);
            let outer_scope = editor(&mut self.editor_context, |mut editor_scope| {
                editor_scope.add_mini_map(imnodes::MiniMapLocation::BottomRight);

                for mut context in self.contexts.iter().cloned() {
                    if let Some(node_id) = &context.node_id {
                        let edit = self.edit.get(&context).and_then(|e| Some(e.to_owned()));
                        let display = self.display.get(&context).and_then(|d| Some(d.to_owned()));

                        editor_scope.add_node(*node_id, |mut node_scope| {
                            node_scope.add_titlebar(|| {
                                if let Some(node_title) = context.node_title() {
                                    ui.text(node_title);
                                }
                            });

                            if let Some(input_pin_id) = &context.input_pin_id {
                                node_scope.add_input(
                                    *input_pin_id,
                                    imnodes::PinShape::Circle,
                                    || {
                                        if let Some(input_label) = context.input_label() {
                                            ui.text(input_label);
                                        }
                                    },
                                );
                            }

                            if let Some(attribute_id) = &context.attribute_id {
                                node_scope.attribute(*attribute_id, || {
                                    let config = context.clone();
                                    let graph = context.as_mut();

                                    // If the entity has an edit/display, it's shown in this block
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
                                        if let Some(output_label) = context.output_label() {
                                            ui.text(output_label);
                                        }
                                    },
                                );
                            }
                        });
                    }
                }

                for (
                    link_id,
                    Link {
                        start_pin, end_pin, ..
                    },
                ) in &self.link_index
                {
                    editor_scope.add_link(*link_id, *end_pin, *start_pin);
                }
            });

            if let Some(link) = outer_scope.links_created() {
                println!("Link created {:?}", link);
                self.link_index.insert(self.idgen.next_link(), link);
            }

            if let Some(dropped) = outer_scope.get_dropped_link() {
                if let Some(dropped) = self.link_index.remove(&dropped) {
                    println!("Link dropped {:?}", dropped);
                }
            }

            detatch.pop();
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

        if let Some(config) = AttributeGraph::load_from_file("node.runmd") {
            if config.hash_code() != self.graph.hash_code() {
                self.graph = config;
            }
        }
    }
}

#[derive(Default)]
struct NodeSync(HashMap<NodeContext, Entity>);

impl<'a> System<'a> for NodeSync {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, NodeContext>,
        WriteStorage<'a, Display<NodeContext>>,
    );

    fn run(&mut self, (entities, mut contexts, mut displays): Self::SystemData) {
        if let Some(config) = AttributeGraph::load_from_file("node.runmd") {
            // load each node block to the graph
            for block in config.find_blocks("node") {
                let mut context = NodeContext::from(block);

                let NodeSync(added) = self;
                let original = context.clone();

                if let None = added.get(&original) {
                    let entity = entities.create();
                    context.as_mut().set_parent_entity(entity, true);

                    match contexts.insert(entity, context) {
                        Ok(_) => {
                            println!("Loaded new node_context entity {:?}", entity);
                            added.insert(original, entity);
                            match displays.insert(
                                entity,
                                Display(|c, _, ui| {
                                    ui.text(format!(
                                        "{:?}",
                                        c.node_id.and_then(|n| Some(
                                            n.get_position(CoordinateSystem::ScreenSpace)
                                        ))
                                    ));
                                }),
                            ) {
                                Ok(_) => {
                                    println!(
                                        "Added display for new node_context entity {:?}",
                                        entity
                                    );
                                }
                                Err(_) => {}
                            }
                        }
                        Err(_) => {}
                    }
                }
            }
        }
    }
}
