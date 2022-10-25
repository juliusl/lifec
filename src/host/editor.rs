use std::collections::HashSet;

use imgui::{ChildWindow, Window, StyleVar};

use crate::{
    editor::{ProgressStatusBar, StartButton, Task},
    prelude::*,
};

/// Extension trait for Host, that provides functions for opening a GUI editor,
///
pub trait Editor {
    /// Opens this host app with the runtime editor extension,
    ///
    fn open_runtime_editor<P>(self)
    where
        P: Project;

    /// Opens this host app with an extension,
    ///
    fn open<P, E>(self, width: f64, height: f64, extension: E)
    where
        P: Project,
        E: Extension + 'static;
}

impl Editor for Host {
    fn open_runtime_editor<P>(mut self)
    where
        P: Project,
    {
        // Register some common components for viewing event state
        self.world_mut().register::<Task>();
        self.world_mut().register::<ProgressStatusBar>();
        self.world_mut().register::<StartButton>();

        // Setup task list view --
        self.world_mut().exec(
            |(entities, events, mut tasks, mut progress_bars, mut start_buttons): (
                Entities,
                ReadStorage<Event>,
                WriteStorage<Task>,
                WriteStorage<ProgressStatusBar>,
                WriteStorage<StartButton>,
            )| {
                for (entity, _) in (&entities, &events).join() {
                    tasks
                        .insert(entity, Task::default())
                        .expect("should be able to insert task");
                    progress_bars
                        .insert(entity, ProgressStatusBar::default())
                        .expect("should be able to insert progress status bar");
                    start_buttons
                        .insert(entity, StartButton::default())
                        .expect("should be able to insert start button");
                }
            },
        );
        self.world_mut().maintain();

        self.open::<P, _>(1920.0, 1080.0, RuntimeEditor::default())
    }

    fn open<P, E>(mut self, width: f64, height: f64, extension: E)
    where
        P: Project,
        E: Extension + 'static,
    {
        self.prepare::<P>();
        let builder = self.new_dispatcher_builder::<P>();

        let set = {
            let operations = self.world().system_data::<Operations>();
            HashSet::from_iter(operations.scan_root().iter().cloned())
        };

        // Consume the compiled world
        let world = self.world.take();

        // Open the window
        atlier::prelude::open_window(
            HostEditor::name(),
            width,
            height,
            HostEditor::from(set),
            extension,
            world,
            Some(builder),
        );
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

/// Tool for viewing and interacting with a host,
///
#[derive(Default)]
pub struct HostEditor {
    /// Available workspace operations to execute,
    ///
    workspace_operations: HashSet<(String, Event)>,
    /// Current event statuses of the host,
    ///
    event_status: Vec<EventStatus>,
    /// Currrent cursor state,
    ///
    cursors: Vec<Cursor>,
    /// Tick rate,
    ///
    tick_rate: u64,
    /// Command to execute a serialized tick,
    ///
    tick: Option<()>,
    /// Command to active an event,
    ///
    activate: Option<Entity>,
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
        self.event_status = events.scan();
        self.cursors = events.scan_cursors();

        if let Some(_) = self.tick.take() {
            events.serialized_tick();
        }

        if let Some(event) = self.activate.take() {
            if events.activate(event) {
                event!(Level::DEBUG, "Activating event {}", event.id());
            }
        }

        if let Some(_) = self.pause.take() {
            if events.can_continue() {
                events.pause();
            } else {
                events.resume();
            }
        }

        if let Some(_) = self.reset.take() {
            events.reset();
        }

        self.tick_rate = events.tick_rate();
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
                if ui.button("Tick") {
                    self.tick = Some(());
                }

                ui.same_line();
                if ui.button("Pause") {
                    self.pause = Some(());
                }

                ui.same_line();
                if ui.button("Reset") {
                    self.reset = Some(());
                }
                ui.same_line();
                ui.text(format!("tick rate: {} hz", self.tick_rate));
                ui.separator();

                let can_activate = self
                    .event_status
                    .iter()
                    .filter_map(|s| match s {
                        EventStatus::Inactive(e) => Some(e),
                        _ => None,
                    })
                    .collect::<Vec<_>>();

                if !can_activate.is_empty() {
                    ChildWindow::new(&format!("Event Commands"))
                        .size([200.0, 0.0])
                        .build(ui, || {
                            let frame_padding = ui.push_style_var(StyleVar::FramePadding([8.0, 5.0]));
                            let window_padding = ui.push_style_var(StyleVar::WindowPadding([16.0, 16.0]));
                            for inactive in can_activate {
                                if ui.button(format!("Start {}", inactive.id())) {
                                    self.activate = Some(inactive.clone());
                                }
                            }
                            frame_padding.end();
                            window_padding.end();
                        });
                    ui.same_line();
                }
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

                ui.new_line();
                ui.text("Events:");
                for (status, cursor) in self.event_status.iter().zip(self.cursors.iter()) {
                    ui.text(format!("{} {}", status, cursor));
                }
                frame_padding.end();
                window_padding.end();
            });
        });
    }
}
