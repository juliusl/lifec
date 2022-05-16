use std::{any::Any, fmt::Display};

use imgui::CollapsingHeader;
use specs::{Component, HashMapStorage};
use super::{Edit, Attribute, Value, App};

/// Section is a component of the runtime editor
/// it displays a collapsable section header, and renders it's editor in it's body
#[derive(Clone, Component)]
#[storage(HashMapStorage)]
pub struct Section<S: Any + Send + Sync + Clone> {
    pub title: String,
    pub editor: Edit<S>,
    pub state: S,
    pub attributes: Vec<Attribute>,
    /// enable to allow external systems to make changes to state, 
    /// in order for systems to commit these changes, RuntimeState::merge_with must be implemented (this is set todo!() by default)
    pub enable_app_systems: bool, 
}

impl<S: Any + Send + Sync + Clone> Section<S> {
    pub fn new(title: impl AsRef<str>, show: fn(&mut S, &imgui::Ui), initial_state: S, attributes: Vec<Attribute>) -> Section<S> {
        Section { title: title.as_ref().to_string(), editor: Edit(show), state: initial_state, enable_app_systems: false, attributes }
    }

    pub fn enable_app_systems(&self) -> Self {
        let mut next = self.clone();
        next.enable_app_systems = true; 
        next
    }

    pub fn add_attribute(&mut self, attr: Attribute) {
        self.attributes.push(attr);
    }

    pub fn text_attr(&mut self, name: impl AsRef<str>, init_value: impl AsRef<str>) {
        self.add_attribute(Attribute::new(0, name.as_ref().to_string(), Value::TextBuffer(init_value.as_ref().to_string())));
    }

    pub fn int_attr(&mut self, name: impl AsRef<str>, init_value: i32) {
        self.add_attribute(Attribute::new(0, name.as_ref().to_string(), Value::Int(init_value)));
    }

    pub fn float_attr(&mut self, name: impl AsRef<str>, init_value: f32) {
        self.add_attribute(Attribute::new(0, name.as_ref().to_string(), Value::Float(init_value)));
    }

    pub fn update(&mut self, func: impl Fn(&mut Self)) -> Self {
        let next = self; 

        (func)(next);

        next.to_owned()
    }
}

impl<S: Any + Send + Sync + Clone + App + Display> From<S> for Section<S> {
    fn from(initial: S) -> Self {
        Section {
            title: format!("{}: {}", S::name().to_string(), initial),
            editor: Edit(S::show_editor),
            state: initial,
            enable_app_systems: false,
            attributes: vec![],
        }
    }
}

impl<S: Any + Send + Sync + Clone> App for Section<S> {
    fn name() -> &'static str {
        "Section"
    }

    fn show_editor(&mut self, ui: &imgui::Ui) {
        if CollapsingHeader::new(&self.title).build(ui) {
            let (Edit(editor), s) = (&mut self.editor, &mut self.state);
            editor(s, ui);

            ui.new_line();
            ui.text("Attributes:"); 
            for a in self.attributes.iter_mut() {
                Attribute::show_editor(a, ui);
            }
        }
    }
}
