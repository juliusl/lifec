use imgui::Ui;

use crate::engine::EngineStatus;

pub trait EngineNode {
    /// Edit ui for an engine,
    /// 
    fn edit_engine(&mut self, ui: &Ui, engine: EngineStatus);

    /// Buttons related to engine,
    /// 
    fn engine_buttons(&mut self, ui: &Ui, engine: EngineStatus); 
}
