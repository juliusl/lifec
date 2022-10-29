use atlier::system::App;
use hdrhistogram::Histogram;
use imgui::{ChildWindow, SliderFlags, StyleVar, Ui, Window};
use specs::{Entities, Entity, Join, Read, ReadStorage, System};
use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;
use tokio::time::Instant;
use tracing::{event, Level};

use crate::guest::Guest;
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
    /// Histogram of tick rate,
    ///
    tick_rate: Histogram<u64>,
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
    /// Guests within the current host,
    /// 
    guests: HashMap<Entity, Guest>,
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
}

impl App for HostEditor {
    fn name() -> &'static str {
        "Lifec Host Editor"
    }

    fn edit_ui(&mut self, ui: &imgui::Ui) {
        let window_padding = ui.push_style_var(StyleVar::WindowPadding([16.0, 16.0]));
        let frame_padding = ui.push_style_var(StyleVar::FramePadding([8.0, 5.0]));
        self.events_window("Events", ui);

        if self.guests.len() > 1 {
            println!("{}", self.guests.len());
        }

        for (_, guest) in self.guests.iter() {
            let Guest { guest_host, owner } = guest;
            ui.text(format!("{}", owner.id()));
        }

        window_padding.end();
        frame_padding.end();
    }

    fn display_ui(&self, _: &imgui::Ui) {}
}

impl<'a> System<'a> for HostEditor {
    type SystemData = (
        Events<'a>,
        Read<'a, tokio::sync::watch::Sender<HostEditor>, EventRuntime>,
        Entities<'a>,
        ReadStorage<'a, Guest>,
    );

    fn run(&mut self, (mut events, watcher, entities, guests): Self::SystemData) {
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

        // Update tick rate histogram,
        //
        if !events.should_exit() && events.can_continue() {
            match self.tick_rate.record(events.tick_rate()) {
                Ok(_) => {}
                Err(err) => {
                    event!(Level::ERROR, "Error recording tick rate, {err}");
                }
            }
        }

        if self.last_refresh.elapsed().as_secs() > 1 {
            self.tick_rate.clear();
            self.last_refresh = Instant::now();
        }

        // Handle node commands
        // TODO: Can record/serialize this
        let mut mutations = HashMap::<Entity, HashMap<Entity, AttributeGraph>>::default();
        for mut node in self.nodes.drain(..) {
            if let Some(command) = node.command.take() {
                events.handle_node_command(command);
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

        // Adds guests to host's set,
        //
        for (entity, guest) in (&entities, &guests).join() {
            if !self.guests.contains_key(&entity) {
                self.guests.insert(entity, guest.clone());
                event!(Level::DEBUG, "Guest {}, added to host editor", entity.id());
            }
        }

        // Update watcher
        //
        watcher.send_if_modified(|current| {
            if current != self {
                event!(Level::DEBUG, "Refreshed");
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
            self.tick_rate.clear();
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

    /// Tools to monitor and adjust tick rate,
    ///
    fn tick_rate_tools(&mut self, ui: &Ui) {
        if self.tick_limit.is_none() {
            if ui.button("Enable rate limit") {
                self.tick_limit = Some(0);
            }
        } else if let Some(tick_limit) = self.tick_limit.as_mut() {
            ui.set_next_item_width(100.0);
            imgui::Slider::new("Rate limit (hz)", 0, 100)
                .flags(SliderFlags::ALWAYS_CLAMP)
                .build(ui, tick_limit);

            ui.same_line();
            if ui.button("Disable limit") {
                self.tick_limit.take();
            }
        }
        ui.new_line();

        imgui::PlotLines::new(
            ui,
            "Tick rate (Hz)",
            self.tick_rate
                .iter_recorded()
                .map(|v| v.value_iterated_to() as f32)
                .collect::<Vec<_>>()
                .as_slice(),
        )
        .graph_size([0.0, 75.0])
        .build();
        ui.text(format!(
            "Max: {} hz",
            self.tick_rate.value_at_percentile(50.0)
        ));
    }

    /// Performance related tools and information
    ///
    fn performance_section(&mut self, ui: &Ui) {
        self.tick_rate_tools(ui);
        ui.new_line();

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

            tab_bar.end();
        }
    }
}

impl Default for HostEditor {
    fn default() -> Self {
        Self {
            tick_rate: Histogram::<u64>::new(2).expect("should be able to create"),
            adhoc_profilers: vec![],
            is_paused: Default::default(),
            is_stopped: false,
            tick_limit: Default::default(),
            last_refresh: Instant::now(),
            tick: Default::default(),
            pause: Default::default(),
            reset: Default::default(),
            nodes: Default::default(),
            guests: Default::default(),
        }
    }
}
