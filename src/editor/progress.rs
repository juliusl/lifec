use atlier::system::{App, Extension, WindowEvent};
use specs::storage::HashMapStorage;
use specs::{Component, WorldExt};

use crate::plugins::{EventRuntime, StatusUpdate};

#[derive(Component, Clone, Default)]
#[storage(HashMapStorage)]
pub struct ProgressStatusBar(pub f32, pub String);

impl App for ProgressStatusBar {
    fn name() -> &'static str {
        "progress_status_bar"
    }

    fn edit_ui(&mut self, _: &imgui::Ui) {}

    fn display_ui(&self, ui: &imgui::Ui) {
        let ProgressStatusBar(progress, status) = self;
        if *progress > 0.0 {
            imgui::ProgressBar::new(*progress)
                .overlay_text(format!("{:.4} %", progress * 100.0))
                .build(ui);
        }

        if !status.is_empty() {
            ui.text(format!("{}", status));
        }
    }
}

impl Extension for ProgressStatusBar {
    fn configure_app_world(world: &mut specs::World) {
        world.register::<ProgressStatusBar>();
    }

    fn configure_app_systems(dispatcher: &mut specs::DispatcherBuilder) {
        dispatcher.add(EventRuntime::default(), "progress_status_bar/event_runtime", &[]);
    }

    fn on_ui(&'_ mut self, app_world: &specs::World, ui: &'_ imgui::Ui<'_>) {
        self.display_ui(ui);
        self.on_run(app_world);
    }

    fn on_window_event(&'_ mut self, _: &specs::World, _: &'_ WindowEvent<'_>) {
        // No-op
    }

    fn on_run(&'_ mut self, app_world: &specs::World) {
        let mut rx = app_world.write_resource::<tokio::sync::mpsc::Receiver<StatusUpdate>>();
        let mut progress = app_world.write_storage::<ProgressStatusBar>();

        if let Some((entity, p, s)) = rx.try_recv().ok() {
           println!("{}", s);
            match progress.insert(entity, ProgressStatusBar(p, s)) {
                Ok(_) => {}
                Err(_) => {}
            }
        }
    }
}
