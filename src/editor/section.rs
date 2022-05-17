use std::{any::Any, fmt::Display};

use imgui::CollapsingHeader;
use specs::{Component, HashMapStorage};
use super::{Edit, Attribute, Value, App};

/// Section is a component of the runtime editor
/// it displays a collapsable section header, and renders it's editor in it's body
#[derive(Clone, Component)]
#[storage(HashMapStorage)]
pub struct Section<S: Any + Send + Sync + Clone> {
    id: u32,
    pub title: String,
    pub editor: Edit<Section<S>>,
    pub state: S,
    pub attributes: Vec<Attribute>,
    /// enable to allow external systems to make changes to state, 
    /// in order for systems to commit these changes, RuntimeState::merge_with must be implemented (this is set todo!() by default)
    pub enable_app_systems: bool, 
}

impl<S: Any + Send + Sync + Clone> Section<S> {
    pub fn new(title: impl AsRef<str>, show: fn(&mut Section<S>, &imgui::Ui), initial_state: S, attributes: Vec<Attribute>) -> Section<S> {
        Section { id: 0, title: title.as_ref().to_string(), editor: Edit(show), state: initial_state, enable_app_systems: false, attributes }
    }

    pub fn enable_app_systems(&self) -> Self {
        let mut next = self.clone();
        next.enable_app_systems = true; 
        next
    }

    pub fn with_title(&mut self, title: impl AsRef<str>) -> Self {
        self.update(move |next| next.title = title.as_ref().to_string())
    }

    pub fn with_attribute(&mut self, attr: Attribute) -> Self {
        let attr = attr;
        self.update(move |next| next.add_attribute(attr))
    }

    pub fn with_text(&mut self, name: impl AsRef<str>, init_value: impl AsRef<str>) -> Self {
        self.update(move |next| next.add_text_attr(name, init_value))
    }

    pub fn with_int(&mut self, name: impl AsRef<str>, init_value: i32) -> Self {
        self.update(move |next| next.add_int_attr(name, init_value))
    }

    pub fn with_float(&mut self, name: impl AsRef<str>, init_value: f32) -> Self {
        self.update(move |next| next.add_float_attr(name, init_value))
    }

    pub fn with_parent_entity(&mut self, id: u32) -> Self {
        self.update(move |next| next.set_parent_entity(id))
    }

    pub fn add_attribute(&mut self, attr: Attribute) {
        self.attributes.push(attr);
    }

    pub fn add_text_attr(&mut self, name: impl AsRef<str>, init_value: impl AsRef<str>) {
        self.add_attribute(Attribute::new(0, name.as_ref().to_string(), Value::TextBuffer(init_value.as_ref().to_string())));
    }

    pub fn add_int_attr(&mut self, name: impl AsRef<str>, init_value: i32) {
        self.add_attribute(Attribute::new(0, name.as_ref().to_string(), Value::Int(init_value)));
    }

    pub fn add_float_attr(&mut self, name: impl AsRef<str>, init_value: f32) {
        self.add_attribute(Attribute::new(0, name.as_ref().to_string(), Value::Float(init_value)));
    }

    pub fn update(&mut self, func: impl FnOnce(&mut Self)) -> Self {
        let next = self; 

        (func)(next);

        next.to_owned()
    }

    pub fn set_parent_entity(&mut self, id: u32) {
        self.id = id;
        for a in self.attributes.iter_mut() {
            a.set_id(id);
        }
    }
}

impl<S: Any + Send + Sync + Clone + App + Display> From<S> for Section<S> {
    fn from(initial: S) -> Self {
        Section {
            id: 0,
            title: format!("{}: {}", S::name().to_string(), initial),
            editor: Edit(|section, ui| {
                S::show_editor(&mut section.state, ui);
            }),
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
            ui.indent();
            let Edit(editor) = &mut self.editor;
            editor(self, ui);

            if !self.attributes.is_empty() {
                ui.new_line();
                if CollapsingHeader::new(format!("Attributes {:#4x}", self.id)).build(ui) {
                    for a in self.attributes.iter_mut() {
                        Attribute::show_editor(a, ui);
                    }
                }
            }
            ui.unindent();
        }
    }
}
