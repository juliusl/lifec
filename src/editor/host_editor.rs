use atlier::system::App;
use imgui::{
    ChildWindow, StyleVar, TableColumnFlags, TableColumnSetup, TableFlags, TreeNodeFlags, Ui,
    Window,
};
use reality::wire::{Protocol, WireObject};
use specs::{Entity, Read, System, Write};
use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;
use tracing::{event, Level};

use crate::debugger::Debugger;
use crate::engine::Performance;
use crate::prelude::EventRuntime;
use crate::{
    prelude::{Node, State},
    state::AttributeGraph,
};

use super::{Canvas, Profiler};

/// Tool for viewing and interacting with a host,
///
#[derive(Default, Clone, PartialEq)]
pub struct HostEditor {
    /// Current nodes,
    ///
    nodes: Vec<Node>,
    /// Adhoc profiler nodes,
    ///
    adhoc_nodes: Vec<Node>,
    /// True if the event runtime is paused,
    ///
    is_paused: bool,
    /// True if there is no more activity for the runtime to process,
    ///
    is_stopped: bool,
    /// Command to execute a serialized tick (step),
    ///
    tick: Option<()>,
    /// Command to pause any events from transitioning,
    ///
    pause: Option<()>,
    /// Command to reset state on all events,
    ///
    reset: Option<()>,

    canvas: Canvas,
    /// If debugger is enabled, it will be displayed in the host editor window,
    /// 
    debugger: Option<Debugger>,
}

impl Hash for HostEditor {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.nodes.hash(state);
        self.adhoc_nodes.hash(state);
        self.is_paused.hash(state);
        self.is_stopped.hash(state);
        self.tick.hash(state);
        self.pause.hash(state);
        self.reset.hash(state);
    }
}

impl HostEditor {
    /// Dispatch a command to tick events,
    ///
    pub fn tick_events(&mut self) {
        self.tick = Some(());
    }

    /// Dispatch a command to pause events,
    ///
    pub fn pause_events(&mut self) {
        self.pause = Some(());
    }

    /// Dispatch a command to reset events,
    ///
    pub fn reset_events(&mut self) {
        self.reset = Some(());
    }

    /// Shows events window,
    ///
    pub fn events_window(&mut self, suffix: impl AsRef<str>, ui: &Ui) {
        let suffix = suffix.as_ref();
        Window::new(format!("Events {suffix}"))
            .size([1500.0, 700.0], imgui::Condition::Appearing)
            .build(ui, || {
                // Toolbar for controlling event runtime
                self.tool_bar(ui);

                ui.spacing();
                ui.separator();
                ui.group(|| {
                    ChildWindow::new(&format!("Events List {suffix}"))
                        .size([900.0, 400.0])
                        .border(true)
                        .build(ui, || {
                            self.event_list(ui);
                        });

                    ui.same_line();
                    ChildWindow::new(&format!("Performance {suffix}"))
                        .size([-1.0, 400.0])
                        .border(true)
                        .build(ui, || {
                            self.performance_section(ui);
                        });
                });

                if let Some(debugger) = self.debugger.as_ref() {
                    ChildWindow::new(&format!("Debugger {suffix}"))
                        .size([0.0, -1.0])
                        .border(true)
                        .build(ui, || {
                            debugger.display_ui(ui);
                        });
                }
            });
    }

    /// Takes nodes from the host editor,
    ///
    pub fn take_nodes(&mut self) -> Vec<Node> {
        self.nodes.drain(..).collect()
    }
}

impl App for HostEditor {
    fn name() -> &'static str {
        "Lifec Host Editor"
    }

    fn edit_ui(&mut self, ui: &imgui::Ui) {
        let window_padding = ui.push_style_var(StyleVar::WindowPadding([16.0, 16.0]));
        let frame_padding = ui.push_style_var(StyleVar::FramePadding([8.0, 5.0]));
        self.events_window("", ui);
        // self.canvas.edit_ui(ui);
        window_padding.end();
        frame_padding.end();
    }

    fn display_ui(&self, _: &imgui::Ui) {}
}

impl<'a> System<'a> for HostEditor {
    type SystemData = (
        State<'a>,
        Read<'a, tokio::sync::watch::Sender<HostEditor>, EventRuntime>,
        Write<'a, Option<Debugger>>,
    );

    fn run(&mut self, (mut events, watcher, debugger): Self::SystemData) {
        self.debugger = debugger.clone();

        // General event runtime state
        self.is_paused = !events.can_continue();
        self.is_stopped = events.should_exit();

        // Handle commands from window
        //
        if let Some(_) = self.tick.take() {
            events.serialized_tick();
        }

        if let Some(_) = self.pause.take() {
            if events.can_continue() {
                events.pause();
            } else {
                events.resume();
            }
        }

        if let Some(_) = self.reset.take() {
            events.reset_all();
        }

        // Handle node commands
        let mut mutations = HashMap::<Entity, HashMap<Entity, AttributeGraph>>::default();
        for mut node in self.take_nodes() {
            if let Some(command) = node.command.take() {
                match events
                    .plugins()
                    .features()
                    .broker()
                    .try_send_node_command(command.clone(), None)
                {
                    Ok(_) => {
                        event!(Level::DEBUG, "Sent node command {:?}", command);
                    }
                    Err(err) => {
                        event!(
                            Level::ERROR,
                            "Could not send node command {err}, {:?}",
                            command
                        );
                    }
                }
            }

            if node.mutations.len() > 0 {
                mutations.insert(node.status.entity(), node.mutations);
            }
        }

        // Get latest node state,
        //
        for mut node in events.event_nodes() {
            if let Some(mutations) = mutations.remove(&node.status.entity()) {
                node.mutations = mutations;
            }

            self.nodes.push(node);
        }

        // Get latest adhoc profiler state,
        //
        self.adhoc_nodes = events.adhoc_nodes();

        // Update watcher
        //
        watcher.send_if_modified(|current| {
            if current != self {
                *current = self.clone();
                true
            } else {
                false
            }
        });
    }
}

impl HostEditor {
    /// Toolbar for buttons and some status,
    ///
    fn tool_bar(&mut self, ui: &Ui) {
        if self.is_paused {
            if ui.button("Tick") {
                self.tick_events();
            }

            ui.same_line();
            if ui.button("Resume") {
                self.pause_events();
            }
        } else {
            if ui.button("Pause") {
                self.pause_events();
            }
        }

        ui.same_line();
        if ui.button("Reset All") {
            self.reset = Some(());
        }

        if self.is_stopped {
            ui.same_line();
            ui.text("Inactive (No more events to process)");
        }
    }

    /// Event nodes in list format,
    ///
    fn event_list(&mut self, ui: &Ui) {
        let mut events = BTreeMap::<String, Vec<&mut Node>>::default();

        for node in self.nodes.iter_mut() {
            let control_symbol = node.control_symbol();

            if !control_symbol.is_empty() {
                if let Some(coll) = events.get_mut(&control_symbol) {
                    coll.push(node);
                } else {
                    events.insert(control_symbol, vec![node]);
                }
            } else {
                if let Some(coll) = events.get_mut("Adhoc Operations") {
                    coll.push(node);
                } else {
                    events.insert(String::from("Adhoc Operations"), vec![node]);
                }
            }
        }

        for (title, nodes) in events {
            let tree_flags = TreeNodeFlags::SPAN_FULL_WIDTH
                | TreeNodeFlags::FRAME_PADDING
                | TreeNodeFlags::NO_TREE_PUSH_ON_OPEN;

            imgui::TreeNode::new(format!("Engine: {title}"))
                .flags(tree_flags)
                .build(ui, || {
                    /// Name column definition
                    ///
                    fn name_column(ui: &Ui) {
                        let mut table_column_setup = TableColumnSetup::new("Name");
                        table_column_setup.flags =
                            TableColumnFlags::NO_HIDE;
                        ui.table_setup_column_with(table_column_setup);
                    }

                    /// Property column definition
                    ///
                    fn property_column(name: &'static str, ui: &Ui) {
                        let mut table_column_setup = TableColumnSetup::new(name);
                        table_column_setup.flags = TableColumnFlags::DEFAULT_HIDE;
                        ui.table_setup_column_with(table_column_setup);
                    }

                    /// Controls column definition
                    ///
                    fn controls_column(ui: &Ui) {
                        let mut table_column_setup = TableColumnSetup::new("Controls");
                        table_column_setup.flags =
                            TableColumnFlags::NO_CLIP | TableColumnFlags::WIDTH_STRETCH;
                        ui.table_setup_column_with(table_column_setup);
                    }

                    let table_flags = TableFlags::BORDERS_INNER_V
                        | TableFlags::RESIZABLE
                        | TableFlags::SIZING_FIXED_FIT
                        | TableFlags::HIDEABLE;

                    if let Some(_) = ui.begin_table_with_flags("", 5, table_flags) {
                        name_column(ui);
                        property_column("Status", ui);
                        property_column("Transition", ui);
                        property_column("Cursor", ui);
                        controls_column(ui);
                        ui.table_headers_row();

                        for node in nodes {
                            ui.table_next_row();
                            ui.table_next_column();
                            node.edit_ui(ui);
                        }
                    }
                });
            ui.spacing();
            ui.separator();
        }
    }

    /// Performance related tools and information
    ///
    fn performance_section(&mut self, ui: &Ui) {
        ui.text("Performance");
        ui.spacing();
        if let Some(tab_bar) = ui.tab_bar("Performance Tabs") {
            let tab = ui.tab_item("Engine events");
            if ui.is_item_hovered() {
                ui.tooltip_text("Performance histograms of event transitions");
            }
            if let Some(token) = tab {
                // This is the performance of engine operation events
                for node in self.nodes.iter() {
                    // TODO: Make these configurable
                    if node.histogram(ui, 100, &[50.0, 75.0, 90.0, 99.0]) {
                        ui.new_line();
                    }
                }

                token.end();
            }

            let tab = ui.tab_item("Operations");
            if ui.is_item_hovered() {
                ui.tooltip_text("Performance histograms of adhoc operation execution");
            }
            if let Some(token) = tab {
                // This is the performance of adhoc operation events
                for node in self.adhoc_nodes.iter() {
                    if node.histogram(ui, 100, &[50.0, 75.0, 90.0, 99.0]) {
                        ui.new_line();
                    }
                }
                token.end();
            }

            let tab = ui.tab_item("Debug");
            if let Some(token) = tab {
                let mut protocol = Protocol::empty();
                let profilers = self.adhoc_nodes.iter().cloned().collect::<Vec<_>>();

                protocol.encoder::<Performance>(move |w, e| {
                    for node in profilers
                        .iter()
                        .filter_map(|p| p.connection.clone())
                        .map(|p| Performance::samples(100, &[50.0, 60.0, 90.0, 99.0], &p))
                        .flatten()
                    {
                        e.encode(&node, w);
                    }

                    e.frame_index = Performance::build_index(&e.interner, &e.frames);
                });

                for f in protocol.decode::<Performance>() {
                    ui.text(format!("{:#?}", f));
                }

                token.end();
            }

            tab_bar.end();
        }
    }
}
