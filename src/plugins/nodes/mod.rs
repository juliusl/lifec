use super::thunks::Form;
use super::{
    BlockContext, Display, Edit, Engine, Plugin, Process, Render, ThunkContext, WriteFiles,
};
use crate::plugins::Println;
use crate::{AttributeGraph, RuntimeState};
use atlier::system::{Extension, Value};
use imgui::{Condition, Ui, Window, MenuItem};
use imnodes::{
    editor, AttributeFlag, AttributeId, CoordinateSystem, InputPinId, Link, LinkId, NodeId,
    OutputPinId,
};
use specs::storage::DenseVecStorage;
use specs::{
    Component, Entities, Entity, Join, ReadStorage, RunNow, System, World, WorldExt, WriteStorage,
};
use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;
use std::time::{Duration, Instant};

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
    pub fn node_pos(&self) -> Option<(&f32, &f32)> {
        self.as_ref()
            .find_attr_value("node_pos")
            .and_then(|a| match a {
                Value::FloatPair(x, y) => Some((x, y)),
                _ => None,
            })
    }

    pub fn node_pos_next(&self) -> Option<(&f32, &f32)> {
        self.as_ref()
            .find_attr("node_pos")
            .and_then(|a| a.transient())
            .and_then(|(_, a)| match a {
                Value::FloatPair(x, y) => Some((x, y)),
                _ => None,
            })
    }

    pub fn set_next_pos(&mut self, x: f32, y: f32) {
        self.as_mut()
            .find_update_attr("node_pos", |a| a.edit_as(Value::FloatPair(x, y)));
    }

    pub fn emit_current_pos(&mut self, x: f32, y: f32) {
        self.as_mut().find_update_attr("node_pos", |a| {
            a.edit_as(Value::FloatPair(x, y));
            a.commit();
        });
    }

    /// Returns the current title of the node.
    pub fn node_title(&self) -> Option<String> {
        self.as_ref()
            .find_block("", "node")
            .and_then(|node| node.find_text("node_title"))
    }

    /// Returns the current input label for this node.
    pub fn input_label(&self) -> Option<String> {
        self.as_ref()
            .find_block("", "node")
            .and_then(|node| node.find_text("input_label"))
    }

    /// Returns the current output label for this node.
    pub fn output_label(&self) -> Option<String> {
        self.as_ref()
            .find_block("", "node")
            .and_then(|node| node.find_text("output_label"))
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
    contexts: HashMap<NodeId, NodeContext>,
    edit: HashMap<NodeId, Edit<NodeContext>>,
    display: HashMap<NodeId, Display<NodeContext>>,
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

    /// Create the link object between two node contexts
    pub fn link(from: NodeContext, to: NodeContext) -> Option<Link> {
        if let (Some(start_node), Some(start_pin), Some(end_node), Some(end_pin)) = (
            from.node_id,
            from.output_pin_id,
            to.node_id,
            to.input_pin_id,
        ) {
            Some(Link {
                start_node,
                end_node,
                start_pin,
                end_pin,
                craeated_from_snap: false,
            })
        } else {
            None
        }
    }

    /// Reverse lookup node_contexts from link
    pub fn reverse_lookup(&self, link: &Link) -> Option<(&NodeContext, &NodeContext)> {
        let Link {
            start_node,
            end_node,
            ..
        } = link;

        let start = self.contexts.get(start_node);
        let end = self.contexts.get(end_node);

        if let (Some(from), Some(to)) = (start, end) {
            Some((from, to))
        } else {
            None
        }
    }

    /// reverse lookup a link into block contexts
    pub fn reverse_lookup_blocks(&self, link: &Link) -> Option<(BlockContext, BlockContext)> {
        self.reverse_lookup(link).and_then(|(from, to)| {
            Some((
                BlockContext::from(from.as_ref().clone()),
                BlockContext::from(to.as_ref().clone()),
            ))
        })
    }

    /// gets all values from the publish block of "from"
    /// and writes to the the accept block of "to"
    pub fn connect(&mut self, link: &Link) {
        if let Some((from, to)) = &self.reverse_lookup(link) {
            if let Some(to_node_id) = to.node_id {
                let from = from.as_ref().clone();
                let from = BlockContext::from(from);
                let to = to.as_ref().clone();
                let mut to = BlockContext::from(to);

                if let Some(publish) = from.get_block("publish") {
                    to.update_block("accept", |accepting| {
                        for (name, value) in publish
                            .iter_attributes()
                            .filter(|a| !a.name().starts_with("block_"))
                            .filter_map(|a| Some((a.name(), a.value())))
                        {
                            accepting.with(name, value.clone());
                        }
                    });

                    if let Some(to_update) = self.contexts.get_mut(&to_node_id) {
                        to_update.as_mut().merge(to.as_ref());
                    }
                }
            }
        }
    }

    /// cleans up the accept cache of to
    pub fn disconnect(&mut self, link: &Link) {
        if let Some((from, to)) = &self.reverse_lookup(link) {
            if let Some(to_node_id) = to.node_id {
                let from = from.as_ref().clone();
                let from = BlockContext::from(from);
                let to = to.as_ref().clone();
                let to = BlockContext::from(to);

                if let (Some(publish), Some(accept)) =
                    (from.get_block("publish"), to.get_block("accept"))
                {
                    if let Some(to_update) = self.contexts.get_mut(&to_node_id) {
                        for mut attr in publish
                            .iter_attributes()
                            .filter(|a| !a.name().starts_with("block_"))
                            .cloned()
                        {
                            attr.set_id(accept.entity());
                            to_update.as_mut().remove(&attr);
                        }
                    }
                }
            }
        }
    }
}

impl Plugin<NodeContext> for Node {
    fn symbol() -> &'static str {
        "node"
    }

    fn call_with_context(_: &mut NodeContext) {
        // No-OP
    }

    fn on_event(&mut self, context: &mut NodeContext)
    where
        Self: Engine + Sized,
    {
        if let None = context.node_id {
            let node_id = self.idgen.next_node();
            context.node_id = Some(node_id);

            if let None = context.attribute_id {
                if context.as_ref().find_block("", "form").is_some()
                    || context.as_ref().find_block("", "thunk").is_some()
                {
                    context.attribute_id = Some(self.idgen.next_attribute());
                }
            }
            if let None = context.output_pin_id {
                context.output_pin_id = Some(self.idgen.next_output_pin());
            }
            if let None = context.input_pin_id {
                context.input_pin_id = Some(self.idgen.next_input_pin());
            }
            self.contexts.insert(node_id, context.clone());
        }
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
                contexts: HashMap::new(),
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
                contexts: HashMap::new(),
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
        world.register::<Println>();
        world.register::<WriteFiles>();
        world.register::<ThunkContext>();
    }

    fn configure_app_systems(builder: &mut specs::DispatcherBuilder) {
        let write_files = NodeSync::<WriteFiles>::default();
        let println = NodeSync::<Println>::default();
        let process = NodeSync::<Process>::default();
        let form = NodeSync::<Form>::default();

        builder.add(write_files, "write_files_node_sync", &[]);
        builder.add(println, "println_node_sync", &[]);
        builder.add(process, "process_node_sync", &[]);
        builder.add(form, "form_node_sync", &[]);
    }

    fn on_ui(&mut self, app_world: &World, ui: &imgui::Ui) {
        let mut frame = Render::<NodeContext>::next_frame(ui);

        let mut size = [800.0, 600.0];
        if let Some(Value::FloatPair(width, height)) = self.graph.find_attr_value("size") {
            size[0] = *width as f32;
            size[1] = *height as f32;
        }

        let mut node_editor_window_title = format!("node_editor");
        if let Some(window_title) = self.graph.find_text("window_title") {
            node_editor_window_title = window_title;
        }

        Window::new(format!("{}", node_editor_window_title,))
            .menu_bar(true)
            .size(size, Condition::Appearing)
            .build(ui, || {
                ui.menu_bar(|| {
                    self.graph.edit_attr_menu(ui);

                    let mut ordered = BTreeMap::default();
                    for (_, context) in self.contexts.iter() {
                        let block = context.as_ref().clone();
                        let block = BlockContext::from(block);
                        ordered.insert(block.block_name(), block);
                    }

                    for (_, block) in ordered.iter_mut() {
                        block.edit_menu(ui);
                    }
                });

                ui.label_text("hash", format!("{}", self.graph.hash_code()));
                
                let detatch = self
                    .editor_context
                    .push(AttributeFlag::EnableLinkDetachWithDragClick);
                let outer_scope = editor(&mut self.editor_context, |mut editor_scope| {
                    editor_scope.add_mini_map(imnodes::MiniMapLocation::BottomRight);

                    for (node_id, mut context) in self.contexts.iter_mut() {
                        let edit = self.edit.get(&node_id).and_then(|e| Some(e.to_owned()));
                        let display = self.display.get(&node_id).and_then(|d| Some(d.to_owned()));

                        editor_scope.add_node(*node_id, |mut node_scope| {
                            let imnodes::ImVec2 { x, y } =
                                node_id.get_position(CoordinateSystem::ScreenSpace);
                            context.emit_current_pos(x, y);

                            node_scope.add_titlebar(|| {
                                if let Some(node_title) = context.node_title() {
                                    ui.text(node_title);
                                }
                            });

                            if let Some(input_pin_id) = &context.input_pin_id {
                                if let Some(_) = context.as_ref().find_block("", "accept") {
                                    node_scope.add_input(
                                        *input_pin_id,
                                        imnodes::PinShape::Circle,
                                        || {
                                            if let Some(input_label) = context.input_label() {
                                                ui.set_next_item_width(130.0);
                                                ui.label_text(input_label, "input");
                                            }
                                        },
                                    );
                                }
                            }

                            if let Some(attribute_id) = &context.attribute_id {
                                node_scope.attribute(*attribute_id, || {
                                    if let Some(true) = context.as_ref().is_enabled("debug") {
                                        let imnodes::ImVec2 { x, y } =
                                            node_id.get_position(CoordinateSystem::ScreenSpace);
                                        ui.text(format!("x: {}, y: {}", x, y));
                                        let imnodes::ImVec2 {
                                            x: width,
                                            y: height,
                                        } = node_id.get_dimensions();
                                        ui.text(format!("width: {}", width));
                                        ui.text(format!("height: {}", height));
                                        ui.text(format!("entity: {}", context.as_ref().entity()));
                                    }

                                    // If the entity has an edit/display, it's shown in this block
                                    frame.on_render(&mut context, edit.clone(), display.clone());
                                });
                            }

                            if let Some(output_pin_id) = &context.output_pin_id {
                                if let Some(_) = context.as_ref().find_block("", "publish") {
                                    node_scope.add_output(
                                        *output_pin_id,
                                        imnodes::PinShape::Triangle,
                                        || {
                                            if let Some(output_label) = context.output_label() {
                                                ui.set_next_item_width(130.0);
                                                ui.label_text("output", output_label);
                                            }
                                        },
                                    );
                                }
                            }
                        });
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
                    self.connect(&link);
                }

                if let Some(dropped) = outer_scope.get_dropped_link() {
                    if let Some(dropped) = self.link_index.remove(&dropped) {
                        self.disconnect(&dropped);
                        println!("Link dropped {:?}", dropped);
                    }
                }

                detatch.pop();
            });

        self.run_now(app_world);
    }
}

impl Engine for Node {
    fn next_mut(&mut self, _: &mut AttributeGraph) {}

    fn exit(&mut self, _: &AttributeGraph) {}
}

impl<'a> System<'a> for Node {
    type SystemData = (
        WriteStorage<'a, NodeContext>,
        ReadStorage<'a, Edit<NodeContext>>,
        ReadStorage<'a, Display<NodeContext>>,
    );

    fn run(&mut self, (mut contexts, edit_node, display_node): Self::SystemData) {
        for (_, graph) in self.contexts.iter() {
            self.graph.merge(graph.as_ref());
        }

        for (context, edit_node, display_node) in
            (&mut contexts, edit_node.maybe(), display_node.maybe()).join()
        {
            if edit_node.is_some() || display_node.is_some() {
                self.on_event(context);

                if let (Some(edit_node), Some(node_id)) = (edit_node, context.node_id) {
                    if !self.edit.contains_key(&node_id) {
                        println!("found edit node for {:?}", node_id);
                        self.edit.insert(node_id, edit_node.clone());
                    }
                }

                if let (Some(display_node), Some(node_id)) = (display_node, context.node_id) {
                    if !self.display.contains_key(&node_id) {
                        println!("found display node for {:?}", node_id);
                        self.display.insert(node_id, display_node.clone());
                    }
                }
            }
        }
    }
}

#[derive(Default)]
struct NodeSync<P>(HashMap<NodeContext, Entity>, Option<P>, Option<Instant>)
where
    P: Plugin<ThunkContext> + Component + Default;

impl<P> NodeSync<P>
where
    P: Plugin<ThunkContext> + Component + Default,
{
    fn render_node(_: &NodeContext, graph: &mut AttributeGraph, ui: &Ui) {
        if let Some(mut _update) = graph.edit_form_block(ui) {
            let imported = _update.entity();
            for attr in _update.iter_mut_attributes() {
                match attr.value() {
                    Value::Symbol(_) => {}
                    _ => {
                        attr.commit();
                        let next_value = attr.value.clone();
                        graph.find_update_imported_attr(imported, &attr.name(), |a| {
                            a.edit_as(next_value);
                            a.commit();
                        });
                    }
                }
            }
        }

        if graph.find_block("", "thunk").is_some() {
            ui.new_line();
            let label = format!("call {} {}", P::symbol(), graph.entity());
            if ui.button(label) {
                P::call(graph);
            }
            ui.new_line();
        }
    }
}

impl<'a, P> System<'a> for NodeSync<P>
where
    P: Plugin<ThunkContext> + Component + Default,
{
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, NodeContext>,
        WriteStorage<'a, Edit<NodeContext>>,
    );

    fn run(&mut self, (entities, mut contexts, mut edits): Self::SystemData) {
        if let Some(instant) = self.2 {
            if instant.elapsed() > Duration::from_millis(16) {
                self.2.take();
            } else {
                return;
            }
        }

        if let Some(source) = AttributeGraph::load_from_file(format!("{}.runmd", P::symbol())) {
            // load each node block to the graph
            for block in source.find_blocks("node") {
                let block_name = block.find_text("block_name").unwrap_or_default();
                if block_name.is_empty() {
                    continue;
                }

                let mut context = BlockContext::root_context(&source, block_name);
                context.update_block("node", |a| {
                    a.with_text("input_label", "accept")
                        .with_text("output_label", "publish");
                });

                let context = NodeContext::from(context.as_ref().clone());
                let NodeSync(added, ..) = self;
                let original = context.clone();

                if let None = added.get(&original) {
                    let entity = entities.create();
                    match contexts.insert(entity, context) {
                        Ok(_) => {
                            println!("Loaded new node_context entity {:?}", entity);
                            added.insert(original, entity);
                            match edits.insert(entity, Edit(Self::render_node)) {
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

            self.2 = Some(Instant::now());
        }
    }
}
