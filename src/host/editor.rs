use imgui::Window;

use crate::{
    editor::{ProgressStatusBar, StartButton, Task},
    prelude::*, project::WorkspaceSource,
};

/// Extension trait for Host, that provides functions for opening a GUI editor,
///
pub trait Editor {
    /// Opens this host app with the runtime editor extension,
    ///
    fn open_runtime_editor(self);

    /// Opens this host app with an extension,
    ///
    fn open<E>(self, width: f64, height: f64, extension: E)
    where
        E: Extension + 'static;
}

impl Editor for Host {
    fn open_runtime_editor(mut self) {
        // Register some common components for viewing event state
        self.world_mut().register::<Task>();
        self.world_mut().register::<ProgressStatusBar>();
        self.world_mut().register::<StartButton>();
        self.world_mut().register::<Connection>();

        // Setup task list view --
        self.world_mut().exec(
            |(
                entities,
                events,
                mut tasks,
                mut progress_bars,
                mut start_buttons,
            ): (
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

        self.open(1920.0, 1080.0, RuntimeEditor::default())
    }

    fn open<E>(mut self, width: f64, height: f64, extension: E)
    where
        E: Extension + 'static,
    {
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
        );
    }
}

/// Tool for viewing and interacting with a host,
/// 
#[derive(Default)]
pub struct HostEditor {
    host: Option<Host>,
    event_status: Vec<EventStatus>,
    tick: Option<()>,
    activate: Option<Entity>,
}

impl<'a> System<'a> for HostEditor {
    type SystemData = (Events<'a>, WorkspaceSource<'a>);

    fn run(&mut self, (mut events, workspace_source): Self::SystemData) {
        self.event_status = events.scan();
        
        if let Some(_) = self.tick.take() {
            events.serialized_tick();
        }

        if let Some(event) = self.activate.take() {
            if events.activate(event) {
                event!(Level::INFO, "Event {} is activating", event.id());
            }
        }

        if self.host.is_none() {
            self.host = Some(workspace_source.new_host());
        }
    }
}

impl App for HostEditor {
    fn name() -> &'static str {
        "Lifec Editor v1"
    }

    fn edit_ui(&mut self, ui: &imgui::Ui) {
        Window::new("Event commands").build(ui, || {
            if let Some(host) = self.host.as_mut() {
                if ui.button("Print engine event graph") { 
                    host.print_engine_event_graph();
                }
            }

            if ui.button("Tick") {
                self.tick = Some(());
            }

            for inactive in self.event_status.iter().filter_map(|s| match s {
                EventStatus::Inactive(e) => Some(e),
                _ => None
            }) {
                if ui.button(format!("Start {}", inactive.id())) {
                    self.activate = Some(inactive.clone());
                }
            }
        });
    }

    fn display_ui(&self, ui: &imgui::Ui) {
        Window::new("Event Status").build(ui, || {

            for status in self.event_status.iter() {
                match status {
                    EventStatus::Scheduled(e) =>    ui.text(format!("scheduled,  event - {}", e.id())),
                    EventStatus::New(e) =>          ui.text(format!("new         event - {}", e.id())),
                    EventStatus::InProgress(e) =>   ui.text(format!("in-progress event - {}", e.id())),
                    EventStatus::Ready(e) =>        ui.text(format!("ready       event - {}", e.id())),
                    EventStatus::Completed(e) =>    ui.text(format!("completed   event - {}", e.id())),
                    EventStatus::Cancelled(e) =>    ui.text(format!("cancelled   event - {}", e.id())),
                    _ => {}
                }
            }
        });
    }
}