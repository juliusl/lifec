use atlier::system::App;
use imgui::{ChildWindow, StyleVar, Ui, Window};
use reality::wire::{Protocol, WireObject};
use specs::{Entity, Read, System};
use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;
use tokio::time::Instant;
use tracing::{event, Level};

use crate::engine::Performance;
use crate::prelude::EventRuntime;
use crate::{
    prelude::{Events, Node},
    state::AttributeGraph,
};

use super::Profiler;

/// Tool for viewing and interacting with a host,
///
#[derive(Clone, PartialEq)]
pub struct HostEditor {
    /// Current nodes,
    ///
    nodes: Vec<Node>,
    /// Adhoc profiler nodes,
    ///
    adhoc_profilers: Vec<Node>,
    /// Timestamp of last refresh,
    ///
    last_refresh: Instant,
    /// True if the event runtime is paused,
    ///
    is_paused: bool,
    /// True if there is no more activity for the runtime to process,
    ///
    is_stopped: bool,
    /// Sets a tick limit,
    ///
    tick_limit: Option<u64>,
    /// Command to execute a serialized tick (step),
    ///
    tick: Option<()>,
    /// Command to pause any events from transitioning,
    ///
    pause: Option<()>,
    /// Command to reset state on all events,
    ///
    reset: Option<()>,
}

impl Hash for HostEditor {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.nodes.hash(state);
        self.adhoc_profilers.hash(state);
        self.last_refresh.hash(state);
        self.is_paused.hash(state);
        self.is_stopped.hash(state);
        self.tick_limit.hash(state);
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

    /// Dispatch a command to set tick limit,
    ///
    pub fn set_tick_limit(&mut self, hz: u64) {
        self.tick_limit = Some(hz);
    }

    /// Disable tick limit,
    ///
    pub fn disable_tick_limit(&mut self) {
        self.tick_limit.take();
    }

    /// Shows events window,
    ///
    pub fn events_window(&mut self, title: impl AsRef<str>, ui: &Ui) {
        Window::new(title)
            .size([1400.0, 700.0], imgui::Condition::Appearing)
            .build(ui, || {
                // Toolbar for controlling event runtime
                self.tool_bar(ui);

                // Left-Section for viewing current events and some informatino on each,
                ui.separator();
                ChildWindow::new(&format!("Event Section"))
                    .size([500.0, 0.0])
                    .always_auto_resize(true)
                    .build(ui, || {
                        // TODO: Can add additional view formats for this, ex: table view
                        self.event_list(ui);
                    });

                // Right section for viewing performance related information,
                ui.same_line();
                ChildWindow::new(&format!("Performance Section")).build(ui, || {
                    self.performance_section(ui);
                });
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
        self.events_window("Events", ui);

        window_padding.end();
        frame_padding.end();
    }

    fn display_ui(&self, _: &imgui::Ui) {}
}

impl<'a> System<'a> for HostEditor {
    type SystemData = (
        Events<'a>,
        Read<'a, tokio::sync::watch::Sender<HostEditor>, EventRuntime>,
    );

    fn run(&mut self, (mut events, watcher): Self::SystemData) {
        if self.last_refresh.elapsed().as_millis() < 16 {
            return;
        }

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

        if let Some(limit) = self.tick_limit {
            events.set_rate_limit(limit);
        } else {
            events.clear_rate_limit();
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
        for mut node in events.nodes() {
            if let Some(mutations) = mutations.remove(&node.status.entity()) {
                node.mutations = mutations;
            }

            self.nodes.push(node);
        }

        // Get latest adhoc profiler state,
        //
        self.adhoc_profilers = events.adhoc_profilers();

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

        self.last_refresh = Instant::now();
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
            if imgui::CollapsingHeader::new(format!("Engine: {title}")).build(ui) {
                for node in nodes {
                    node.edit_ui(ui);
                    ui.new_line();
                    ui.separator();
                }
            }
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
                for node in self.adhoc_profilers.iter() {
                    if node.histogram(ui, 100, &[50.0, 75.0, 90.0, 99.0]) {
                        ui.new_line();
                    }
                }
                token.end();
            }

            let tab = ui.tab_item("Debug");
            if let Some(token) = tab {
                let mut protocol = Protocol::empty();
                let profilers = self.adhoc_profilers.iter().cloned().collect::<Vec<_>>();

                protocol.encoder::<Performance>(move |w, e| {
                    for node in profilers.iter()
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

impl Default for HostEditor {
    fn default() -> Self {
        Self {
            adhoc_profilers: vec![],
            is_paused: Default::default(),
            is_stopped: false,
            tick_limit: Default::default(),
            last_refresh: Instant::now(),
            tick: Default::default(),
            pause: Default::default(),
            reset: Default::default(),
            nodes: Default::default(),
        }
    }
}
