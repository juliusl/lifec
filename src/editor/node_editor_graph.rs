use atlier::system::{App, Attribute, Value};
use imgui::*;
use imnodes::{
    editor, AttributeFlag, AttributeId, EditorContext, IdentifierGenerator, ImVec2, InputPinId,
    Link, LinkId, NodeId, OutputPinId,
};
use knot::store::{Store, Visitor};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Clone)]
pub struct NodeComponent {
    title: String,
    node_id: NodeId,
    input_id: InputPinId,
    output_id: OutputPinId,
    attribute_id: AttributeId,
    attribute: Attribute,
    thunk: Option<fn(&mut BTreeMap<String, Value>)>,
    values: Option<BTreeMap<String, Value>>,
}

impl NodeComponent {
    pub fn title(&self) -> &String {
        &self.title
    }

    pub fn move_editor_to(&mut self) {
        self.node_id.move_editor_to();
    }

    /// updates the current state of the node_component
    pub fn update(
        &mut self,
        thunk: fn(&mut BTreeMap<String, Value>),
        input_name: impl AsRef<str>,
        input: Value,
    ) {
        self.thunk = Some(thunk);

        if let None = self.values {
            self.values = Some(BTreeMap::new());
        }

        if let Some(values) = self.values.as_mut() {
            values.insert(input_name.as_ref().to_string(), input);
        }
    }
}

#[derive(Default)]
pub struct NodeEditorGraph {
    editor_context: Option<imnodes::EditorContext>,
    idgen: Option<imnodes::IdentifierGenerator>,
    nodes: Vec<NodeComponent>,
    links: HashSet<Link>,
    link_index: HashMap<LinkId, Link>,
    values: Store<Value>,
    editing: Option<bool>,
    debugging: Option<bool>,
    filtering: Option<String>,
    filtering_nodes: Option<String>,
    thunk_index: BTreeMap<String, fn(&mut BTreeMap<String, Value>)>,
}

impl App for NodeEditorGraph {
    fn name() -> &'static str {
        "Node Editor Graph"
    }

    fn show_editor(&mut self, ui: &imgui::Ui) {
        if let Some(context) = self.editor_context.as_mut() {
            if let Some(debugging) = self.debugging.as_ref() {
                if *debugging {
                    ChildWindow::new("Debugger")
                        .size([500.0, 0.0])
                        .build(ui, || {
                            // if let Some(filtering_nodes) = self.filtering_nodes.as_mut() {
                            //     ui.input_text("Filter nodes", filtering_nodes).build();
                            // }

                            if CollapsingHeader::new("Edit attributes").begin(ui) {
                                if let Some(filter) = self.filtering.as_mut() {
                                    ui.input_text("Filter attributes", filter).build();
                                    ui.new_line();
                                }

                                self.nodes
                                    .iter_mut()
                                    .filter(|n| {
                                        if let Some(filter) = &self.filtering {
                                            format!("{}", n.attribute).contains(filter)
                                        } else {
                                            false
                                        }
                                    })
                                    .for_each(|n| {
                                        ui.set_next_item_width(200.0);
                                        ui.input_text(format!("title {:?}", &n.node_id), &mut n.title).build();

                                        n.attribute.edit(ui);
                                        if let Some(values) = &n.values {
                                            ui.text("Current Values:");
                                            ui.new_line();
                                            for t in values {
                                                ui.text(format!("{:?}", t));
                                            }
                                        }
                                        if ui.button(format!("Move editor to {}", n.title)) {
                                            n.node_id.move_editor_to();
                                        }
                                        ui.new_line();
                                        ui.separator();
                                        self.values = self.values.link_create_if_not_exists(
                                            n.attribute.value().clone(),
                                            Value::Symbol("ACTIVE".to_string()),
                                        );
                                    });
                            }

                            if CollapsingHeader::new("Active values").begin(ui) {
                                let (seen, _) = self.values.new_walk_ordered(
                                    Value::Symbol("ACTIVE".to_string()),
                                    Some(&ValueWalker {}),
                                );

                                for s in seen {
                                    ui.text(s.to_string());
                                }
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
                            if let Some(filtering_nodes) = &self.filtering_nodes {
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
                            } = node_component;

                            ui.set_next_item_width(130.0);
                            editor_scope.add_node(*node_id, |mut node_scope| {
                                ui.set_next_item_width(130.0);
                                node_scope.add_titlebar(|| {
                                    ui.text(title);
                                });

                                if let atlier::system::Value::Reference(r) =
                                    attribute.clone().value()
                                {
                                    match self.values.get_at(r) {
                                        None => {
                                            let resetting = attribute.get_value_mut();
                                            *resetting = Value::Empty;
                                        }
                                        _ => {}
                                    }
                                }

                                match attribute.value() {
                                    // Empty means this attribute needs a value
                                    atlier::system::Value::Symbol(symbol) => {
                                        let output_key = format!("{}::output::", symbol);
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

                                                    if let Some(output) = values.get(&output_key) {
                                                        self.values =
                                                            self.values.node(output.clone());
                                                    }
                                                }
                                            });

                                            node_scope.add_output(
                                                *output_id,
                                                imnodes::PinShape::TriangleFilled,
                                                || {
                                                    ui.set_next_item_width(130.0);
                                                    ui.text(output_key);
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
                                    }
                                    // Reference means use the value from reference
                                    atlier::system::Value::Reference(r) => {
                                        node_scope.add_input(
                                            *input_id,
                                            imnodes::PinShape::CircleFilled,
                                            || match self.values.get_at(r) {
                                                Some((value, _)) => {
                                                    ui.set_next_item_width(130.0);
                                                    ui.text(format!("{}", value));
                                                }
                                                None => {}
                                            },
                                        );

                                        node_scope.attribute(*attribute_id, || {
                                            if let Some(editing) = self.editing {
                                                ui.disabled(!editing, || {
                                                    ui.set_next_item_width(130.0);
                                                    attribute.edit(ui);
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

                                            if let Some(editing) = self.editing {
                                                ui.disabled(!editing, || {
                                                    let old = attribute.clone();
                                                    attribute.edit(ui);

                                                    if old != *attribute {
                                                        let new_value = attribute.value().clone();
                                                        self.values =
                                                            self.values.node(new_value.clone());
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
                    if let Some(idgen) = self.idgen.as_mut() {
                        if self.links.insert(link) {
                            self.link_index.insert(idgen.next_link(), link);
                        }
                    }
                }

                if let Some(dropped) = outer_scope.get_dropped_link() {
                    if let Some(dropped_link) = self.link_index.clone().get(&dropped) {
                        let to = dropped_link.end_node;
                        if let Some(n) = self.find_node(&to).cloned() {
                            if let Value::Reference(_) = &n.attribute.value() {
                                if let Some(n) = self.find_node_mut(&to) {
                                    let updating = n.attribute.get_value_mut();
                                    *updating = Value::Empty
                                }
                            }

                            if let Value::Symbol(_) = &n.attribute.value() {
                                if let Some(n) = self.find_node_mut(&to) {
                                    if let Some(values) = n.values.as_mut() {
                                        values.clear();
                                    }
                                }
                            }
                        }

                        let from = dropped_link.start_node;
                        if let Some(n) = self.find_node(&from).cloned() {
                            if let Value::Symbol(_) = &n.attribute.value() {
                                if let Some(n) = self.find_node_mut(&from) {
                                    if let Some(values) = n.values.as_mut() {
                                        values.clear();
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
    }
}

impl NodeEditorGraph {
    /// creates a new graph editor
    pub fn new(editor_context: EditorContext, idgen: IdentifierGenerator) -> NodeEditorGraph {
        Self {
            editor_context: Some(editor_context),
            idgen: Some(idgen),
            nodes: vec![],
            links: HashSet::new(),
            link_index: HashMap::new(),
            values: Store::default(),
            debugging: Some(false),
            editing: Some(false),
            filtering: Some(String::default()),
            filtering_nodes: Some(String::default()),
            thunk_index: BTreeMap::new(),
        }
    }

    pub fn nodes(&mut self) -> &mut Vec<NodeComponent> {
        &mut self.nodes
    }

    pub fn add_thunk(&mut self, name: impl AsRef<str>, thunk: fn(&mut BTreeMap<String, Value>)) {
        self.thunk_index.insert(name.as_ref().to_string(), thunk);
    }

    /// add's a node to the graph
    pub fn add_node(&mut self, attr: &mut Attribute) {
        if let Some(idgen) = self.idgen.as_mut() {
            self.values = self.values.node(attr.value().clone());

            self.nodes.push(NodeComponent {
                title: attr.name()[6..].to_string(),
                node_id: idgen.next_node(),
                input_id: idgen.next_input_pin(),
                output_id: idgen.next_output_pin(),
                attribute_id: idgen.next_attribute(),
                attribute: attr.clone(),
                thunk: None,
                values: None,
            });
        }
    }

    /// returns true if nodes are empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// finds a specific node component with matching nodeid
    pub fn find_node(&self, id: &NodeId) -> Option<&NodeComponent> {
        self.nodes.iter().find(|n| n.node_id == *id)
    }

    /// mutable version of find node
    pub fn find_node_mut(&mut self, id: &NodeId) -> Option<&mut NodeComponent> {
        self.nodes.iter_mut().find(|n| n.node_id == *id)
    }

    /// update resolves reference values
    pub fn update(&mut self) {
        let mut store = Store::<NodeId>::default();

        for _ in 0..self.links.len() {
            for Link {
                start_node,
                end_node,
                ..
            } in self.links.clone().iter()
            {
                store = store.link_create_if_not_exists(start_node.clone(), end_node.clone());
            }
        }

        if let Some(last) = store.nodes().iter().last() {
            let (_, visited) = store.new_walk_mut(**last, Some(self));

            let _ = std::fs::write("node_editor.out", format!("{:?}", visited));
        }
    }

    pub fn thunk_index(&mut self) -> &mut BTreeMap<String, fn(&mut BTreeMap<String, Value>)> {
        &mut self.thunk_index
    }

    pub fn show_enable_debug_option(&mut self, ui: &imgui::Ui) {
        if let Some(debugging) = self.debugging.as_mut() {
            ui.checkbox("Enable debug mode", debugging);
        }
    }

    pub fn show_enable_edit_attributes_option(&mut self, ui: &imgui::Ui) {
        if let Some(readonly) = self.editing.as_mut() {
            ui.checkbox("Enable attribute editing", readonly);
        }
    }

    pub fn refresh_values(&mut self) {
        self.values = Store::default();
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

    pub fn is_debugging_enabled(&mut self) -> bool {
        self.debugging.unwrap_or(false)
    }

    /// rearrange a set of linked nodes
    pub fn rearrange(&mut self) {
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
        if let (Some(from_node), Some(to_node)) = (&self.find_node(f), &self.find_node(t)) {
            let from = &from_node.attribute;
            let to = &to_node.attribute;

            match (from.value(), to.value()) {
                // Set the value of to, to from's value
                (Value::Reference(from), Value::Reference(_)) => {
                    let from = *from;
                    if let Some(update) = self.find_node_mut(t) {
                        let updating = update.attribute.get_value_mut();
                        *updating = Value::Reference(from);
                    }
                }
                (Value::Reference(from), Value::Empty) => {
                    let from = *from;
                    if let Some(update) = self.find_node_mut(t) {
                        let updating = update.attribute.get_value_mut();
                        *updating = Value::Reference(from);
                    }
                }
                (Value::Symbol(symbol), Value::Empty) => {
                    if let Some(values) = &from_node.values {
                        let output_key = format!("{}::output::", symbol);

                        if let Some(output) = values.get(&output_key) {
                            let reference = output.to_ref();
                            if let Some(update) = self.find_node_mut(t) {
                                let updating = update.attribute.get_value_mut();
                                *updating = reference;
                            }
                        }
                    }
                }
                (value, Value::Empty) => {
                    let reference = value.to_ref();
                    if let Some(update) = self.find_node_mut(t) {
                        let updating = update.attribute.get_value_mut();
                        *updating = reference;
                    }
                }
                (Value::Symbol(other), Value::Symbol(symbol)) => {
                    if symbol.starts_with("thunk::") {
                        let thunk = symbol[7..].to_string();
                        if let Some(thunk) = self.thunk_index.get(&thunk) {
                            let thunk = thunk.clone();
                            let input_name = format!("{}", &from.name());
                            let input = from_node.clone().clone();
                            let input = input
                                .values
                                .as_ref()
                                .and_then(|a| a.get(&format!("{}::output::", other)));

                            if let Some(input) = input.clone() {
                                if let Some(update) = self.find_node_mut(t) {
                                    update.update(thunk, input_name, input.to_owned())
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
                                .values
                                .get_at(reference)
                                .and_then(|(v, _)| Some(v.clone()));

                            if let Some(input) = input {
                                if let Some(update) = self.find_node_mut(t) {
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

                            if let Some(update) = self.find_node_mut(t) {
                                update.update(thunk, input_name, input)
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        true
    }
}
