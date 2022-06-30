
use atlier::system::Extension;
use specs::{Component, WorldExt};
use specs::storage::DenseVecStorage;

use super::StartButton;

/// This component is to enable sequencing within a task
#[derive(Component, Clone, Default)]
#[storage(DenseVecStorage)]
pub struct NextButton(
    /// Owner
    pub StartButton,
    /// Enable auto mode
    pub Option<bool>,
    /// Next
    pub Option<StartButton>
);

impl Extension for NextButton {
    fn configure_app_world(world: &mut specs::World) {
        world.register::<NextButton>();
    }

    fn on_ui(&'_ mut self, app_world: &specs::World, ui: &'_ imgui::Ui<'_>) {
        if let Self(.., Some(start_button)) = self {
            ui.text("Next - ");
            ui.same_line();
            start_button.on_ui(app_world, ui);
        }
    }
}