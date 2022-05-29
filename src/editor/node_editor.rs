use atlier::system::{App, Attribute, Extension, Value};
use imgui::{ChildWindow, MenuItem};
use specs::{
    storage::HashMapStorage, Component, Entities, Join, Read, ReadStorage, RunNow, System,
    WorldExt, WriteStorage,
};
use std::collections::BTreeMap;

use crate::{editor::unique_title, plugins::Thunk, Runtime, RuntimeState};

use super::{node_editor_graph::NodeEditorGraph, RuntimeEditor, Section, SectionAttributes};

pub struct NodeEditor<S>
where
    S: RuntimeState + Component,
{
    pub imnodes: imnodes::Context,
    pub editors: BTreeMap<u32, NodeEditorGraph>,
    sections: BTreeMap<u32, Section<S>>,
    runtime_editor: Option<RuntimeEditor<S>>,
    thunks: BTreeMap<String, fn(&mut BTreeMap<String, Value>)>,
}

/// Stores a graph representation of attributes
#[derive(Component, Clone)]
#[storage(HashMapStorage)]
pub struct AttributeGraph(knot::store::Store<Attribute>);

impl<S> NodeEditor<S>
where
    S: RuntimeState + Component,
{
    pub fn new() -> Self {
        Self {
            imnodes: imnodes::Context::new(),
            editors: BTreeMap::new(),
            sections: BTreeMap::new(),
            thunks: BTreeMap::new(),
            runtime_editor: None,
        }
    }
}

impl<S> NodeEditor<S>
where
    S: RuntimeState + Component,
{
    pub fn with_thunk<T>(&mut self)
    where
        T: Thunk,
    {
        self.add_thunk(T::symbol(), T::call);
    }

    pub fn add_thunk(&mut self, name: impl AsRef<str>, thunk: fn(&mut BTreeMap<String, Value>)) {
        self.thunks.insert(name.as_ref().to_string(), thunk);
    }
}

impl<S> Extension for NodeEditor<S>
where
    S: RuntimeState + Component,
{
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

impl<'a, S> System<'a> for NodeEditor<S>
where
    S: RuntimeState + Component,
{
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, SectionAttributes>,
        ReadStorage<'a, Section<S>>,
        WriteStorage<'a, AttributeGraph>,
        Read<'a, RuntimeEditor<S>>,
    );
    /// This system initializes a node editor when it detects
    /// the attribute "enable node editor" has been set to true
    /// It will read all the attributes in the collection with the prefix node::
    /// and initialize the node_editor state
    /// When the attribute is set to false, this system will remove those resources from this
    /// system
    fn run(
        &mut self,
        (entities, attributes, sections, _attribute_graph, runtime): Self::SystemData,
    ) {
        if let None = self.runtime_editor {
            self.runtime_editor = Some(runtime.clone());
        }

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

                            if let Some(section) = sections.get(e) {
                                self.sections.insert(e.id(), section.clone());
                            }
                        }
                        Some(editor) => editor.update(),
                    },
                    Some(false) => {
                        self.editors.remove(&e.id());
                        self.sections.remove(&e.id());
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

impl<S> App for NodeEditor<S>
where
    S: RuntimeState + Component,
{
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
                            // ui.menu("File", ||{

                            // });

                            ui.menu("View", || {
                                editor.show_enable_runtime_editor_view(ui);
                                editor.show_enable_graph_resource_view(ui);
                            });

                            ui.menu("Edit", || {
                                if MenuItem::new("Refresh values")
                                    .enabled(editor.is_debugging_enabled())
                                    .build(ui)
                                {
                                    editor.refresh_values();
                                }
                                ui.separator();

                                ui.menu("Attributes", || {
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
                                        if MenuItem::new(format!("Add {} thunk", key)).build(ui) {
                                            editor.add_node(&mut Attribute::new(
                                                0,
                                                unique_title("node::"),
                                                Value::Symbol(format!(
                                                    "thunk::{}",
                                                    key.to_string()
                                                )),
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
                                ui.menu("Arrange", || {
                                    if MenuItem::new("Connected nodes").build(ui) {
                                        editor.rearrange();
                                    }

                                    if MenuItem::new("All nodes vertically").build(ui) {
                                        editor.arrange_vertical();
                                    }
                                });

                                ui.separator();
                                ui.menu("Move editor to", || {
                                    for n in editor.nodes() {
                                        if MenuItem::new(n.title()).build(ui) {
                                            n.move_editor_to();
                                        }
                                    }
                                });
                            });

                            ui.menu("Options", || {
                                editor.show_enable_edit_attributes_option(ui);
                            });
                        });

                        if let (Some(_), Some(section)) =
                            (self.runtime_editor.as_mut(), self.sections.get_mut(id))
                        {
                            if editor.is_runtime_editor_open() {
                                ChildWindow::new("Runtime editor").size([500.0, 0.0]).build(
                                    ui,
                                    || {
                                        section.show_editor(ui);

                                        let mut overview = Runtime::<S>::default();
                                        overview.state = Some(section.state.clone());

                                        RuntimeEditor::from(overview).show_current(ui);
                                    },
                                );
                                ui.same_line();
                            }
                        }

                        editor.show_editor(ui);
                    });
            }
        }
    }
}
