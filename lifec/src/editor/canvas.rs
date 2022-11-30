use std::{
    collections::{BTreeMap, HashMap},
    fmt::Display,
    ops::Deref,
    sync::Arc,
};

use atlier::system::Extension;
use imgui::{ChildWindow, DragDropFlags, DragDropTarget, StyleVar, TableFlags, Ui, Window, StyleColor};
use reality::{
    wire::{Interner, ResourceId},
    BlockProperties, Documentation, Value,
};
use specs::{Component, Entity, HashMapStorage, World, WorldExt};
use std::fmt::Write;
use tracing::{event, Level};

use crate::{
    appendix::Appendix,
    engine::WorkspaceCommand,
    prelude::{find_doc_interner, Event, Thunk},
    state::AttributeGraph,
};

/// Struct for building and configuring an Event component,
///
/// Normally you would declare and operation or event w/ .runmd. This extension allows you to
/// build it within this tooling.
///
#[derive(Component, Default, Clone)]
#[storage(HashMapStorage)]
pub struct Canvas {
    /// Pending workspace commands,
    ///
    commands: Vec<WorkspaceCommand>,
    /// True if the plugin tree should be opened,
    ///
    plugin_tree_opened: bool,
    /// Optionally, existing entity being edited
    ///  
    context: CanvasContext,
    /// Appendix for looking up existing names, symbols, etc
    ///
    appendix: Arc<Appendix>,
    /// Custom attributes that are being applied in a block properties collection,
    ///
    custom_attributes: HashMap<usize, BlockProperties>,
    /// Custom attributes that are being applied in a block properties collection,
    ///
    documentation: HashMap<usize, BTreeMap<String, Documentation>>,
    /// Interner to lookup strings,
    ///
    interner: Interner,
}

/// Enumeration of possible contexts this canvas can be opened w/,
///
#[derive(Default, Clone)]
pub enum CanvasContext {
    /// Canvas was opened empty,
    ///
    #[default]
    Empty,
    /// Canvas was opened to create a new runtime,
    ///
    New(Entity, String, Option<String>),
    /// Canvas was opened to edit a runtime,
    ///
    Edit(Entity),
}

impl Display for CanvasContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CanvasContext::Empty => write!(f, "Canvas"),
            CanvasContext::New(e, _, _) => write!(f, "Canvas - New##{:?}", e),
            CanvasContext::Edit(e) => write!(f, "Canvas - Editing##{:?}", e),
        }
    }
}

impl Canvas {
    /// Creates a canvas for building a new plugin sequence,
    ///
    pub fn new(world: &World) -> Canvas {
        let appendix = world
            .try_fetch::<Arc<Appendix>>()
            .and_then(|a| Some(a.deref().clone()))
            .unwrap_or_default();
        let new_entity = world.entities().create();
        Self {
            context: CanvasContext::New(new_entity, String::default(), None),
            appendix,
            ..Default::default()
        }
    }

    /// Creates a new canvas for editing an existing plugin sequence,
    ///
    /// Returns None if the existing entity does not have an Event component that can be edited,
    ///
    pub fn edit(world: &World, existing: Entity) -> Option<Canvas> {
        if let Some(event) = world.read_component::<Event>().get(existing) {
            let mut canvas = Self::new(world);
            canvas.context = CanvasContext::Edit(existing);
            // If sequence is None, that means the event has been activated. If the event is activated then it cannot be edited,
            if let Event(_, thunks, Some(sequence)) = event {
                for (thunk, entity) in thunks.iter().zip(sequence.iter_entities()) {
                    let properties = world
                        .read_component::<BlockProperties>()
                        .get(entity)
                        .cloned();

                    canvas.add_plugin(canvas.commands.len(), world, *thunk, properties);
                }
            }

            Some(canvas)
        } else {
            None
        }
    }

    /// Returns the entity that owns this Canvas component,
    ///
    pub fn entity(&self) -> Option<Entity> {
        match self.context {
            CanvasContext::Empty => None,
            CanvasContext::New(e, _, _) | CanvasContext::Edit(e) => Some(e),
        }
    }
}

/// Main entry point for the Canvas extension,
///
impl Extension for Canvas {
    fn on_ui(&'_ mut self, world: &specs::World, ui: &'_ imgui::Ui<'_>) {
        let window_padding = ui.push_style_var(StyleVar::WindowPadding([16.0, 16.0]));
        let frame_padding = ui.push_style_var(StyleVar::FramePadding([8.0, 5.0]));
        Window::new(format!("{}", self.context))
            .size([1064.0, 700.0], imgui::Condition::Appearing)
            .build(ui, || {
                match &mut self.context {
                    CanvasContext::Empty => {}
                    CanvasContext::New(e, name, tag) => {
                        ui.input_text(format!("name##{:?}", e), name).build();
                        if let Some(tag) = tag.as_mut() {
                            ui.input_text(format!("tag##{:?}", e), tag).build();
                        } else {
                            if ui.input_text(format!("tag##{:?}", e), &mut String::default()).build() {
                                *tag = Some(String::default());
                            }
                        }

                        ui.label_text("entity", format!("{:?}", e));
                        ui.new_line();
                        ui.text_wrapped("Use this widget to build a new plugin sequence that can be used in either an engine event, or adhoc operation");
                    }
                    CanvasContext::Edit(existing) => {
                        ui.label_text(
                            format!("name##{}", existing.id()),
                            self.appendix.name(&existing).unwrap_or(&String::default()),
                        );
                        ui.label_text("entity", format!("{:?}", existing));
                        ui.text("Use this widget to edit an existing event or adhoc operation");

                        ui.new_line();
                        let token = ui.push_style_color(StyleColor::Text, 
                            imgui::color::ImColor32::from_rgba(66, 150, 250, 171).to_rgba_f32s());
                        ui.text_wrapped(
                            "Note: Since the existing event/operation has already been compiled, any previous custom attributes used will not show up in the transpile preview"
                        );
                        token.end();
                    }
                }

                ui.spacing();
                ui.separator();
                if let Some(token) = ui.begin_table_with_flags("layout", 2, TableFlags::RESIZABLE) {
                    ui.table_next_row();
                    ui.table_next_column();
                    // Shows a tree of plugins that will be added,
                    self.plugins_tree(world, ui);

                    ui.table_next_column();
                    // Shows a preview of the transpiled runmd that can be copied,
                    self.preview_transpiled(ui);

                    token.end();
                }
            });

        window_padding.end();
        frame_padding.end();
    }
}

impl Canvas {
    /// Plugins tree
    ///
    fn plugins_tree(&mut self, world: &World, ui: &Ui) {
        ChildWindow::new("Plugins").build(ui, || {
            imgui::TreeNode::new("Plugins")
                .opened(self.plugin_tree_opened, imgui::Condition::Always)
                .build(ui, || {
                    let snapshot = self.commands.clone();
                    for (idx, w) in snapshot.iter().enumerate() {
                        let node = imgui::TreeNode::new(format!("{}##{idx}", w))
                            .label::<String, _>(format!("{}", w))
                            .push(ui);
                        if let Some(tooltip) = imgui::drag_drop::DragDropSource::new("REORDER")
                            .flags(DragDropFlags::SOURCE_NO_PREVIEW_TOOLTIP)
                            .begin_payload(ui, idx)
                        {
                            tooltip.end();
                        }

                        if let Some(target) = imgui::drag_drop::DragDropTarget::new(ui) {
                            self.accept_payload(world, idx, target);
                        }

                        if let Some(node) = node {
                            self.edit_plugin_properties(world, w, idx, ui);
                            node.pop();
                        }
                    }
                });
        });

        if let Some(target) = imgui::drag_drop::DragDropTarget::new(ui) {
            match target.accept_payload::<WorkspaceCommand, _>("ADD_PLUGIN", DragDropFlags::empty())
            {
                Some(result) => match result {
                    Ok(command) => {
                        let idx = self.commands.len();
                        if let WorkspaceCommand::AddPlugin(thunk) = command.data {
                            self.add_plugin(idx, world, thunk, None);
                        }
                    }
                    Err(err) => {
                        event!(Level::ERROR, "Error accepting workspace command, {err}");
                    }
                },
                None => {}
            }
        }
    }

    /// Adds a plugin to the canvas at idx,
    ///
    fn add_plugin(
        &mut self,
        idx: usize,
        world: &World,
        thunk: Thunk,
        properties: Option<BlockProperties>,
    ) {
        if let Some(interner) = find_doc_interner(thunk.0, world) {
            self.interner = self.interner.merge(&interner);
        }

        if let Some(properties) = properties {
            self.custom_attributes.insert(idx, properties);
        } else {
            let mut properties = BlockProperties::new(thunk.0);
            properties.add(thunk.0, Value::Symbol(String::default()));
            self.custom_attributes.insert(idx, properties);
        }

        self.documentation.insert(idx, Default::default());
        self.plugin_tree_opened = true;
        self.commands.push(WorkspaceCommand::AddPlugin(thunk));
    }

    /// Transpile preview,
    ///
    fn preview_transpiled(&self, ui: &Ui) {
        ChildWindow::new("Transpiled").build(ui, || {
            let mut transpiled = self.transpile_runmd();
            if ui.button(format!("Copy to clipboard##{}", self.context)) {
                ui.set_clipboard_text(&transpiled);
            }
            ui.input_text_multiline("transpiled", &mut transpiled, [0.0, 0.0])
                .read_only(true)
                .build();
        });
    }

    /// Transpiles canvas state into .runmd,
    ///
    fn transpile_runmd(&self) -> String {
        let mut transpiled = String::new();

        match &self.context {
            CanvasContext::New(_, name, Some(tag)) if !name.is_empty() => {
                writeln!(transpiled, "+ {tag} .operation {name}").ok();
            },
            CanvasContext::New(_, name, None) if !name.is_empty() => {
                writeln!(transpiled, "+ .operation {name}").ok();
            },
            _ => {

            }
        }

        for (idx, c) in self.commands.iter().enumerate() {
            match c {
                WorkspaceCommand::AddPlugin(Thunk(name, ..)) => {
                    if let Some(properties) = self.custom_attributes.get(&idx) {
                        writeln!(
                            transpiled,
                            ": .{name} {}",
                            properties
                                .property(name)
                                .unwrap_or(&reality::BlockProperty::Empty)
                                .symbol()
                                .unwrap_or(&String::default())
                        )
                        .ok();

                        for (name, value) in properties
                            .iter_properties()
                            .filter(|(n, _)| !n.ends_with("::name") && n != name)
                        {
                            if let Some(property) = properties.property(format!("{name}::name")) {
                                if let Some(symbol) = property.symbol() {
                                    writeln!(
                                        transpiled,
                                        ": {:<10} .{} {}",
                                        symbol,
                                        name,
                                        value.symbol().unwrap_or(&String::default())
                                    )
                                    .ok();
                                } else if let Some(symbols) = property.symbol_vec() {
                                    let values = value.symbol_vec().unwrap_or_default();
                                    for (idx, s) in symbols.iter().enumerate() {
                                        writeln!(
                                            transpiled,
                                            ": {:<10} .{} {}",
                                            s,
                                            name,
                                            values.get(idx).unwrap_or(&String::default())
                                        )
                                        .ok();
                                    }
                                }
                            } else {
                                if let Some(symbol) = value.symbol() {
                                    writeln!(transpiled, ": .{name} {}", symbol).ok();
                                } else if let Some(symbols) = value.symbol_vec() {
                                    for s in symbols.iter() {
                                        writeln!(transpiled, ": .{name} {}", s).ok();
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        transpiled
    }

    /// Handles applying a new custom attr to a plugin w/in the canvas,
    ///
    fn handle_apply_custom_attr(
        &'_ mut self,
        idx: usize,
        target: Thunk,
        custom_attr_id: u64,
        world: &specs::World,
    ) {
        match self.commands.get(idx) {
            Some(WorkspaceCommand::AddPlugin(dest)) if target == *dest => {
                let doc_id = ResourceId::new_with_dynamic_id::<Documentation>(custom_attr_id);
                if let Some(doc) = world.try_fetch_by_id::<Documentation>(doc_id) {
                    if doc.modifies.is_empty() && doc.reads.is_empty() && doc.is_custom_attr {
                        if let Some(properties) = self.custom_attributes.get_mut(&idx) {
                            let id = self.interner.clone().add_ident(dest.0) ^ custom_attr_id;
                            let name = self.interner.strings().get(&id).unwrap();
                            properties.add(name, Value::Symbol(String::default()));

                            if doc.name_required {
                                properties
                                    .add(format!("{name}::name"), Value::Symbol(String::default()));
                            }

                            if let Some(documentation) = self.documentation.get_mut(&idx) {
                                documentation.insert(name.to_string(), doc.deref().clone());
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Handles accepting a drag drop payload,
    ///
    /// For example, adding plugin, applying a custom attribute
    ///
    fn accept_payload(&mut self, world: &World, idx: usize, target: DragDropTarget) {
        match target.accept_payload::<WorkspaceCommand, _>("ADD_PLUGIN", DragDropFlags::empty()) {
            Some(result) => match result {
                Ok(command) => {
                    self.commands.insert(idx, command.data);
                    self.commands.remove(idx + 1);
                }
                Err(err) => {
                    event!(Level::ERROR, "Error accepting workspace command, {err}");
                }
            },
            None => {}
        }

        match target
            .accept_payload::<WorkspaceCommand, _>("APPLY_CUSTOM_ATTRIBUTE", DragDropFlags::empty())
        {
            Some(result) => match result {
                Ok(command) => {
                    if let WorkspaceCommand::ApplyCustomAttribute(target, custom_attr_id) =
                        command.data
                    {
                        self.handle_apply_custom_attr(idx, target, custom_attr_id, world);
                    }
                }
                Err(err) => {
                    event!(Level::ERROR, "Error accepting workspace command, {err}");
                }
            },
            None => {}
        }

        match target.accept_payload::<usize, _>("REORDER", DragDropFlags::empty()) {
            Some(result) => match result {
                Ok(data) => {
                    let from = data.data;
                    self.commands.swap(idx, from);
                    if let Some(swapping) = self.custom_attributes.remove(&idx) {
                        if let Some(replacing) = self.custom_attributes.insert(from, swapping) {
                            self.custom_attributes.insert(idx, replacing);
                        }
                    }

                    if let Some(swapping) = self.documentation.remove(&idx) {
                        if let Some(replacing) = self.documentation.insert(from, swapping) {
                            self.documentation.insert(idx, replacing);
                        }
                    }
                }
                Err(err) => {
                    event!(Level::ERROR, "Error accepting workspace command, {err}");
                }
            },
            None => {}
        }
    }

    /// Displays ui to edit custom attribute fields for a plugin,
    ///
    fn edit_plugin_properties(&mut self, world: &World, w: &WorkspaceCommand, idx: usize, ui: &Ui) {
        if let WorkspaceCommand::AddPlugin(Thunk(name, ..)) = w {
            if let Some(properties) = self.custom_attributes.get_mut(&idx) {
                if let Some(prop_mut) = properties.property_mut(name) {
                    prop_mut.edit(
                        move |value| {
                            AttributeGraph::edit_value(format!("{name}##{idx}"), value, None, ui)
                        },
                        |_| {},
                        || None,
                    )
                }

                let mut to_remove = vec![];
                // let mut to_add = vec![];

                for (name, property) in properties.iter_properties_mut().filter(|(n, _)| n != name)
                {
                    if let Some(doc) = self.documentation.get(&idx) {
                        property.edit(
                            move |value| {
                                AttributeGraph::edit_value(
                                    format!("{name}##{idx}"),
                                    value,
                                    None,
                                    ui,
                                );
                                if let Some(doc) = doc.get(name) {
                                    if !doc.attribute_types.is_empty() && ui.is_item_hovered() {
                                        ui.tooltip(|| {
                                            for a in doc.attribute_types.iter() {
                                                ui.text(format!("{a}:"));
                                                if let Some(comment) = doc.comments.get(a) {
                                                    ui.text(format!("{comment}"));
                                                }
                                                ui.new_line();
                                            }
                                        });
                                    }
                                }
                            },
                            move |values| {
                                imgui::ListBox::new(format!("{name}##{idx}")).build(ui, || {
                                    for (_idx, value) in values.iter_mut().enumerate() {
                                        AttributeGraph::edit_value(
                                            format!("{name}##{_idx}-{idx}"),
                                            value,
                                            None,
                                            ui,
                                        );
                                    }
                                });
                                if let Some(doc) = doc.get(name) {
                                    if !doc.attribute_types.is_empty() && ui.is_item_hovered() {
                                        ui.tooltip(|| {
                                            for a in doc.attribute_types.iter() {
                                                ui.text(format!("{a}:"));
                                                if let Some(comment) = doc.comments.get(a) {
                                                    ui.text(format!("{comment}"));
                                                }
                                                ui.new_line();
                                            }
                                        });
                                    }
                                }
                            },
                            || None,
                        );

                        ui.same_line();
                        if ui.small_button("Del") {
                            to_remove.push(name.clone());
                        }
                        // ui.same_line();
                        // if let Some(doc) = doc.get(name) {
                        //     if doc.is_list {
                        //         ui.same_line();
                        //         if ui.small_button(format!("Inc##{idx}")) {
                        //             match w {
                        //                 WorkspaceCommand::AddPlugin(thunk) => {
                        //                     to_add.push(thunk);
                        //                 },
                        //                 _ => {}
                        //             }
                        //         }
                        //     }
                        // }
                    }
                }

                for name in to_remove.drain(..) {
                    properties.remove(name);
                }

                // for t in to_add.drain(..) {
                //     self.add_plugin(self.commands.len(), world, *t, None);
                // }
            }
        }
    }
}
