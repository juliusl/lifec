use std::collections::HashSet;

use imgui::Window;

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
        let builder = self.new_dispatcher_builder::<P>();

        // Consume the compiled world
        let world = self.world.take();

        // Open the window
        atlier::prelude::open_window(
            HostEditor::name(),
            width,
            height,
            HostEditor::default(),
            extension,
            world,
            Some(builder),
        );
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
}

impl<'a> System<'a> for HostEditor {
    type SystemData = Events<'a>;

    fn run(&mut self, mut events: Self::SystemData) {
        self.event_status = events.scan();

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

        self.tick_rate = events.tick_rate();
    }
}

impl App for HostEditor {
    fn name() -> &'static str {
        "Lifec Editor"
    }

    fn edit_ui(&mut self, ui: &imgui::Ui) {
        Window::new("Event commands").build(ui, || {
            if ui.button("Tick") {
                self.tick = Some(());
            }

            if ui.button("Pause") {
                self.pause = Some(());
            }

            for inactive in self.event_status.iter().filter_map(|s| match s {
                EventStatus::Inactive(e) => Some(e),
                _ => None,
            }) {
                if ui.button(format!("Start {}", inactive.id())) {
                    self.activate = Some(inactive.clone());
                }
            }
        });
    }

    fn display_ui(&self, ui: &imgui::Ui) {
        Window::new("Event Status").build(ui, || {
            ui.text(format!("tick rate: {} hz", self.tick_rate));

            for (tag, operation) in self.workspace_operations.iter() {
                ui.text(format!("tag: {tag}, operation: {}", operation.symbol()));
            }

            for status in self.event_status.iter() {
                match status {
                    EventStatus::Scheduled(e) => ui.text(format!("scheduled,  event - {}", e.id())),
                    EventStatus::New(e) => ui.text(format!("new         event - {}", e.id())),
                    EventStatus::InProgress(e) => {
                        ui.text(format!("in-progress event - {}", e.id()))
                    }
                    EventStatus::Ready(e) => ui.text(format!("ready       event - {}", e.id())),
                    EventStatus::Completed(e) => ui.text(format!("completed   event - {}", e.id())),
                    EventStatus::Cancelled(e) => ui.text(format!("cancelled   event - {}", e.id())),
                    _ => {}
                }
            }
        });
    }
}
