use atlier::system::{App, Attribute, Extension, Value};
use imgui::MenuItem;
use specs::{
    storage::HashMapStorage, Component, Entities, Join, ReadStorage, RunNow, System, WorldExt,
    WriteStorage,
};
use std::collections::BTreeMap;

use crate::editor::unique_title;

use super::{node_editor_graph::NodeEditorGraph, SectionAttributes};

pub struct NodeEditor {
    pub imnodes: imnodes::Context,
    pub editors: BTreeMap<u32, NodeEditorGraph>,
    thunks: BTreeMap<String, fn(&mut BTreeMap<String, Value>)>,
}

/// Stores a graph representation of attributes
#[derive(Component, Clone)]
#[storage(HashMapStorage)]
pub struct AttributeGraph(knot::store::Store<Attribute>);

impl NodeEditor {
    pub fn new() -> NodeEditor {
        NodeEditor {
            imnodes: imnodes::Context::new(),
            editors: BTreeMap::new(),
            thunks: BTreeMap::new(),
        }
    }
}

impl NodeEditor {
    pub fn add_thunk(&mut self, name: impl AsRef<str>, thunk: fn(&mut BTreeMap<String, Value>)) {
        self.thunks.insert(name.as_ref().to_string(), thunk);
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
                    Some(true) => match self.editors.get_mut(&e.id()) {
                        None => {
                            let editor_context = self.imnodes.create_editor();
                            let idgen = editor_context.new_identifier_generator();

                            let mut editor = NodeEditorGraph::new(editor_context, idgen);
                            for attr in attributes
                                .clone_attrs()
                                .iter_mut()
                                .filter(|a| a.name().starts_with("node::"))
                            {
                                editor.add_node(attr);
                            }

                            for (call, thunk) in &self.thunks {
                                editor.add_thunk(call, thunk.clone());
                            }

                            self.editors.insert(e.id(), editor);
                        }
                        Some(editor) => editor.update(),
                    },
                    Some(false) => {
                        self.editors.remove(&e.id());
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

        for (id, editor) in self.editors.iter_mut() {
            if !editor.is_empty() {
                Window::new(format!("Node editor {}", id))
                    .size([1500.0, 600.0], Condition::Appearing)
                    .menu_bar(true)
                    .build(ui, || {
                        ui.menu_bar(|| {
                            ui.menu("File", ||{
                                
                            });

                            ui.menu("Edit", || {
                                ui.menu("Attributes", ||{
                                    if MenuItem::new("Add text attribute").build(ui) {
                                        editor.add_node(&mut Attribute::new(
                                            0,
                                            unique_title("node::text"),
                                            Value::TextBuffer(String::default()),
                                        ));
                                    }

                                    if MenuItem::new("Add float attribute").build(ui) {
                                        editor.add_node(&mut Attribute::new(
                                            0,
                                            unique_title("node::float"),
                                            Value::Float(0.0),
                                        ));
                                    }

                                    if MenuItem::new("Add int attribute").build(ui) {
                                        editor.add_node(&mut Attribute::new(
                                            0,
                                            unique_title("node::int"),
                                            Value::Int(0),
                                        ));
                                    }

                                    if MenuItem::new("Add bool attribute").build(ui) {
                                        editor.add_node(&mut Attribute::new(
                                            0,
                                            unique_title("node::bool"),
                                            Value::Bool(false),
                                        ));
                                    }
                                });
                                ui.menu("Thunks", || {
                                    let index = editor.thunk_index();

                                    for (key, _) in index.clone() {
                                        if MenuItem::new(format!("Add {}", key)).build(ui) {
                                            editor.add_node(&mut Attribute::new(
                                                0,
                                                unique_title("node::"),
                                                Value::Symbol(format!("thunk::{}", key.to_string())),
                                            ));
                                        }
                                    }
                                });

                                if MenuItem::new("Add empty reference").build(ui) {
                                    editor.add_node(&mut Attribute::new(
                                        0,
                                        unique_title("node::reference"),
                                        Value::Empty,
                                    ));
                                }
                            });

                            ui.menu("Tools", || {
                                if MenuItem::new("Arrange graph").build(ui) {
                                    editor.rearrange();
                                }

                                if MenuItem::new("Arrange nodes vertically").build(ui) {
                                    editor.arrange_vertical();
                                }

                                ui.separator();
                                if MenuItem::new("Refresh values")
                                    .enabled(editor.is_debugging_enabled())
                                    .build(ui)
                                {
                                    editor.refresh_values();
                                }
                            });

                            ui.menu("Options", || {
                                editor.show_enable_debug_option(ui);
                                editor.show_enable_edit_attributes_option(ui);
                            });
                        });

                        editor.show_editor(ui);
                    });
            }
        }
    }
}
