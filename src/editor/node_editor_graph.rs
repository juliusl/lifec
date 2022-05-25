use atlier::system::{App, Attribute, Value};
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
    values: BTreeMap<String, Value>,
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
    thunk_index: BTreeMap<String, fn(&mut BTreeMap<String, Value>)>,
}

impl App for NodeEditorGraph {
    fn name() -> &'static str {
        "Node Editor Graph"
    }

    fn show_editor(&mut self, ui: &imgui::Ui) {
        if let Some(context) = self.editor_context.as_mut() {
            if ui.button("Rearrange") {
                NodeEditorGraph::rearrange(&mut self.links);
            }

            if let Some(readonly) = self.editing.as_mut() {
                ui.same_line();
                ui.checkbox("enable attribute editing", readonly);
            }

            let detatch = context.push(AttributeFlag::EnableLinkDetachWithDragClick);

            let outer_scope = editor(context, |mut editor_scope| {
                editor_scope.add_mini_map(imnodes::MiniMapLocation::BottomRight);
                self.nodes.iter_mut().for_each(|node_component| {
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

                        match attribute.value() {
                            // Empty means this attribute needs a value
                            atlier::system::Value::Symbol(symbol) => {
                                node_scope.add_input(
                                    *input_id,
                                    imnodes::PinShape::Triangle,
                                    || {
                                        ui.set_next_item_width(130.0);
                                        ui.text(attribute.name());
                                        ui.set_next_item_width(130.0);
                                        ui.label_text("symbol", symbol);
                                    },
                                );
                                if let Some(thunk) = thunk {
                                    node_scope.attribute(*attribute_id, || {
                                        ui.set_next_item_width(130.0);
                                        let call = symbol[6..].to_string();
                                        if ui.button(format!("{}", call)) {
                                            thunk(values);

                                            if let Some(output) = values.get("output") {
                                                self.values = self.values.node(output.clone());
                                            }
                                        }
                                    });

                                    node_scope.add_output(
                                        *output_id,
                                        imnodes::PinShape::TriangleFilled,
                                        || {
                                            ui.set_next_item_width(130.0);
                                            ui.text("output");
                                        },
                                    );
                                }
                            }
                            // Empty means this attribute needs a value
                            atlier::system::Value::Empty => {
                                node_scope.add_input(*input_id, imnodes::PinShape::Circle, || {
                                    ui.set_next_item_width(130.0);
                                    ui.text(attribute.name());
                                    ui.set_next_item_width(130.0);
                                    ui.text("Empty");
                                });
                            }
                            // Reference means use the value from reference
                            atlier::system::Value::Reference(r) => {
                                node_scope.add_input(
                                    *input_id,
                                    imnodes::PinShape::CircleFilled,
                                    || {
                                        if let Some((value, _)) = self.values.get_at(r) {
                                            ui.set_next_item_width(130.0);
                                            ui.text(format!("{}", value));
                                        } else {
                                            ui.set_next_item_width(130.0);
                                            ui.text("Missing value");
                                        }
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
                                                self.values = self.values.node(new_value.clone());
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
                                n.values.clear();
                            }
                        }
                    }

                    self.links.remove(dropped_link);
                    self.link_index.remove(&dropped);
                }
            }

            detatch.pop();
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
            editing: Some(false),
            thunk_index: BTreeMap::new(),
        }
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
                values: BTreeMap::new(),
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
            store.new_walk_mut(**last, Some(self));
        }
    }

    /// rearrange a set of linked nodes
    pub fn rearrange(links: &mut HashSet<Link>) {
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

impl Visitor<NodeId> for NodeEditorGraph {
    fn visit(&self, _: &NodeId, _: &NodeId) -> bool {
        true
    }

    fn visit_mut(&mut self, f: &NodeId, t: &NodeId) -> bool {
        if let (Some(from_node), Some(to_node)) = (&self.find_node(f), &self.find_node(t)) {
            let from = &from_node.attribute;
            let to = &to_node.attribute;

            match (from.value(), to.value()) {
                (Value::Symbol(_), Value::Empty) => {
                    if let Some(output) = from_node.values.get("output") {
                        let reference = output.to_ref();
                        if let Some(update) = self.find_node_mut(t) {
                            let updating = update.attribute.get_value_mut();
                            *updating = reference;
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
                (input, Value::Symbol(symbol)) => {
                    if symbol.starts_with("call::") {
                        let call = symbol[6..].to_string();
                        if let Some(thunk) = self.thunk_index.get(&call) {
                            let thunk = thunk.clone();
                            let input_name = format!("{}", &from.name());
                            let input = if let Value::Reference(reference) = input {
                                if let Some((input, _)) = self.values.get_at(reference).clone() {
                                    input.clone()
                                } else {
                                    input.clone()
                                }
                            } else {
                                input.clone()
                            };

                            if let Some(update) = self.find_node_mut(t) {
                                update.thunk = Some(thunk);
                                update.values.insert(input_name, input);
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
