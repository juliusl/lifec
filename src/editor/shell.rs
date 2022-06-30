use atlier::system::App;
use imgui::{InputTextCallbackHandler, StyleVar, StyleColor};


pub struct Shell {

}

impl InputTextCallbackHandler for Shell {
    
}

impl App for Shell {
    fn name() -> &'static str {
        "shell"
    }

    /// TODO: this should show the history
    fn display_ui(&self, _: &imgui::Ui) {
        // todo!()
    }

    /// TODO: this should be the cursor
    fn edit_ui(&mut self, _: &imgui::Ui) {
        // todo!()
    }
}