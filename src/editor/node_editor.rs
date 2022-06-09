use super::{
    node_editor_graph::NodeEditorGraph, Loader, RuntimeEditor, Section,
};
use crate::{editor::unique_title, plugins::Thunk, RuntimeState, AttributeGraph};
use atlier::system::{App, Attribute, Extension, Value};
use imgui::{ChildWindow, MenuItem};
use specs::{
    Component, Entities, Join, Read, ReadStorage, RunNow, System, WriteStorage
};
use std::collections::{BTreeMap, HashMap};

pub struct NodeEditor<S>
where
    S: RuntimeState + Component,
{
    pub imnodes: imnodes::Context,
    pub editors: BTreeMap<u32, NodeEditorGraph>,
    sections: BTreeMap<u32, Section<S>>,
    runtime_editor: Option<RuntimeEditor<S>>,
    thunks: BTreeMap<String, fn(&mut AttributeGraph)>,
    thunk_toolips: HashMap<String, String>,
}

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
            thunk_toolips: HashMap::new(),
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
        if !T::description().is_empty() {
            self.add_thunk_tooltip(T::symbol(), T::description());
        }
    }

    pub fn add_thunk(&mut self, name: impl AsRef<str>, thunk: fn(&mut AttributeGraph)) {
        self.thunks.insert(name.as_ref().to_string(), thunk);
    }

    pub fn add_thunk_tooltip(&mut self, name: impl AsRef<str>, tooltip_content: impl AsRef<str>) {
        self.thunk_toolips.insert(name.as_ref().to_string(), tooltip_content.as_ref().to_string());
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

    fn configure_app_world(_: &mut specs::World) {
        
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
        ReadStorage<'a, AttributeGraph>,
        ReadStorage<'a, Section<S>>,
        WriteStorage<'a, Loader>,
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
        (entities, attributes, sections, mut section_loader, runtime): Self::SystemData,
    ) {
        if let None = self.runtime_editor {
            self.runtime_editor = Some(runtime.clone());
        }

        entities.join().for_each(|e| {
            if let Some(attributes) = attributes.get(e) {
                match attributes.is_enabled("enable_node_editor") {
                    Some(true) => match self.editors.get_mut(&e.id()) {
                        None => {
                            let editor_context = self.imnodes.create_editor();
                            let idgen = editor_context.new_identifier_generator();

                            let mut editor =
                                NodeEditorGraph::new(format!("{}", e.id()), editor_context, idgen);

                            let mut attributes = attributes.clone();

                            for attr in attributes.find_symbols_mut("node") {
                                editor.add_node("", attr, None);
                            }

                            for (call, thunk) in &self.thunks {
                                editor.add_thunk(call, thunk.clone());
                            }

                           editor.load_attribute_store(&attributes);

                            self.editors.insert(e.id(), editor);

                            if let Some(section) = sections.get(e) {
                                self.sections.insert(e.id(), section.clone().enable_edit_attributes());
                            }
                        }
                        Some(editor) => editor.update(),
                    },
                    Some(false) => {
                        self.editors.remove(&e.id());
                        let section = self.sections.remove(&e.id());

                        if let Some(section) = section {
                            match section_loader.insert(
                                e,
                                Loader::LoadSection(section.dispatcher().clone()),
                            ) {
                                Ok(_) => {
                                    println!("NodeEditor dispatched load section");
                                }
                                Err(err) => {
                                    eprintln!("Could not dispatch load section {}", err);
                                }
                            }
                        }
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
                                        editor.add_node(
                                            "",
                                            &mut Attribute::new(
                                                *id,
                                                unique_title("node::text"),
                                                Value::TextBuffer(String::default()),
                                            ),
                                            None,
                                        );

                                        if let Some(n) = editor.nodes_mut().iter_mut().last() {
                                            n.move_node_to_grid_center();
                                        }
                                    }

                                    if MenuItem::new("Add float attribute").build(ui) {
                                        editor.add_node(
                                            "",
                                            &mut Attribute::new(
                                                *id,
                                                unique_title("node::float"),
                                                Value::Float(0.0),
                                            ),
                                            None,
                                        );

                                        if let Some(n) = editor.nodes_mut().iter_mut().last() {
                                            n.move_node_to_grid_center();
                                        }
                                    }

                                    if MenuItem::new("Add int attribute").build(ui) {
                                        editor.add_node(
                                            "",
                                            &mut Attribute::new(
                                                *id,
                                                unique_title("node::int"),
                                                Value::Int(0),
                                            ),
                                            None,
                                        );

                                        if let Some(n) = editor.nodes_mut().iter_mut().last() {
                                            n.move_node_to_grid_center();
                                        }
                                    }

                                    if MenuItem::new("Add bool attribute").build(ui) {
                                        editor.add_node(
                                            "",
                                            &mut Attribute::new(
                                                *id,
                                                unique_title("node::bool"),
                                                Value::Bool(false),
                                            ),
                                            None,
                                        );

                                        if let Some(n) = editor.nodes_mut().iter_mut().last() {
                                            n.move_node_to_grid_center();
                                        }
                                    }
                                });
                                ui.menu("Thunks", || {
                                    let index = editor.thunk_index_mut();

                                    for (key, _) in index.clone() {
                                        if MenuItem::new(format!("Add {} thunk", key)).build(ui) {
                                            editor.add_node(
                                                "",
                                                &mut Attribute::new(
                                                    *id,
                                                    unique_title("node::"),
                                                    Value::Symbol(format!(
                                                        "thunk::{}",
                                                        key.to_string()
                                                    )),
                                                ),
                                                None,
                                            );

                                            if let Some(n) = editor.nodes_mut().iter_mut().last() {
                                                n.move_node_to_grid_center();
                                            }
                                        }

                                        if ui.is_item_hovered() {
                                            if let Some(tooltip) = self.thunk_toolips.get(&key) {
                                                ui.tooltip_text(tooltip);
                                            }
                                        }
                                    }
                                });

                                if MenuItem::new("Add empty reference").build(ui) {
                                    editor.add_node(
                                        "",
                                        &mut Attribute::new(
                                            *id,
                                            unique_title("node::reference"),
                                            Value::Empty,
                                        ),
                                        None,
                                    );

                                    if let Some(n) = editor.nodes_mut().iter_mut().last() {
                                        n.move_node_to_grid_center();
                                    }
                                }
                            });

                            ui.menu("Tools", || {
                                ui.menu("Arrange", || {
                                    if MenuItem::new("Connected nodes").build(ui) {
                                        editor.arrange_linked();
                                    }

                                    if MenuItem::new("All nodes vertically").build(ui) {
                                        editor.arrange_vertical();
                                    }
                                });

                                ui.separator();
                                ui.menu("Move editor to", || {
                                    for n in editor.nodes_mut() {
                                        if MenuItem::new(n.title()).build(ui) {
                                            n.move_editor_to();
                                        }
                                    }
                                });

                                ui.separator();
                                if MenuItem::new("Dump editor output attributes").build(ui) {
                                    println!("Outputting: {}", editor.resolve_attributes().len());
                                    editor.resolve_attributes().iter().for_each(|a| {
                                        println!("{}", a);
                                    });
                                }
                            });

                            ui.menu("Options", || {
                                editor.show_enable_edit_attributes_option(ui);
                                editor.show_preserve_thunk_reference_inputs(ui);
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

                                        if editor.is_debugging_enabled() {
                                            if ui.button("Dump runtime editor output") {
                                                section.state().iter_attributes().for_each(|a| {
                                                    println!("{}", a);
                                                });
                                            }
    
                                            if ui.button("Save attribute store") {
                                                section.attributes.copy_attribute(&editor.save_attribute_store(*id));
                                            }

                                            section.edit_attr(
                                                "Attribute Store",
                                                format!("file::{}_attribute_store.out", editor.title()),
                                                ui,
                                            );
                                        }
                                    },
                                );
                                ui.same_line();
                            }
                        }

                        editor.show_editor(ui);

                        if let Some(section) = self.sections.get_mut(id) {
                            editor.resolve_attributes().iter().for_each(|a| {
                                section.attributes.copy_attribute(a);
                            });
                        }
                    });
            }
        }
    }
}
