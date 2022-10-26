use atlier::system::App;
use hdrhistogram::Histogram;
use imgui::{ChildWindow, SliderFlags, StyleVar, Window};
use specs::System;
use std::collections::HashSet;
use tracing::{event, Level};

use crate::prelude::{Event, Events, Node};

/// Tool for viewing and interacting with a host,
///
pub struct HostEditor {
    /// Current nodes,
    ///
    nodes: Vec<Node>,
    /// Available workspace operations to execute,
    ///
    workspace_operations: HashSet<(String, Event)>,
    /// Event tick rate, TODO: Can buffer this so that it is more stable,
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
    /// Command to execute a serialized tick,
    ///
    tick: Option<()>,
    /// Command to pause the runtime,
    ///
    pause: Option<()>,
    /// Command to reset all events,
    ///
    reset: Option<()>,
}

impl Default for HostEditor {
    fn default() -> Self {
        Self {
            tick_rate: Histogram::<u64>::new(2).expect("should be able to create"),
            is_paused: Default::default(),
            is_stopped: false,
            tick_limit: Default::default(),
            tick: Default::default(),
            pause: Default::default(),
            reset: Default::default(),
            nodes: Default::default(),
            workspace_operations: Default::default(),
        }
    }
}

impl<'a> System<'a> for HostEditor {
    type SystemData = Events<'a>;

    fn run(&mut self, mut events: Self::SystemData) {
        self.is_paused = !events.can_continue();
        self.is_stopped = events.should_exit();

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

        // Update tick rate for events
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
    }
}

impl App for HostEditor {
    fn name() -> &'static str {
        "Lifec Editor"
    }

    fn edit_ui(&mut self, ui: &imgui::Ui) {
        let window_padding = ui.push_style_var(StyleVar::WindowPadding([16.0, 16.0]));
        let frame_padding = ui.push_style_var(StyleVar::FramePadding([8.0, 5.0]));
        Window::new("Events")
            .size([1200.0, 700.0], imgui::Condition::Appearing)
            .build(ui, || {
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
                ui.separator();

                ChildWindow::new(&format!("Event Section"))
                    .size([400.0, 0.0])
                    .build(ui, || {
                        // TODO - Add some filtering?
                        for node in self.nodes.iter_mut() {
                            node.edit_ui(ui);
                            ui.new_line();
                        }
                    });

                ui.same_line();
                ChildWindow::new(&format!("Statuses")).build(ui, || {
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

                    for node in self.nodes.iter() {
                        if node.histograms(ui) {
                            ui.new_line();
                        }
                    }
                });
            });

        window_padding.end();
        frame_padding.end();
    }

    fn display_ui(&self, _: &imgui::Ui) {}
}

impl From<HashSet<(String, Event)>> for HostEditor {
    fn from(value: HashSet<(String, Event)>) -> Self {
        Self {
            workspace_operations: value,
            ..Default::default()
        }
    }
}
