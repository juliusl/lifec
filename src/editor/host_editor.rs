use atlier::system::App;
use hdrhistogram::Histogram;
use imgui::{ChildWindow, SliderFlags, StyleVar, Ui, Window};
use specs::System;
use tracing::{event, Level};

use crate::prelude::{Events, Node};

/// Tool for viewing and interacting with a host,
///
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

impl App for HostEditor {
    fn name() -> &'static str {
        "Lifec Host Editor"
    }

    fn edit_ui(&mut self, ui: &imgui::Ui) {
        let window_padding = ui.push_style_var(StyleVar::WindowPadding([16.0, 16.0]));
        let frame_padding = ui.push_style_var(StyleVar::FramePadding([8.0, 5.0]));
        Window::new("Events")
            .size([1300.0, 700.0], imgui::Condition::Appearing)
            .build(ui, || {
                // Toolbar for controlling event runtime
                self.tool_bar(ui);

                // Left-Section for viewing current events and some informatino on each,
                ui.separator();
                ChildWindow::new(&format!("Event Section"))
                    .size([400.0, 0.0])
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
        window_padding.end();
        frame_padding.end();
    }

    fn display_ui(&self, _: &imgui::Ui) {}
}

impl<'a> System<'a> for HostEditor {
    type SystemData = Events<'a>;

    fn run(&mut self, mut events: Self::SystemData) {
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

        // Handle node commands
        //
        for mut node in self.nodes.drain(..) {
            if let Some(command) = node.command.take() {
                match command {
                    crate::editor::NodeCommand::Activate(event) => {
                        if events.activate(event) {
                            event!(Level::DEBUG, "Activating event {}", event.id());
                        }
                    }
                    crate::editor::NodeCommand::Reset(event) => {
                        if events.reset(event) {
                            event!(Level::DEBUG, "Reseting event {}", event.id());
                        }
                    }
                    crate::editor::NodeCommand::Pause(event) => {
                        if events.pause_event(event) {
                            event!(Level::DEBUG, "Pausing event {}", event.id());
                        }
                    }
                    crate::editor::NodeCommand::Resume(event) => {
                        if events.resume_event(event) {
                            event!(Level::DEBUG, "Resuming event {}", event.id());
                        }
                    }
                    crate::editor::NodeCommand::Cancel(event) => {
                        if events.cancel(event) {
                            event!(Level::DEBUG, "Cancelling event {}", event.id());
                        }
                    }
                    crate::editor::NodeCommand::Custom(name, entity) => {
                        event!(
                            Level::DEBUG,
                            "Custom command {name} received for {}",
                            entity.id()
                        );
                        // TODO -- Could add some custom providers
                    }
                }
            }
        }

        // Get latest node state,
        //
        for node in events.nodes() {
            self.nodes.push(node);
        }

        // Get latest adhoc profiler state,
        //
        self.adhoc_profilers = events.adhoc_profilers();
    }
}

impl HostEditor {
    /// Toolbar for buttons and some status,
    ///
    fn tool_bar(&mut self, ui: &Ui) {
        if self.is_paused {
            if ui.button("Tick") {
                self.tick = Some(());
            }

            ui.same_line();
            if ui.button("Resume") {
                self.pause = Some(());
            }
        } else {
            if ui.button("Pause") {
                self.pause = Some(());
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
        // TODO - Add some filtering?
        for node in self.nodes.iter_mut() {
            node.edit_ui(ui);
            ui.new_line();
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

        for node in self.nodes.iter() {
            if node.histograms(ui) {
                ui.new_line();
            }
        }

        for node in self.adhoc_profilers.iter() {
            if node.histograms(ui) {
                ui.new_line();
            }
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
            tick: Default::default(),
            pause: Default::default(),
            reset: Default::default(),
            nodes: Default::default(),
        }
    }
}