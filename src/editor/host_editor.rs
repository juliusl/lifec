use atlier::system::App;
use imgui::{ChildWindow, StyleVar, Window};
use specs::{System, Read};
use std::collections::HashSet;
use tracing::{event, Level};

use crate::{prelude::{Event, Events, Node}, host::Runner};

/// Tool for viewing and interacting with a host,
///
#[derive(Default)]
pub struct HostEditor {
    /// Current nodes,
    ///
    nodes: Vec<Node>,
    /// Available workspace operations to execute,
    ///
    workspace_operations: HashSet<(String, Event)>,
    /// Event tick rate,
    ///
    tick_rate: u64,
    /// True if the event runtime is paused,
    ///
    is_paused: bool,
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

impl<'a> System<'a> for HostEditor {
    type SystemData = Events<'a>;

    fn run(&mut self, mut events: Self::SystemData) {
        self.is_paused = !events.can_continue();

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

        // Update tick rate for events
        //
        self.tick_rate = events.tick_rate();

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
                    crate::editor::NodeCommand::Custom(name, entity) => {
                        event!(
                            Level::DEBUG,
                            "Custom command {name} received for {}",
                            entity.id()
                        );
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
        Window::new("Events")
            .size([580.0, 700.0], imgui::Condition::Appearing)
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
                }
                ui.same_line();
                ui.text(format!("tick rate: {} hz", self.tick_rate));
                ui.separator();
                ui.new_line();

                ChildWindow::new(&format!("Event Section"))
                    .size([250.0, 0.0])
                    .build(ui, || {
                        let frame_padding = ui.push_style_var(StyleVar::FramePadding([8.0, 5.0]));
                        let window_padding =
                            ui.push_style_var(StyleVar::WindowPadding([16.0, 16.0]));
                        // TODO - Add some filtering?
                        for node in self.nodes.iter_mut() {
                            node.edit_ui(ui);
                            ui.new_line();
                        }
                        frame_padding.end();
                        window_padding.end();
                    });
                ui.same_line();
            });
    }

    fn display_ui(&self, ui: &imgui::Ui) {
        Window::new("Events").build(ui, || {
            ChildWindow::new(&format!("Statuses")).build(ui, || {
                let frame_padding = ui.push_style_var(StyleVar::FramePadding([8.0, 5.0]));
                let window_padding = ui.push_style_var(StyleVar::WindowPadding([16.0, 16.0]));
                ui.text("Operations:");
                for (tag, operation) in self.workspace_operations.iter() {
                    ui.text(format!("tag: {tag}, name: {}", operation.symbol()));
                }
                frame_padding.end();
                window_padding.end();
            });
        });
    }
}

impl From<HashSet<(String, Event)>> for HostEditor {
    fn from(value: HashSet<(String, Event)>) -> Self {
        Self {
            workspace_operations: value,
            ..Default::default()
        }
    }
}
