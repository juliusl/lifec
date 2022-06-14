use atlier::system::{App, Attribute, Value};
use imgui::*;
use imnodes::{
    editor, AttributeFlag, AttributeId, EditorContext, IdentifierGenerator, ImVec2, InputPinId,
    Link, LinkId, NodeId, OutputPinId, CoordinateSystem,
};
use knot::store::{Store, Visitor};
use ron::ser::PrettyConfig;
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::{AttributeGraph, plugins::ThunkContext};

use super::unique_title;

#[derive(Clone)]
pub struct NodeComponent {
    title: String,
    node_id: NodeId,
    input_id: InputPinId,
    output_id: OutputPinId,
    attribute_id: AttributeId,
    attribute: Attribute,
    thunk: Option<fn(&mut AttributeGraph)>,
    values: Option<AttributeGraph>,
    tooltip: Option<String>,
}

impl NodeComponent {
    pub fn title(&self) -> &String {
        &self.title
    }

    pub fn move_editor_to(&mut self) {
        self.node_id.move_editor_to();
    }

    pub fn move_node_to_grid_center(&mut self) {
        self.move_node_to_grid(400.0, 200.0);
    }

    pub fn move_node_to_grid(&mut self, x: f32, y: f32) {
        self.node_id.set_position(x, y, CoordinateSystem::EditorSpace);
    }

    pub fn move_node_to_screen(&mut self, x: f32, y: f32) {
        self.node_id.set_position(x, y, CoordinateSystem::ScreenSpace);
    }

    /// updates the current state of the node_component
    pub fn update(
        &mut self,
        thunk: fn(&mut AttributeGraph),
        input_name: impl AsRef<str>,
        input: Value,
    ) {
        self.thunk = Some(thunk);

        if let None = self.values {
            self.values = Some(AttributeGraph::default());
        }

        if let Some(values) = self.values.as_mut() {
            values.with(input_name.as_ref().to_string(), input);
        }
    }
}

#[derive(Default)]
pub struct NodeEditorGraph {
    title: String,
    editor_context: Option<imnodes::EditorContext>,
    idgen: Option<imnodes::IdentifierGenerator>,
    thunk_index: BTreeMap<String, fn(&mut AttributeGraph)>,
    link_index: HashMap<LinkId, Link>,
    links: HashSet<Link>,
    value_store: Store<Value>,
    nodes: Vec<NodeComponent>,
    attribute_store: Store<(i32, Attribute)>,
    graph: AttributeGraph
}

impl AsRef<AttributeGraph> for NodeEditorGraph {
    fn as_ref(&self) -> &AttributeGraph {
        &self.graph
    }
}

impl AsMut<AttributeGraph> for NodeEditorGraph {
    fn as_mut(&mut self) -> &mut AttributeGraph {
        &mut self.graph
    }
}

impl App for NodeEditorGraph {
    fn name() -> &'static str {
        "node_editor_graph"
    }

    fn show_editor(&mut self, ui: &imgui::Ui) {
        let debugging = self.as_ref().is_enabled("editing_graph_resources");
        let mut graph = self.graph.clone();

        if let Some(context) = self.editor_context.as_mut() {
            if let Some(debugging) = debugging {
                if debugging {
                    ChildWindow::new("Debugger")
                        .size([400.0, 0.0])
                        .always_auto_resize(true)
                        .build(ui, || {
                            if ui.collapsing_header("Active thunks", TreeNodeFlags::empty()) {
                                if let Some(Value::Bool(pause_updating_graph)) = graph.find_attr_value_mut("pause_updating_graph") {
                                    ui.checkbox("Pause graph updates", pause_updating_graph);
                                    if ui.is_item_hovered() {
                                        ui.tooltip_text("Pausing graph updates allows you to edit the thunk state. Useful for testing thunks w/o modifying existing values directly.");
                                    }
                                }

                                ui.new_line();
                                ui.indent();
                                self.nodes
                                    .iter_mut()
                                    .filter(|n| n.values.is_some() && n.thunk.is_some())
                                    .for_each(|n| {
                                        let values = n
                                            .values
                                            .as_mut()
                                            .expect("filtered only values with some");
                                        if let Value::Symbol(symbol) = n.attribute.value() {

                                            // if !self.pause_updating_graph {
                                            //     thunk_context.state_mut().with(
                                            //         "opened::".to_string(),
                                            //         Value::Bool(true),
                                            //     );
                                            // }

                                            // ui.disabled(!self.pause_updating_graph, || {
                                            //     thunk_context.show_editor(ui);
                                            // });

                                            if ui.button(format!("Call [{}]", symbol)) {
                                                let thunk = n
                                                    .thunk
                                                    .clone()
                                                    .expect("filtered only thunks with some");

                                                thunk(values);
                                            }

                                            ui.same_line();
                                            if ui.button(format!("Move to [{}]", symbol)) {
                                                n.node_id.move_editor_to();
                                            }

                                            if ui.button(format!("Refresh values [{}]", symbol)) {
                                                values.clear_index();
                                            }

                                            ui.new_line();
                                        }
                                    });
                                ui.unindent();
                            }

                            if ui.collapsing_header("Active attributes", TreeNodeFlags::empty()) {
                                if let Some(Value::TextBuffer(filter)) = graph.find_attr_value_mut("filtering_attributes") {
                                    ui.input_text("Filter attributes", filter).build();
                                    ui.new_line();
                                }

                                self.nodes
                                    .iter_mut()
                                    .filter(|n| {
                                        if let Some(Value::TextBuffer(filter)) = graph.find_attr_value("filtering_attributes") {
                                            format!("{}", n.attribute).contains(filter)
                                        } else {
                                            false
                                        }
                                    })
                                    .for_each(|n| {
                                        ui.set_next_item_width(200.0);
                                        ui.input_text(
                                            format!("title {:?}", &n.node_id),
                                            &mut n.title,
                                        )
                                        .build();

                                        n.attribute.edit_ui(ui);
                                        if let Some(values) = &n.values {
                                            ui.text("Current Values:");
                                            ui.new_line();
                                            for t in values.iter_attributes().map(|a| a.value()) {
                                                ui.text(format!("{:?}", t));
                                            }
                                        }
                                        if ui.button(format!("Move editor to {}", n.title)) {
                                            n.node_id.move_editor_to();
                                        }
                                        ui.new_line();
                                        ui.separator();
                                        self.value_store = self.value_store.link_create_if_not_exists(
                                            n.attribute.value().clone(),
                                            Value::Symbol("ACTIVE".to_string()),
                                        );
                                    });
                            }

                            if ui.collapsing_header("Active values", TreeNodeFlags::empty()) {
                                let (seen, _) = self.value_store.new_walk_ordered(
                                    Value::Symbol("ACTIVE".to_string()),
                                    Some(&ValueWalker {}),
                                );

                                for s in seen {
                                    ui.text(s.to_string());
                                }
                            }
                        
                            if ui.collapsing_header("Attribute Store", TreeNodeFlags::empty()) {
                                let attribute_visitor = AttributeWalker { ui }; 
                                 self.attribute_store.new_walk_ordered_now(Some(&attribute_visitor));
                            }
                        });
                    ui.same_line();
                }
            }

            if let Some(ntoken) = ChildWindow::new("Node Editor").size([0.0, 0.0]).begin(ui) {
                let detatch = context.push(AttributeFlag::EnableLinkDetachWithDragClick);
                let outer_scope = editor(context, |mut editor_scope| {
                    editor_scope.add_mini_map(imnodes::MiniMapLocation::BottomRight);
                    self.nodes
                        .iter_mut()
                        .filter(|n| {
                            if let Some(Value::TextBuffer(filtering_nodes)) = graph.find_attr_value("filtering_nodes") {
                                n.title.contains(filtering_nodes)
                            } else {
                                true
                            }
                        })
                        .for_each(|node_component| {
                            let NodeComponent {
                                title,
                                node_id,
                                input_id,
                                output_id,
                                attribute_id,
                                attribute,
                                thunk,
                                values,
                                tooltip,
                            } = node_component;

                            let title = title.as_str();
                            ui.set_next_item_width(130.0);
                            editor_scope.add_node(*node_id, |mut node_scope| {
                                ui.set_next_item_width(130.0);
                                node_scope.add_titlebar(|| {
                                    ui.text(title);
                                });

                                if let atlier::system::Value::Reference(r) =
                                    attribute.clone().value()
                                {
                                    match self.value_store.get_at(r) {
                                        None => {
                                            let resetting = attribute.value_mut();
                                            *resetting = Value::Empty;
                                        }
                                        _ => {}
                                    }
                                }

                                match attribute.value() {
                                    // Empty means this attribute needs a value
                                    atlier::system::Value::Symbol(symbol) => {
                                        node_scope.add_input(
                                            *input_id,
                                            imnodes::PinShape::Triangle,
                                            || {
                                                ui.set_next_item_width(130.0);
                                                ui.label_text("symbol", symbol);
                                            },
                                        );
                                        if let (Some(thunk), Some(values)) = (thunk, values) {
                                            node_scope.attribute(*attribute_id, || {
                                                ui.set_next_item_width(130.0);
                                                let thunk_name = symbol[7..].to_string();
                                                if ui.button(format!("{}", thunk_name)) {
                                                    thunk(values);
                                                }
                                                if ui.is_item_hovered() {
                                                    if let Some(tooltip) = tooltip {
                                                        ui.tooltip_text(tooltip);
                                                    }
                                                }
                                            });

                                            let mut current_outputs = vec![];
                                            values.find_symbol_values("output").iter().for_each(
                                                |(k, o)| {
                                                    current_outputs.push(k.to_string());
                                                    self.value_store = self.value_store.node(o.clone());
                                                },
                                            );

                                            values.find_symbol_values("returns").iter().for_each(
                                                |(k, o)| {
                                                    current_outputs.push(k.to_string());
                                                    self.value_store = self.value_store.node(o.clone());
                                                },
                                            );

                                            node_scope.add_output(
                                                *output_id,
                                                imnodes::PinShape::TriangleFilled,
                                                || {
                                                    ui.set_next_item_width(130.0);

                                                    current_outputs.iter().for_each(|o| {
                                                        ui.text(o.replace(symbol, ""));
                                                    });
                                                },
                                            );
                                        }
                                    }
                                    // Empty means this attribute needs a value
                                    atlier::system::Value::Empty => {
                                        node_scope.add_input(
                                            *input_id,
                                            imnodes::PinShape::Circle,
                                            || {
                                                ui.set_next_item_width(130.0);
                                                ui.text(attribute.name());
                                                ui.set_next_item_width(130.0);
                                                ui.text("Empty");
                                            },
                                        );

                                        node_scope.add_output(
                                            *output_id,
                                            imnodes::PinShape::Triangle,
                                            || {
                                                ui.set_next_item_width(130.0);
                                                ui.text("");
                                            },
                                        );
                                    }
                                    // Reference means use the value from reference
                                    atlier::system::Value::Reference(r) => {
                                        node_scope.add_input(
                                            *input_id,
                                            imnodes::PinShape::CircleFilled,
                                            || match self.value_store.get_at(r) {
                                                Some((value, _)) => {
                                                    ui.set_next_item_width(130.0);
                                                    ui.text(format!("{}", value));
                                                }
                                                None => {}
                                            },
                                        );

                                        node_scope.attribute(*attribute_id, || {
                                            if let Some(editing) = graph.is_enabled("editing") {
                                                ui.disabled(!editing, || {
                                                    ui.set_next_item_width(130.0);
                                                    attribute.edit_ui(ui);
                                                });
                                            }
                                        });

                                        node_scope.add_output(
                                            *output_id,
                                            imnodes::PinShape::TriangleFilled,
                                            || {
                                                ui.set_next_item_width(130.0);
                                                ui.text("value");
                                            },
                                        );
                                    }
                                    _ => {
                                        node_scope.attribute(*attribute_id, || {
                                            ui.set_next_item_width(130.0);

                                            if let Some(editing) = graph.is_enabled("editing") {
                                                ui.disabled(!editing, || {
                                                    let old = attribute.clone();
                                                    attribute.edit_ui(ui);

                                                    if old != *attribute {
                                                        let new_value = attribute.value().clone();
                                                        self.value_store =
                                                            self.value_store.node(new_value.clone());
                                                    }
                                                });
                                            }
                                        });

                                        node_scope.add_output(
                                            *output_id,
                                            imnodes::PinShape::TriangleFilled,
                                            || {
                                                ui.set_next_item_width(130.0);
                                                ui.text("value");
                                            },
                                        );
                                    }
                                }

                                ui.new_line();
                            });
                        });

                    self.link_index.iter().for_each(|(link_id, link)| {
                        editor_scope.add_link(*link_id, link.end_pin, link.start_pin);
                    });
                });

                if let Some(link) = outer_scope.links_created() {
                    self.add_link(link);
                }

                if let Some(dropped) = outer_scope.get_dropped_link() {
                    self.attribute_store = Store::default();

                    if let Some(dropped_link) = self.link_index.clone().get(&dropped) {
                        let to = dropped_link.end_node;
                        if let Some(n) = self.find_node(to).cloned() {
                            if let Value::Reference(r) = &n.attribute.value() {
                                if let Some(true) = graph.is_enabled("preserve_thunk_reference_inputs") {
                                    let referenced_value = &self.value_store.clone();
                                    let referenced_value = referenced_value.get_at(r);
                                    let node = self.find_node_mut(to);
    
                                    if let (Some((v, _)), Some(n)) = (referenced_value, node) {
                                        let v = v.clone();
                                        let updating = n.attribute.value_mut();
                                        *updating = v.clone();
                                    }
                                } else if let Some(n) = self.find_node_mut(to) {
                                    let updating = n.attribute.value_mut();
                                    *updating = Value::Empty;
                                }
                            }

                            if let Value::Symbol(_) = &n.attribute.value() {
                                if let Some(n) = self.find_node_mut(to) {
                                    if let Some(values) = n.values.as_mut() {
                                        values.clear_index();
                                    }
                                }
                            }
                        }

                        let from = dropped_link.start_node;
                        if let Some(n) = self.find_node(from).cloned() {
                            if let Value::Reference(_) = &n.attribute.value() {
                                if let Some(n) = self.find_node_mut(from) {
                                    let updating = n.attribute.value_mut();
                                    *updating = Value::Empty
                                }
                            }

                            if let Value::Symbol(_) = &n.attribute.value() {
                                if let Some(n) = self.find_node_mut(from) {
                                    if let Some(values) = n.values.as_mut() {
                                        values.clear_index();
                                    }
                                }
                            }
                        }

                        self.links.remove(dropped_link);
                        self.link_index.remove(&dropped);
                    }
                }

                detatch.pop();
                ntoken.end();
            }
        }
    
        *self.as_mut() = graph;
    }
}

impl NodeEditorGraph {
    /// creates a new graph editor
    pub fn new(title: impl AsRef<str>, editor_context: EditorContext, idgen: IdentifierGenerator) -> NodeEditorGraph {
        Self {
            title: title.as_ref().to_string(),
            editor_context: Some(editor_context),
            idgen: Some(idgen),
            nodes: vec![],
            links: HashSet::new(),
            link_index: HashMap::new(),
            value_store: Store::default(),
            attribute_store: Store::default(),
            thunk_index: BTreeMap::new(),
            graph: AttributeGraph::default(),
        }
    }

    pub fn title(&self) -> String {
        self.title.to_string()
    }

    /// get's a mutable collection of nodes
    pub fn nodes_mut(&mut self) -> &mut Vec<NodeComponent> {
        &mut self.nodes
    }

    /// resolve current attributes that have a reference value into an actual value
    /// only resolves attributes that are connected
    pub fn resolve_attributes(&self) -> Vec<Attribute> {
        self.attribute_store.nodes().iter().map(|(_, n)| {
            let mut n = n.clone();
            let n = &mut n;
            match n.value() {
                Value::Reference(r) => {
                    if let Some((v, _)) = self.value_store.get_at(&r) {
                        let updating = n.value_mut();
                        *updating = v.clone();
                    }
                },
                _ => {}
            }
            n.clone()
        }).collect()
    }

    pub fn save_attribute_store(&self, id: u32) -> Attribute {
        let vec = match ron::ser::to_string_pretty(&self.attribute_store, PrettyConfig::default()) {
            Ok(vec) => vec.as_bytes().to_vec(),
            Err(_) => vec![],
        };

        Attribute::new(id, format!("file::{}_attribute_store.out", self.title()), Value::BinaryVector(vec))
    }

    pub fn load_attribute_store(&mut self, attributes: &AttributeGraph) {
        let attribute_store = attributes.find_attr_value(format!("file::{}_attribute_store.out", self.title()));
        if let Some(Value::BinaryVector(attr_store)) = attribute_store {
            match ron::de::from_bytes::<Store<(i32, Attribute)>>(attr_store) {
                Ok(_) => {
                
                }
                Err(_) => todo!(),
            }
        }
    }

    /// add's a thunk to the graph
    pub fn add_thunk(&mut self, name: impl AsRef<str>, thunk: fn(&mut AttributeGraph)) {
        self.thunk_index.insert(name.as_ref().to_string(), thunk);
    }

    /// add's a node to the graph, use an empty title to infer the title from the attribute name
    pub fn add_node(&mut self, title: impl AsRef<str>, attr: &mut Attribute, tooltip: Option<String>) {
        attr.commit();

        if let Some(idgen) = self.idgen.as_mut() {
            self.value_store = self.value_store.node(attr.value().clone());

            self.nodes.push(NodeComponent {
                title: {
                    let title = title.as_ref();
                    if title.is_empty() {
                        unique_title("node")
                    } else {
                        title.to_string()
                    }
                },
                node_id: idgen.next_node(),
                input_id: idgen.next_input_pin(),
                output_id: idgen.next_output_pin(),
                attribute_id: idgen.next_attribute(),
                attribute: attr.clone(),
                thunk: None,
                values: None,
                tooltip,
            });
        }
    }

    /// add's a link to the graph
    pub fn add_link(&mut self, link: Link) {
        if let Some(idgen) = self.idgen.as_mut() {
            if self.links.insert(link) {
                self.link_index.insert(idgen.next_link(), link);
            }
        }
    }

    /// returns true if nodes are empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// finds a specific node component with matching nodeid
    pub fn find_node(&self, id: impl Into<i32>) -> Option<&NodeComponent> {
        let id: i32 = id.into();
        self.nodes.iter().find(|n| { 
            let a: i32 = n.node_id.into();
            let b: i32 = id;
            a == b
        })
    }

    /// mutable version of find node
    pub fn find_node_mut(&mut self, id: impl Into<i32>) -> Option<&mut NodeComponent> {
        let id: i32 = id.into();
        self.nodes.iter_mut().find(|n| { 
            let a: i32 = n.node_id.into();
            let b: i32 = id;
            a == b
        })
    }

    /// update resolves reference values
    pub fn update(&mut self) {
        if let Some(true) = self.as_ref().is_enabled("pause_updating_graph") {
            return;
        }

        let mut store = Store::<NodeId>::default();

        for _ in 0..self.links.len() {
            for Link {
                start_node,
                end_node,
                ..
            } in self.links.clone().iter()
            {
                store = store.link_create_if_not_exists( start_node.clone(), end_node.clone());
            }
        }

        if let Some(last) = store.nodes().iter().last() {
            let (_, visited) = store.new_walk_mut(**last, Some(self));

            let _ = std::fs::write("node_editor.out", format!("{:?}", visited));
        }

        if let Some(true) = self.as_ref().is_enabled("editing") {
            self.attribute_store = Store::default();
        }
    }

    /// gets a mutable reference to the thunk index
    pub fn thunk_index_mut(&mut self) -> &mut BTreeMap<String, fn(&mut AttributeGraph)> {
        &mut self.thunk_index
    }

    /// returns true if the runtime editor view is open
    pub fn is_runtime_editor_open(&self) -> bool {
        self.graph.is_enabled("is_runtime_editor_open").unwrap_or(false)
    }

    pub fn show_enable_graph_resource_view(&mut self, ui: &imgui::Ui) {
        if let Some(Value::Bool(debugging)) = self.as_mut().find_attr_value_mut("editing_graph_resources") {
            ui.checkbox("Graph Resources", debugging);
        }
    }

    pub fn show_enable_runtime_editor_view(&mut self, ui: &imgui::Ui) {
        if let Some(Value::Bool(show_runtime_editor)) = self.as_mut().find_attr_value_mut("show_runtime_editor") {
            ui.checkbox("Runtime Editor", show_runtime_editor);
        }
    }

    pub fn show_enable_edit_attributes_option(&mut self, ui: &imgui::Ui) {
        if let Some(Value::Bool(readonly)) = self.as_mut().find_attr_value_mut("editing") {
            ui.checkbox("Enable attribute editing", readonly);
        }
    }

    pub fn show_preserve_thunk_reference_inputs(&mut self, ui: &imgui::Ui) {
        if let Some(Value::Bool(preserve_thunk_reference_inputs)) = self.as_mut().find_attr_value_mut("preserve_thunk_reference_inputs") {
            
        ui.checkbox("Preserve thunk reference inputs", preserve_thunk_reference_inputs);
        }
    }

    pub fn is_debugging_enabled(&mut self) -> bool {
        self.as_ref().is_enabled("editing_graph_resources").unwrap_or(false)
    }

    /// rearrange all linked nodes
    pub fn arrange_linked(&mut self) {
        let links = &mut self.links;
        let mut store = Store::<NodeId>::default();
        store.walk_unique = false;

        let coordinate_system = imnodes::CoordinateSystem::ScreenSpace;

        // This first part arranges the events horizontally
        // meanwhile, start's adding links to the above store
        for _ in 0..links.len() {
            for Link {
                start_node,
                end_node,
                ..
            } in links.clone().iter()
            {
                let ImVec2 { x, y } = start_node.get_position(coordinate_system);
                let start_x = x + 700.0;
                let start_y = y;

                end_node.set_position(start_x, start_y, coordinate_system);
                store = store.link_create_if_not_exists(start_node.clone(), end_node.clone());
            }
        }

        // This next part arranges the events that need space vertically, usually only places where events branch
        // we use the store we created above to rewalk the graph in order to figure out if we have branches
        // if we have branches, then the children of the parent need to spaced vertically.
        // if we don't have any branches, then we don't need any spacing vertically
        if let Some(last) = store.nodes().iter().last() {
            let (seen, _) =
                store.new_walk::<_, NodeEditorGraph>(**last, Some(&NodeEditorGraph::default()));

            for s in seen {
                let node = store.get(s);
                if let Some((id, refs)) = node.1 {
                    if refs.len() >= 2 {
                        // println!("vertical rearranging {:?} {:?}", id, refs);
                        for (pos, end_node) in store
                            .clone()
                            .visit(*id)
                            .iter()
                            .filter_map(|r| r.1)
                            .enumerate()
                        {
                            let ImVec2 { x: _, y } = id.get_position(coordinate_system);

                            let start_y = y + (pos as f32) * 200.0;

                            let ImVec2 { x, y: _ } = end_node.get_position(coordinate_system);
                            end_node.set_position(x, start_y, coordinate_system);
                        }
                    }
                }
            }
        }
    }

    pub fn refresh_values(&mut self) {
        self.value_store = Store::default();
    }

    pub fn arrange_vertical(&mut self) {
        self.nodes.iter_mut().enumerate().for_each(|(i, n)| {
            let spacing = i as f32;
            let spacing = spacing * 150.0;

            let ImVec2 { x, y: _ } = n
                .node_id
                .get_position(imnodes::CoordinateSystem::ScreenSpace);
            n.node_id
                .set_position(x, spacing, imnodes::CoordinateSystem::ScreenSpace);
        });
    }

    pub fn create_link(from: NodeComponent, to: NodeComponent) -> Link {
        Link {
            start_node: from.node_id,
            end_node: to.node_id,
            start_pin: from.output_id,
            end_pin: to.input_id,
            craeated_from_snap: false,
        }
    }
}

struct AttributeWalker<'a, 'ui> { ui: &'a imgui::Ui<'ui> }

impl<'a, 'ui> Visitor<(i32, Attribute)> for AttributeWalker<'a, 'ui> {
    fn visit(&self, (from_node_id, from): &(i32, Attribute), (to_node_id, to): &(i32, Attribute)) -> bool {
        let ui = self.ui;
        ui.label_text("from", format!("{} nid: {}", from.name(), from_node_id));
        ui.label_text("to", format!("{} nid: {}", to.name(), to_node_id));
        ui.new_line();
        true
    }
}

struct ValueWalker;

impl Visitor<Value> for ValueWalker {
    fn visit(&self, from: &Value, to: &Value) -> bool {
        match (&from, &to) {
            (Value::Symbol(symbol), _) => symbol == "ACTIVE",
            _ => false,
        }
    }
}

impl Visitor<NodeId> for NodeEditorGraph {
    fn visit(&self, _: &NodeId, _: &NodeId) -> bool {
        true
    }

    fn visit_mut(&mut self, f: &NodeId, t: &NodeId) -> bool {
        let mut next_store = self.attribute_store.clone();

        if let (Some(from_node), Some(to_node)) = (&self.find_node(*f), &self.find_node(*t)) {
            let from = &from_node.attribute;
            let to = &to_node.attribute;

            next_store = next_store.link_create_if_not_exists(
                (from_node.node_id.into(), from.clone()), 
                (to_node.node_id.into(), to.clone()));

            match (from.value(), to.value()) {
                // Set the value of to, to from's value
                (Value::Reference(from), Value::Reference(_)) => {
                    let from = *from;
                    if let Some(update) = self.find_node_mut(*t) {
                        let updating = update.attribute.value_mut();
                        *updating = Value::Reference(from);
                    }
                }
                (Value::Reference(from), Value::Empty) => {
                    let from = *from;
                    if let Some(update) = self.find_node_mut(*t) {
                        let updating = update.attribute.value_mut();
                        *updating = Value::Reference(from);
                    }
                }
                (Value::Symbol(_), Value::Empty) => {
                    if let Some(values) = &from_node.values {
                        let context = ThunkContext::from(values.clone());

                        if let Some((_, output)) = context.returns().first() {
                            let reference = output.to_ref();
                            if let Some(update) = self.find_node_mut(*t) {
                                let updating = update.attribute.value_mut();
                                *updating = reference;
                            }
                        }
                    }
                }
                (Value::Empty, Value::Symbol(_)) => {
                    return false;
                }
                (value, Value::Empty) => {
                    let reference = value.to_ref();
                    if let Some(update) = self.find_node_mut(*t) {
                        let updating = update.attribute.value_mut();
                        *updating = reference;
                    }
                }
                // symbol -> symbol, can share outputs
                (Value::Symbol(_), Value::Symbol(symbol)) => {
                    if symbol.starts_with("thunk::") {
                        let thunk_name = symbol[7..].to_string();
                        if let Some(thunk) = self.thunk_index.get(&thunk_name) {
                            let thunk = thunk.clone();
                            let input = from_node.clone().clone();

                            if let Some(values) = input.values {
                                let thunk_context = ThunkContext::from(values);
                                let inputs = thunk_context.outputs();

                                if let Some(update) = self.find_node_mut(*t) {
                                    for (input_name, input) in inputs {
                                        update.update(thunk, input_name, input.to_owned())
                                    }

                                    thunk_context.returns().iter().for_each(|(name, value)| {
                                        update.update(
                                            thunk,
                                            name,
                                            value.clone(),
                                        );
                                    });
                                }
                            } else {
                                return false;
                            }
                        }
                    }
                }
                (Value::Reference(reference), Value::Symbol(symbol)) => {
                    if symbol.starts_with("thunk::") {
                        let thunk = symbol[7..].to_string();
                        if let Some(thunk) = self.thunk_index.get(&thunk) {
                            let thunk = thunk.clone();
                            let input_name = format!("{}", &from.name());
                            let input = self
                                .value_store
                                .get_at(reference)
                                .and_then(|(v, _)| Some(v.clone()));

                            if let Some(input) = input {
                                if let Some(update) = self.find_node_mut(*t) {
                                    update.update(thunk, input_name, input)
                                }
                            } else {
                                return false;
                            }
                        }
                    }
                }
                (input, Value::Symbol(symbol)) => {
                    if symbol.starts_with("thunk::") {
                        let thunk = symbol[7..].to_string();
                        if let Some(thunk) = self.thunk_index.get(&thunk) {
                            let thunk = thunk.clone();
                            let input_name = format!("{}", &from.name());
                            let input = input.clone();

                            if let Some(update) = self.find_node_mut(*t) {
                                update.update(thunk, input_name, input)
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if next_store != self.attribute_store {
            self.attribute_store = next_store;
        }

        true
    }
}
