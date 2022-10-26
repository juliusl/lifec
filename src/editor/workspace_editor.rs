
use atlier::system::{Extension, App};
pub use tokio::sync::broadcast::{channel, Receiver, Sender};

use super::Appendix;

/// Extension
/// 
#[derive(Default)]
pub struct WorkspaceEditor {
    enable_demo: bool, 
    appendix: Appendix,
}

impl Extension for WorkspaceEditor {
    fn on_ui(&'_ mut self, _: &specs::World, ui: &'_ imgui::Ui<'_>) {
        ui.checkbox("enable imgui demo window", &mut self.enable_demo);
        if self.enable_demo {
            ui.show_demo_window(&mut self.enable_demo);
        }

        for (_, state) in self.appendix.state.iter_mut() {
            state.graph.edit_ui(ui);
            state.graph.display_ui(ui);
            ui.new_line();
            ui.separator();
        }
    }
}

impl From<Appendix> for WorkspaceEditor {
    fn from(appendix: Appendix) -> Self {
        Self { enable_demo: false, appendix }
    }
}