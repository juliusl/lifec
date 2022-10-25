use atlier::system::App;

/// General descritpion, name, summary, etc
/// 
#[derive(Default, Hash, PartialEq, Eq)]
pub struct General{
    pub name: String,
}

impl App for General {
    fn name() -> &'static str {
        "general"
    }

    fn edit_ui(&mut self, _: &imgui::Ui) {
        // no - op
    }

    fn display_ui(&self, ui: &imgui::Ui) {
        ui.text(format!("name: {}", self.name));
    }
}