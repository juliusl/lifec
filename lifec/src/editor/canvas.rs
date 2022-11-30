use std::{
    collections::{BTreeMap, HashMap},
    ops::Deref,
};

use atlier::system::Extension;
use imgui::{
    drag_drop::DragDropPayload, ChildWindow, DragDropFlags, DragDropTarget, StyleVar, TableFlags,
    Ui, Window, MouseButton,
};
use reality::{
    wire::{Interner, ResourceId},
    BlockProperties, Documentation, Value, BlockProperty,
};
use specs::World;
use std::fmt::Write;
use tracing::{event, Level};

use crate::{
    engine::WorkspaceCommand,
    prelude::{find_doc_interner, Thunk},
    state::AttributeGraph,
};

/// Struct for building an operation,
///
#[derive(Default, Clone)]
pub struct Canvas {
    /// Pending workspace commands,
    ///
    commands: Vec<WorkspaceCommand>,
    /// True if the plugin tree should be opened,
    ///
    opened: bool,
    /// Optionally, existing entity being edited
    ///  
    existing: Option<u32>,
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

/// Main entry point for the Canvas extension,
///
impl Extension for Canvas {
    fn on_ui(&'_ mut self, world: &specs::World, ui: &'_ imgui::Ui<'_>) {
        let window_padding = ui.push_style_var(StyleVar::WindowPadding([16.0, 16.0]));
        let frame_padding = ui.push_style_var(StyleVar::FramePadding([8.0, 5.0]));
        Window::new("Canvas")
            .size([1064.0, 700.0], imgui::Condition::Appearing)
            .build(ui, || {
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
                .opened(self.opened, imgui::Condition::Always)
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
                            self.edit_plugin_properties(w, idx, ui);
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
                            if let Some(interner) = find_doc_interner(thunk.0, world) {
                                self.interner = self.interner.merge(&interner);
                            }

                            let mut properties = BlockProperties::new(thunk.0);
                            properties.add(thunk.0, Value::Symbol(String::default()));
                            self.custom_attributes.insert(idx, properties);

                            self.documentation.insert(idx, Default::default());
                        }
                        self.commands.push(command.data);
                        self.opened = true;
                    }
                    Err(err) => {
                        event!(Level::ERROR, "Error accepting workspace command, {err}");
                    }
                },
                None => {}
            }
        }
    }

    /// Transpile preview,
    ///
    fn preview_transpiled(&self, ui: &Ui) {
        ChildWindow::new("Transpiled").build(ui, || {
            ui.input_text_multiline("transpiled", &mut self.transpile_runmd(), [0.0, 0.0])
                .read_only(true)
                .build();
        });
    }

    /// Transpiles canvas state into .runmd,
    ///
    fn transpile_runmd(&self) -> String {
        let mut transpiled = String::new();

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
    fn edit_plugin_properties(&mut self, w: &WorkspaceCommand, idx: usize, ui: &Ui) {
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

                for (name, property) in properties.iter_properties_mut().filter(|(n, _)| n != name) {
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
                    }
                }

                for name in to_remove.iter() {
                    properties.remove(name);
                }
            }
        }
    }
}
