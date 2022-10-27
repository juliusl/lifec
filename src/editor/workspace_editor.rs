
use atlier::system::{Extension};
pub use tokio::sync::broadcast::{channel, Receiver, Sender};

use crate::engine::Engines;

use super::Appendix;

/// Extension
/// 
#[derive(Default)]
pub struct WorkspaceEditor {
    enable_demo: bool, 
    appendix: Appendix,
}

impl Extension for WorkspaceEditor {
    fn on_ui(&'_ mut self, world: &specs::World, ui: &'_ imgui::Ui<'_>) {
        ui.checkbox("enable imgui demo window", &mut self.enable_demo);
        if self.enable_demo {
            ui.show_demo_window(&mut self.enable_demo);
        }

        {
            let engines = world.system_data::<Engines>();

            for e in engines.scan_engines() {
                match e {
                    crate::engine::EngineStatus::Inactive(e) => {
                        if let Some(name) = self.appendix.name(&e) {
                            ui.text(format!("engine: {name}"));
                            ui.text(format!("status: inactive"));
                            ui.text(format!("id: {}", e.id()));
                        }
                    },
                    crate::engine::EngineStatus::Active(e) => {
                        if let Some(name) = self.appendix.name(&e) {
                            ui.text(format!("engine: {name}"));
                            ui.text(format!("status: active"));
                            ui.text(format!("id: {}", e.id()));
                        }
                    },
                }
                ui.new_line();
                ui.separator();
            }
        }

        // for (_, state) in self.appendix.state.iter_mut() {
        //     state.graph.edit_ui(ui);
        //     state.graph.display_ui(ui);
        //     ui.new_line();
        //     ui.separator();
        // }
    }
}

impl From<Appendix> for WorkspaceEditor {
    fn from(appendix: Appendix) -> Self {
        Self { enable_demo: false, appendix }
    }
}