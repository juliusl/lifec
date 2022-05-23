use atlier::system::{App, Attribute, Extension};
use imnodes::{editor, AttributeFlag, AttributeId, InputPinId, Link, LinkId, NodeId, OutputPinId};
use specs::{
    storage::HashMapStorage, Component, Entities, Join, ReadStorage, RunNow, System, WorldExt,
    WriteStorage,
};
use std::collections::{HashMap, HashSet};

use super::{node_editor_graph::NodeEditorGraph, SectionAttributes};

pub struct NodeEditor {
    pub imnodes: imnodes::Context,
    pub imnode_editors: HashMap<u32, (imnodes::EditorContext, imnodes::IdentifierGenerator)>,
    pub nodes: HashMap<u32, Vec<NodeComponent>>,
    pub links: HashMap<u32, (HashSet<Link>, HashMap<LinkId, Link>)>,
}

#[derive(Clone)]
pub struct NodeComponent {
    title: String,
    node_id: NodeId,
    input_id: InputPinId,
    output_id: OutputPinId,
    attribute_id: AttributeId,
    attribute: Attribute,
}

/// Stores a graph representation of attributes
#[derive(Component, Clone)]
#[storage(HashMapStorage)]
pub struct AttributeGraph(knot::store::Store<Attribute>);

impl NodeEditor {
    pub fn new() -> NodeEditor {
        NodeEditor {
            imnodes: imnodes::Context::new(),
            imnode_editors: HashMap::new(),
            nodes: HashMap::new(),
            links: HashMap::new(),
        }
    }
}

impl Extension for NodeEditor {
    fn extend_app_world(&mut self, world: &specs::World, ui: &imgui::Ui) {
        self.run_now(world);
        self.show_editor(ui);
    }

    fn configure_app_world(world: &mut specs::World) {
        world.register::<AttributeGraph>();
    }

    fn configure_app_systems(_: &mut specs::DispatcherBuilder) {
        // We call the system in extend_app_world directly because we need to be able to render
        // state directly from the system
    }
}

impl<'a> System<'a> for NodeEditor {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, SectionAttributes>,
        WriteStorage<'a, AttributeGraph>,
    );
    /// This system initializes a node editor when it detects
    /// the attribute "enable node editor" has been set to true
    /// It will read all the attributes in the collection with the prefix node::
    /// and initialize the node_editor state
    /// When the attribute is set to false, this system will remove those resources from this
    /// system
    fn run(&mut self, (entities, attributes, _attribute_graph): Self::SystemData) {
        entities.join().for_each(|e| {
            if let Some(attributes) = attributes.get(e) {
                match attributes.is_attr_checkbox("enable node editor") {
                    Some(true) => match self.imnode_editors.get(&e.id()) {
                        None => {
                            let editor_context = self.imnodes.create_editor();
                            let mut idgen = editor_context.new_identifier_generator();

                            let mut nodes = vec![];

                            for attr in attributes
                                .clone_attrs()
                                .iter_mut()
                                .filter(|a| a.name().starts_with("node::"))
                            {
                                nodes.push(NodeComponent {
                                    title: attr.name()[6..].to_string(),
                                    node_id: idgen.next_node(),
                                    input_id: idgen.next_input_pin(),
                                    output_id: idgen.next_output_pin(),
                                    attribute_id: idgen.next_attribute(),
                                    attribute: attr.clone(),
                                });
                            }

                            self.nodes.insert(e.id(), nodes);
                            self.imnode_editors.insert(e.id(), (editor_context, idgen));
                            self.links.insert(e.id(), (HashSet::new(), HashMap::new()));
                        }
                        _ => (),
                    },
                    Some(false) => {
                        self.nodes.remove(&e.id());
                        self.imnode_editors.remove(&e.id());
                        self.links.remove(&e.id());

                        // TODO: Save the attribute graph to storage
                        // match attributes.is_attr_checkbox("allow node editor to change state on close") {
                        //     Some(true) => {
                        //         if let (
                        //             Some(nodes),
                        //             Some(_),
                        //             Some(_)) =
                        //             (nodes, editor_context, links)
                        //         {
                        //             if let Some(attributes) = write_attributes.get_mut(e) {
                        //                 let mut_attrs = attributes.get_attrs_mut();
                        //                 for n in nodes {
                        //                     if let Some(attr) = mut_attrs.iter_mut().find(|a| a.name() == n.attribute.name()) {
                        //                         let value = attr.get_value_mut();
                        //                         *value = n.attribute.value().clone();
                        //                     } else {
                        //                         mut_attrs.push(n.attribute);
                        //                     }
                        //                 }
                        //             }
                        //         }
                        //     },
                        //     _ => {},
                        // }
                    }
                    _ => (),
                }
            }
        })
    }
}

impl App for NodeEditor {
    fn name() -> &'static str {
        "Node Editor"
    }

    fn show_editor(&mut self, ui: &imgui::Ui) {
        use imgui::Condition;
        use imgui::Window;

        for (id, (context, idgen)) in self.imnode_editors.iter_mut() {
            if let Some(nodes) = self.nodes.get_mut(id) {
                Window::new(format!("Node editor {}", id))
                    .size([1500.0, 600.0], Condition::Appearing)
                    .build(ui, || {
                        if ui.button("Rearrange") {
                            if let Some((links, _)) = self.links.get_mut(id) {
                                NodeEditorGraph::rearrange(links);
                            }
                        }

                        let detatch = context.push(AttributeFlag::EnableLinkDetachWithDragClick);

                        let outer_scope = editor(context, |mut editor_scope| {
                            editor_scope.add_mini_map(imnodes::MiniMapLocation::BottomRight);
                            nodes.iter_mut().for_each(|node_component| {
                                let NodeComponent {
                                    title,
                                    node_id,
                                    input_id,
                                    output_id,
                                    attribute_id,
                                    attribute,
                                } = node_component;

                                ui.set_next_item_width(130.0);
                                editor_scope.add_node(*node_id, |mut node_scope| {
                                    ui.set_next_item_width(130.0);
                                    node_scope.add_titlebar(|| {
                                        ui.text(title);
                                    });
                                    node_scope.attribute(*attribute_id, || {
                                        ui.set_next_item_width(130.0);
                                        attribute.edit(ui);
                                    });

                                    node_scope.add_input(
                                        *input_id,
                                        imnodes::PinShape::Circle,
                                        || {
                                            ui.set_next_item_width(130.0);
                                            ui.text("in");
                                        },
                                    );

                                    node_scope.add_output(
                                        *output_id,
                                        imnodes::PinShape::Circle,
                                        || {
                                            ui.set_next_item_width(130.0);
                                            ui.text("out");
                                        },
                                    );
                                });
                            });

                            if let Some((_, link_index)) = self.links.get(id) {
                                link_index.iter().for_each(|(link_id, link)| {
                                    editor_scope.add_link(*link_id, link.end_pin, link.start_pin);
                                });
                            }
                        });

                        if let Some(link) = outer_scope.links_created() {
                            if let Some((links, link_index)) = self.links.get_mut(id) {
                                if links.insert(link) {
                                    link_index.insert(idgen.next_link(), link);
                                }
                            }
                        }

                        if let Some(dropped) = outer_scope.get_dropped_link() {
                            if let Some((links, link_index)) = self.links.get_mut(id) {
                                if let Some(dropped_link) = link_index.get(&dropped) {
                                    links.remove(dropped_link);
                                    link_index.remove(&dropped);
                                }
                            }
                        }

                        detatch.pop();
                    });
            }
        }
    }
}
