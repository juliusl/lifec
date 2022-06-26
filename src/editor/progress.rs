use atlier::system::{App, Extension, WindowEvent};
use specs::storage::HashMapStorage;
use specs::{Component, WorldExt};

use crate::plugins::{EventRuntime, StatusUpdate};

#[derive(Component, Clone, Default)]
#[storage(HashMapStorage)]
pub struct Progress(pub f32, pub String);

impl App for Progress {
    fn name() -> &'static str {
        "progress"
    }

    fn edit_ui(&mut self, _: &imgui::Ui) {}

    fn display_ui(&self, ui: &imgui::Ui) {
        let Progress(progress, ..) = self;
        if *progress > 0.0 {
            imgui::ProgressBar::new(self.0)
                .overlay_text(format!("{:.4} %", self.0 * 100.0))
                .build(ui);
            ui.text(format!("{}", self.1));
        }
    }
}

impl Extension for Progress {
    fn configure_app_world(world: &mut specs::World) {
        world.register::<Progress>();
    }

    fn configure_app_systems(dispatcher: &mut specs::DispatcherBuilder) {
        dispatcher.add(EventRuntime::default(), "event_runtime", &[]);
    }

    fn on_ui(&'_ mut self, _: &specs::World, _: &'_ imgui::Ui<'_>) {
        // No-op
    }

    fn on_window_event(&'_ mut self, _: &specs::World, _: &'_ WindowEvent<'_>) {
        // No-op
    }

    fn on_run(&'_ mut self, app_world: &specs::World) {
        let mut rx = app_world.write_resource::<tokio::sync::mpsc::Receiver<StatusUpdate>>();
        let mut progress = app_world.write_storage::<Progress>();

        if let Some((entity, p, s)) = rx.try_recv().ok() {
            println!("{:?} {:.4} {}", entity, p, s);
            match progress.insert(entity, Progress(p, s)) {
                Ok(_) => {}
                Err(_) => {}
            }
        }
    }
}
