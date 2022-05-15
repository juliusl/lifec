use std::{any::Any, fmt::Display};

use atlier::system::App;
use imgui::CollapsingHeader;
use specs::{Component, HashMapStorage};

use super::Edit;

/// Section is a component of the runtime editor
/// it displays a collapsable section header, and renders it's editor in it's body
#[derive(Clone, Component)]
#[storage(HashMapStorage)]
pub struct Section<S: Any + Send + Sync + Clone> {
    pub title: String,
    pub editor: Edit<S>,
    pub state: S,
    /// enable to allow external systems to make changes to state, 
    /// in order for systems to commit these changes, RuntimeState::merge_with must be implemented (this is set todo!() by default)
    pub enable_app_systems: bool, 
}

impl<S: Any + Send + Sync + Clone> Section<S> {
    pub fn enable_app_systems(&self) -> Self {
        let mut next = self.clone();
        next.enable_app_systems = true; 
        next
    }
}

impl<S: Any + Send + Sync + Clone + App + Display> From<S> for Section<S> {
    fn from(initial: S) -> Self {
        Section {
            title: format!("{}: {}", S::title().to_string(), initial),
            editor: Edit(S::show_editor),
            state: initial,
            enable_app_systems: false,
        }
    }
}

impl<S: Any + Send + Sync + Clone> App for Section<S> {
    fn title() -> &'static str {
        "Section"
    }

    fn show_editor(&mut self, ui: &imgui::Ui) {
        if CollapsingHeader::new(&self.title).build(ui) {
            let (Edit(editor), s) = (&mut self.editor, &mut self.state);
            editor(s, ui);
        }
    }
}
