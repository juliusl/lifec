use atlier::system::App;

/// General descritpion, name, summary, etc
///
#[derive(Clone, Default, Hash, PartialEq, Eq)]
pub struct General {
    /// Name of this entity,
    pub name: String,
}

impl App for General {
    fn name() -> &'static str {
        "general_description"
    }

    fn edit_ui(&mut self, _: &imgui::Ui) {
        // no - op
    }

    fn display_ui(&self, ui: &imgui::Ui) {
        ui.text(format!("name: {}", self.name));
    }
}
