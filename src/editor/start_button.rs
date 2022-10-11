use crate::*;
use specs::storage::DenseVecStorage;
use specs::{Component, Entity, World, WorldExt};

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
    pub Option<Entity>,
);

impl Extension for StartButton {
    fn configure_app_world(world: &mut World) {
        world.register::<StartButton>();
    }

    fn on_ui(&'_ mut self, app_world: &World, ui: &'_ imgui::Ui<'_>) {
        if let Self(.., Some(entity)) = self {
            let mut components = app_world.write_component::<Self>();
            if let Some(start_button) = components.get_mut(*entity) {
                if let Self(pressed, .., label, Some(entity)) = start_button {
                    if ui.button(format!("{} {}", label, entity.id())) {
                        *pressed = Some(true);
                    }
                }

                ui.same_line();
                ui.text(&start_button.1);
            }
        }
    }
}
