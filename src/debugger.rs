use std::{
    collections::{BTreeMap, VecDeque},
    ops::Deref,
};

use atlier::system::{App, Value};
use chrono::{NaiveDateTime, Utc};
use imgui::{TableFlags, TreeNode, Ui};
use reality::{
    wire::{ControlDevice, Decoder, Encoder, FrameIndex, Protocol, ResourceId, WireObject},
    Keywords,
};
use specs::{Entity, World};
use tokio::sync::mpsc::Sender;
use tracing::{event, Level};

use crate::{
    engine::{Completion, Yielding},
    prelude::{Appendix, ErrorContext, General, Listener, NodeCommand, Plugins, StatusUpdate},
    state::AttributeGraph,
};

/// Struct for engine debugger,
///
#[derive(Default)]
pub struct Debugger {
    /// Appendix to look up metadata,
    ///
    appendix: Appendix,
    /// Map of completions,
    ///
    completions: VecDeque<((Entity, Entity), Completion)>,
    /// Status updates,
    ///
    status_updates: BTreeMap<Entity, VecDeque<StatusUpdate>>,
    /// Errors,
    ///
    errors: Vec<ErrorContext>,
    /// Number of status updates to keep,
    ///
    status_update_limits: Option<usize>,
    /// Number of completions to keep,
    ///
    completion_limits: Option<usize>,
    /// Command dispatcher,
    ///
    _command_dispatcher: Option<Sender<(NodeCommand, Option<Yielding>)>>,
    /// Update notification,
    ///
    updated: Option<()>,
    /// Encoded form,
    ///
    encoded: Option<Encoder>,
}

impl Clone for Debugger {
    fn clone(&self) -> Self {
        Self {
            appendix: self.appendix.clone(),
            completions: self.completions.clone(),
            status_updates: self.status_updates.clone(),
            errors: self.errors.clone(),
            status_update_limits: self.status_update_limits.clone(),
            completion_limits: self.completion_limits.clone(),
            _command_dispatcher: self._command_dispatcher.clone(),
            updated: self.updated.clone(),
            encoded: None,
        }
    }
}

impl App for Debugger {
    fn name() -> &'static str {
        "lifec_debugger"
    }

    fn edit_ui(&mut self, ui: &imgui::Ui) {
        imgui::ChildWindow::new("Completion Tree")
            .size([600.0, 0.0])
            .build(ui, || {
                self.completion_tree(ui);
            });

        ui.same_line();
        imgui::ChildWindow::new("Status Updates")
            .border(true)
            .size([0.0, 0.0])
            .build(ui, || {
                self.updates_log(ui);
            });
    }

    fn display_ui(&self, ui: &imgui::Ui) {
        if let Some(encoder) = self.encoded.as_ref() {
            TreeNode::new("Control Device").build(ui, || {
                if let Some(table) = ui.begin_table("Idents", 2) {
                    let control_device = ControlDevice::new(encoder.interner());
                    ui.table_next_row();
                    ui.table_next_column();
                    ui.text("Data");
                    ui.table_next_column();
                    ui.text(format!("{} frames", control_device.data.len()));

                    ui.table_next_row();
                    ui.table_next_column();
                    ui.text("Read");
                    ui.table_next_column();
                    ui.text(format!("{} frames", control_device.read.len()));

                    ui.table_next_row();
                    ui.table_next_column();
                    ui.text("Index");
                    ui.table_next_column();
                    ui.text(format!("{} frames", control_device.index.len()));

                    let mut s = encoder
                        .interner
                        .strings()
                        .iter()
                        .map(|(_, s)| s)
                        .collect::<Vec<_>>();
                    s.sort();
                    for ident in s {
                        ui.table_next_row();
                        ui.table_next_column();
                        ui.text(format!("{ident}"));

                        ui.table_next_column();
                        ui.text(format!("{} chars", ident.len()));
                    }

                    table.end();
                }
            });

            TreeNode::new("Frames").build(ui, || {
                if let Some(table) = ui.begin_table_with_flags(
                    "frames",
                    7,
                    TableFlags::BORDERS_INNER_V
                        | TableFlags::RESIZABLE
                        | TableFlags::SIZING_FIXED_FIT,
                ) {
                    ui.table_setup_column("Keyword");
                    ui.table_setup_column("Entity");
                    ui.table_setup_column("Name");
                    ui.table_setup_column("Symbol");
                    ui.table_setup_column("Frame Len");
                    ui.table_setup_column("Attribute");
                    ui.table_setup_column("Value");
                    ui.table_headers_row();

                    for frame in encoder.frames.iter() {
                        ui.table_next_row();
                        ui.table_next_column();
                        ui.text(format!("{:?}", frame.keyword()));

                        ui.table_next_column();
                        let (id, gen) = frame.parity();
                        ui.text(format!("{id}.{gen}"));

                        ui.table_next_column();
                        if let Some(name) = frame.name(&encoder.interner) {
                            ui.text(name);
                        }

                        ui.table_next_column();
                        if let Some(symbol) = frame.symbol(&encoder.interner) {
                            ui.text(symbol);
                        }

                        ui.table_next_column();
                        ui.text(format!("{}", frame.frame_len()));

                        ui.table_next_column();
                        if let Some(attribute) = frame.attribute() {
                            ui.text(format!("{}", attribute));
                        }

                        ui.table_next_column();
                        match frame.keyword() {
                            Keywords::Add | Keywords::Define => {
                                if let Some(value) =
                                    frame.read_value(&encoder.interner, &encoder.blob_device)
                                {
                                    match value {
                                        Value::Empty => {
                                            ui.text("empty");
                                        }
                                        Value::Bool(b) => {
                                            ui.text(format!("{}", b));
                                        }
                                        Value::TextBuffer(t) => {
                                            ui.text(format!("{}", t));
                                        }
                                        Value::Int(i) => {
                                            ui.text(format!("{}", i));
                                        }
                                        Value::IntPair(a, b) => {
                                            ui.text(format!("{a}, {b}"));
                                        }
                                        Value::IntRange(a, b, c) => {
                                            ui.text(format!("{a}, {b}, {c}"));
                                        }
                                        Value::Float(f) => {
                                            ui.text(format!("{f}"));
                                        }
                                        Value::FloatPair(a, b) => {
                                            ui.text(format!("{a}, {b}"));
                                        }
                                        Value::FloatRange(a, b, c) => {
                                            ui.text(format!("{a}, {b}, {c}"));
                                        }
                                        Value::BinaryVector(_) => {}
                                        Value::Reference(r) => {
                                            ui.text(format!("{}", r));
                                        }
                                        Value::Symbol(s) => {
                                            ui.text(format!("{:?}", s));
                                        }
                                        Value::Complex(c) => {
                                            ui.text(format!("{:?}", c));
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    table.end()
                }
            });
        }
    }
}

impl Debugger {
    /// Propagates updated,
    ///
    pub fn propagate_update(&mut self) -> Option<()> {
        let mut protocol = Protocol::empty();
        protocol.encoder::<Debugger>(|world, encoder| {
            self.encode(world, encoder);
        });

        self.encoded = protocol.take_encoder(Debugger::resource_id());
        self.updated.take()
    }

    /// Set the update notification,
    ///
    pub fn set_update(&mut self) {
        self.updated = Some(());
    }

    /// Returns an iterator over completions,
    ///
    pub fn completions(&self) -> impl Iterator<Item = &Completion> {
        self.completions.iter().map(|(_, c)| c)
    }

    /// Returns the status logs for an entity,
    ///
    pub fn status_log(&self, entity: Entity) -> Option<VecDeque<StatusUpdate>> {
        self.status_updates.get(&entity).cloned()
    }

    /// Returns all status logs,
    ///
    pub fn status_logs(&self) -> BTreeMap<Entity, VecDeque<StatusUpdate>> {
        self.status_updates.clone()
    }

    /// Returns the debugger's appendix,
    ///
    pub fn appendix(&self) -> &Appendix {
        &self.appendix
    }

    /// Sets the appendix,
    ///
    pub fn set_appendix(&mut self, appendix: Appendix) {
        self.appendix = appendix;
    }

    /// Displays a tree view of completion history,
    ///
    pub fn completion_tree(&self, ui: &Ui) {
        let mut groups = BTreeMap::<String, Vec<Completion>>::default();
        for (_, completion) in self.completions.iter() {
            let control_symbol = if let Some(name) = self.appendix.control_symbol(&completion.event)
            {
                name.to_string()
            } else if let Some(id) = completion.query.property("event_id").and_then(|p| p.int()) {
                self.appendix
                    .name_by_id(id as u32)
                    .and_then(|n| Some(n.to_string()))
                    .unwrap_or_default()
            } else {
                String::default()
            };

            if let Some(group) = groups.get_mut(&control_symbol) {
                group.push(completion.clone());
            } else {
                groups.insert(control_symbol, vec![completion.clone()]);
            }
        }

        for (group, completions) in groups {
            imgui::TreeNode::new(group).build(ui, || {
                for Completion {
                    timestamp,
                    event,
                    thunk,
                    control_values,
                    query,
                    returns,
                } in completions.iter()
                {
                    imgui::TreeNode::new(format!("{:?}{:?}", event, thunk))
                        .label::<String, _>(format!(
                            "{} Completion {} {}.{}",
                            timestamp,
                            self.appendix.control_symbol(&event).unwrap_or_default(),
                            self.appendix.name(&event).unwrap_or_default(),
                            self.appendix.name(thunk).unwrap_or_default()
                        ))
                        .build(ui, || {
                            ui.new_line();
                            if !control_values.is_empty() {
                                ui.text("Control Values");
                                ui.disabled(false, || {
                                    for (name, value) in control_values.iter() {
                                        AttributeGraph::edit_value(
                                            format!("{name}"),
                                            &mut value.clone(),
                                            None,
                                            ui,
                                        );
                                    }
                                });
                            }

                            ui.text(format!("Input - {}", query.name()));
                            ui.disabled(false, || {
                                for (i, (name, property)) in
                                    query.clone().iter_properties_mut().enumerate()
                                {
                                    property.edit(
                                        move |value| {
                                            AttributeGraph::edit_value(
                                                format!("{name} {i}.{}.{}", event.id(), thunk.id()),
                                                value,
                                                None,
                                                ui,
                                            )
                                        },
                                        move |values| {
                                            ui.indent();
                                            ui.group(|| {
                                                for (idx, value) in values.iter_mut().enumerate() {
                                                    AttributeGraph::edit_value(
                                                        format!(
                                                            "{name} {i}-{idx}.{}.{}",
                                                            event.id(),
                                                            thunk.id()
                                                        ),
                                                        value,
                                                        None,
                                                        ui,
                                                    );
                                                }
                                            });
                                            ui.unindent();
                                        },
                                        || None,
                                    );
                                }
                            });

                            ui.new_line();
                            if let Some(returns) = returns {
                                ui.text(format!("Output - {}", returns.name()));
                                ui.disabled(false, || {
                                    for (i, (name, property)) in
                                        returns.clone().iter_properties_mut().enumerate()
                                    {
                                        property.edit(
                                            move |value| {
                                                AttributeGraph::edit_value(
                                                    format!(
                                                        "{name} c_{i}.{}.{}",
                                                        event.id(),
                                                        thunk.id()
                                                    ),
                                                    value,
                                                    None,
                                                    ui,
                                                )
                                            },
                                            move |values| {
                                                imgui::ListBox::new(format!(
                                                    "{name} c_{i}.{}.{}",
                                                    event.id(),
                                                    thunk.id()
                                                ))
                                                .build(ui, || {
                                                    for (idx, value) in
                                                        values.iter_mut().enumerate()
                                                    {
                                                        AttributeGraph::edit_value(
                                                            format!(
                                                                "{name} c_{i}-{idx}.{}.{}",
                                                                event.id(),
                                                                thunk.id()
                                                            ),
                                                            value,
                                                            None,
                                                            ui,
                                                        );
                                                    }
                                                });
                                            },
                                            || None,
                                        );
                                    }
                                });
                            }
                            ui.new_line();
                            ui.separator();
                        });
                }
            });
        }
    }

    /// Dispalys logs in a tree format,
    ///
    pub fn updates_log(&mut self, ui: &Ui) {
        let mut logs = BTreeMap::<String, BTreeMap<Entity, &VecDeque<StatusUpdate>>>::default();

        for (e, status_updates) in self.status_updates.iter() {
            let control = self.appendix().control_symbol(e).unwrap_or_default();

            if !logs.contains_key(&control) {
                logs.insert(control.clone(), BTreeMap::default());
            }

            if let Some(updates) = logs.get_mut(&control) {
                updates.insert(*e, status_updates);
            }
        }

        for (log, status_updates) in logs {
            TreeNode::new(format!("Logs {}", log)).build(ui, || {
                for (idx, (entity, updates)) in status_updates.iter().enumerate() {
                    TreeNode::new(format!("{} {}", idx, entity.id()))
                        .label::<String, _>(format!(
                            "{}: {}",
                            entity.id(),
                            self.appendix().name(entity).unwrap_or_default()
                        ))
                        .build(ui, || {
                            let p = updates
                                .iter()
                                .map(|(_, p, _)| *p)
                                .last()
                                .unwrap_or_default();
                            if ui.small_button(format!("Copy to clipboard {}.{}", idx, entity.id()))
                            {
                                let message = updates
                                    .iter()
                                    .map(|(_, _, m)| m.to_string())
                                    .collect::<Vec<_>>()
                                    .join("\n");
                                ui.set_clipboard_text(message);
                            }
                            for (_, _, message) in updates.iter() {
                                if message.starts_with("1:") {
                                    ui.text_colored(
                                        [0.0, 0.8, 0.8, 1.0],
                                        message.trim_start_matches("1:"),
                                    );
                                } else {
                                    ui.text(message);
                                }
                            }

                            if p > 0.0 {
                                imgui::ProgressBar::new(p).build(ui);
                            }
                        });
                }
            });
        }
    }
}

impl PartialEq for Debugger {
    fn eq(&self, other: &Self) -> bool {
        self.appendix == other.appendix && self.completions == other.completions
    }
}

impl Listener for Debugger {
    fn create(world: &specs::World) -> Self {
        let command_dispatcher = world
            .system_data::<Plugins>()
            .features()
            .broker()
            .command_dispatcher();

        Self {
            appendix: world.fetch::<Appendix>().deref().clone(),
            _command_dispatcher: Some(command_dispatcher),
            ..Default::default()
        }
    }

    fn on_status_update(&mut self, status_update: &crate::prelude::StatusUpdate) {
        if !self.status_updates.contains_key(&status_update.0) {
            self.status_updates
                .insert(status_update.0, Default::default());
        }

        if let Some(status_updates) = self.status_updates.get_mut(&status_update.0) {
            if status_updates.len() > self.status_update_limits.unwrap_or(10) {
                status_updates.pop_front();
            }

            status_updates.push_back(status_update.clone());
        }

        self.set_update();
    }

    fn on_completion(&mut self, completion: crate::engine::Completion) {
        if self.completions.len() > self.completion_limits.unwrap_or(1000) {
            event!(Level::TRACE, "Discarding old results");
            self.completions.pop_front();
        }

        self.completions
            .push_back(((completion.event, completion.thunk), completion));

        self.set_update();
    }

    fn on_error_context(&mut self, error: &crate::prelude::ErrorContext) {
        self.errors.push(error.clone());

        self.set_update();
    }

    fn on_operation(&mut self, _: crate::prelude::Operation) {}

    fn on_completed_event(&mut self, _: &specs::Entity) {}
}

impl WireObject for Debugger {
    fn encode<BlobImpl>(&self, _: &specs::World, encoder: &mut reality::wire::Encoder<BlobImpl>)
    where
        BlobImpl: std::io::Read + std::io::Write + std::io::Seek + Clone + Default,
    {
        for ((event, plugin), _completion) in self.completions.iter() {
            let mut completion = encoder.start_extension("debugger", "completion");

            let [a, b] = bytemuck::cast::<i64, [i32; 2]>(_completion.timestamp.timestamp());
            completion
                .as_mut()
                .add_value("timestamp", Value::IntPair(a, b));

            completion
                .as_mut()
                .add_symbol(
                    "event",
                    self.appendix().control_symbol(event).unwrap_or_default(),
                )
                .set_parity(*event);

            completion
                .as_mut()
                .add_symbol("plugin", self.appendix().name(plugin).unwrap_or_default())
                .set_parity(*plugin);

            if !_completion.control_values.is_empty() {
                let mut control_map = completion
                    .as_mut()
                    .start_extension("completion", "control_values");
                for (name, value) in _completion.control_values.iter() {
                    control_map.as_mut().add_value(name, value.clone());
                }
            }

            {
                let mut query = completion.as_mut().start_extension("completion", "query");
                for (property, value) in _completion.query.iter_properties() {
                    match value {
                        reality::BlockProperty::Single(value) => {
                            query.as_mut().add_value(property, value.clone());
                        }
                        reality::BlockProperty::List(values) => {
                            for value in values {
                                query.as_mut().add_value(property, value.clone());
                            }
                        }
                        _ => {}
                    }
                }
            }

            if let Some(_returns) = _completion.returns.as_ref() {
                let mut returns = completion.as_mut().start_extension("completion", "returns");
                for (property, value) in _returns.iter_properties() {
                    match value {
                        reality::BlockProperty::Single(value) => {
                            returns.as_mut().add_value(property, value.clone());
                        }
                        reality::BlockProperty::List(values) => {
                            for value in values {
                                returns.as_mut().add_value(property, value.clone());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        for (entity, _status_updates) in self.status_updates.iter() {
            let name = self.appendix().name(entity).unwrap_or_default();

            let mut status_update = encoder.start_extension("status_update", name);
            status_update.set_entity(*entity);
            for (_, p, s) in _status_updates.iter() {
                status_update
                    .as_mut()
                    .add_symbol("message", s)
                    .set_parity(*entity);
                status_update
                    .as_mut()
                    .add_float("progress", *p)
                    .set_parity(*entity);
            }
        }
    }

    fn decode(
        _: &reality::wire::Protocol,
        _: &reality::wire::Interner,
        _: &std::io::Cursor<Vec<u8>>,
        _: &[reality::wire::Frame],
    ) -> Self {
        let debugger = Debugger::default();
        debugger
    }

    fn decode_v2<'a, BlobImpl>(world: &World, mut decoder: Decoder<'a, BlobImpl>) -> Self
    where
        Self: Sized,
        BlobImpl: std::io::Read + std::io::Write + std::io::Seek + Clone + Default,
    {
        let mut debugger = Debugger::default();
        while let Some(mut decoder) = decoder.decode_extension("debugger", "completion") {
            let (a, b) = decoder
                .decode_value("timestamp")
                .expect("should have a timestamp frame")
                .int_pair()
                .expect("should be a float pair");
            let timestamp = bytemuck::cast::<[i32; 2], i64>([a, b]);
            let timestamp = chrono::DateTime::<Utc>::from_utc(
                NaiveDateTime::from_timestamp_opt(timestamp, 0).expect("should be a timestamp"),
                Utc,
            );

            let event_entity = decoder
                .peek()
                .expect("should have an event frame")
                .get_entity(world, false);
            let event = decoder
                .decode_value("event")
                .expect("should have a frame for events")
                .symbol()
                .expect("should be a symbol");
            debugger.appendix.general.insert(
                event_entity.id(),
                General {
                    name: event,
                    ..Default::default()
                },
            );

            let plugin_entity = decoder
                .peek()
                .expect("should have a plugin frame")
                .get_entity(world, false);
            let plugin = decoder
                .decode_value("plugin")
                .expect("should have a frame for plugins")
                .symbol()
                .expect("should be a symbol");
            debugger.appendix.general.insert(
                plugin_entity.id(),
                General {
                    name: plugin,
                    ..Default::default()
                },
            );

            let mut control_value_map = BTreeMap::<String, Value>::default();
            if let Some(mut control_values) =
                decoder.decode_extension("completion", "control_values")
            {
                for (name, value) in control_values.decode_values() {
                    control_value_map.insert(name, value);
                }
            }

            let mut query = decoder
                .decode_extension("completion", "query")
                .expect("should have an extension for query");
            let query = query.decode_properties("query");

            let returns = decoder
                .decode_extension("completion", "returns")
                .and_then(|mut r| Some(r.decode_properties("returns")));

            debugger.on_completion(Completion {
                timestamp,
                event: event_entity,
                thunk: plugin_entity,
                control_values: control_value_map,
                query,
                returns,
            });
        }

        let status_updates = decoder.decode_namespace("status_update");
        for (_, mut status_updates) in status_updates {
            let entity = status_updates
                .peek()
                .expect("should have a frame")
                .get_entity(world, false);
            while let (Some(message), Some(progress)) = (
                status_updates.decode_value("message"),
                status_updates.decode_value("progress"),
            ) {
                let progress = progress.float().expect("should be a float");
                let message = message.symbol().expect("should be a symbol");

                debugger.on_status_update(&(entity, progress, message));
            }
        }
        debugger
    }

    fn build_index(
        _: &reality::wire::Interner,
        frames: &[reality::wire::Frame],
    ) -> reality::wire::FrameIndex {
        let mut frame_index = FrameIndex::default();
        let range = 0..frames.len();
        frame_index.insert(String::default(), vec![range]);

        frame_index
    }

    fn resource_id() -> reality::wire::ResourceId {
        ResourceId::new::<Debugger>()
    }
}
