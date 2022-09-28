use specs::{World, DispatcherBuilder, Entity, Component, WorldExt};
use specs::storage::DenseVecStorage;
use crate::*;
use crate::plugins::*;
use super::Call;

/// This button is to start actions that take computation time
#[derive(Component, Clone, Default, Debug)]
#[storage(DenseVecStorage)]
pub struct StartButton(
    /// Pressed
    pub Option<bool>, 
    /// Status
    pub String, 
    /// Label
    pub String, 
    /// Caller
    pub Option<Entity>
);

impl Extension for StartButton {
    fn configure_app_world(world: &mut World) {
        world.register::<StartButton>();
    }

    fn configure_app_systems(dispatcher: &mut DispatcherBuilder) {
        dispatcher.add(
            Call::default(), 
            "start_button/call_event", 
            &[]);
        dispatcher.add(
            EventRuntime::default(),
            "start_button/event_runtime",
            &["start_button/call_event"],
        );
    }

    fn on_ui(&'_ mut self, app_world: &World, ui: &'_ imgui::Ui<'_>) {
        if let Self(.., Some(entity)) = self {
            let mut components = app_world.write_component::<Self>();
            if let Some(start_button) = components.get_mut(*entity) {
                if let Self(pressed, .., label, Some(entity)) = self {
                    if ui.button(format!("{} {}", label, entity.id())) {
                        *pressed = Some(true);
                     }
                }
                start_button.display_ui(ui);

                ui.same_line();
                ui.text(&self.1);
            }
        }
    }
}
