use atlier::system::{Extension, WindowEvent};
use specs::{World, DispatcherBuilder, Entity, Component};
use specs::storage::DenseVecStorage;

use crate::plugins::*;
use super::Call;

/// This button is to start actions that take computation time
#[derive(Component, Clone, Default)]
#[storage(DenseVecStorage)]
pub struct StartButton(pub bool, pub String, pub String, pub Option<Entity>);

impl App for StartButton {
    fn name() -> &'static str {
        "start_button"
    }

    fn edit_ui(&mut self, ui: &imgui::Ui) {
        if let Self(.., label, Some(entity)) = self {
            self.0 = ui.button(format!("{} {}", label, entity.id()));
        }
    }

    fn display_ui(&self, ui: &imgui::Ui) {
        ui.same_line();
        ui.text(&self.1);
    }
}

impl Extension for StartButton {
    fn configure_app_world(world: &mut World) {
        world.register::<StartButton>();
    }

    fn configure_app_systems(dispatcher: &mut DispatcherBuilder) {
        dispatcher.add(
            Call::default(), 
            "call_event", 
            &[]);
        dispatcher.add(
            EventRuntime::default(),
            "start_button/event_runtime",
            &["call_event"],
        );
    }

    fn on_ui(&'_ mut self, app_world: &World, ui: &'_ imgui::Ui<'_>) {
        if let Self(.., Some(entity)) = self {
            let mut components = app_world.write_component::<Self>();
            if let Some(start_button) = components.get_mut(*entity) {
                start_button.edit_ui(ui);
                start_button.display_ui(ui);
            }
        }
    }

    fn on_window_event(&'_ mut self, _: &World, _: &'_ WindowEvent<'_>) {
        // No-op
    }

    fn on_run(&'_ mut self, _: &World) {
        //todo!()
    }
}
