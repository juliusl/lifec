use std::fmt::Display;

use crate::RuntimeState;
use imgui::CollapsingHeader;
use specs::{Component, HashMapStorage};
use super::{App, Attribute, Edit, Value};

/// This trait allows others to author extensions using Section<S> as the main runtime-state
/// for the extension
pub trait SectionExtension<S: RuntimeState> {
    /// To consume this method must be called in the edit fn for the section
    fn extend_section(section: &mut Section<S>, ui: &imgui::Ui);
}

/// Section is a component of the runtime editor
/// it displays a collapsable section header, and renders it's editor in it's body
#[derive(Clone, Component)]
#[storage(HashMapStorage)]
pub struct Section<S: RuntimeState> {
    id: u32,
    pub title: String,
    pub editor: Edit<Section<S>>,
    pub state: S,
    pub attributes: Vec<Attribute>,
    /// enable to allow external systems to make changes to state,
    /// in order for systems to commit these changes, RuntimeState::merge_with must be implemented (this is set todo!() by default)
    pub enable_app_systems: bool,
    /// enable inherent attribute editor for section
    pub enable_edit_attributes: bool,
}

impl<S: RuntimeState> Section<S> {
    pub fn new(
        title: impl AsRef<str>,
        show: fn(&mut Section<S>, &imgui::Ui),
        initial_state: S,
    ) -> Section<S> {
        Section {
            id: 0,
            title: title.as_ref().to_string(),
            editor: Edit(show),
            state: initial_state,
            enable_app_systems: false,
            enable_edit_attributes: false,
            attributes: vec![],
        }
    }

    pub fn get_parent_entity(&self) -> u32 {
        self.id
    }

    pub fn get_attr(&self, with_name: impl AsRef<str>) -> Option<&Attribute> {
        self.attributes
            .iter()
            .find(|a| a.name() == with_name.as_ref())
    }

    pub fn get_attr_mut(&mut self, with_name: impl AsRef<str>) -> Option<&mut Attribute> {
        self.attributes
            .iter_mut()
            .find(|a| a.name() == with_name.as_ref())
    }

    pub fn show_attr_debug(
        &mut self,
        label: impl AsRef<str>,
        attr_name: impl AsRef<str>,
        ui: &imgui::Ui,
    ) {
        if let Some(value) = self.get_attr(attr_name) {
            ui.label_text(label.as_ref(), format!("{:?}", value));
        }
    }

    pub fn is_attr_checkbox(&self, with_name: impl AsRef<str>) -> Option<bool> {
        if let Some(Value::Bool(value)) = self.get_attr(with_name).and_then(|a| Some(a.value())) {
            Some(*value)
        } else {
            None
        }
    }

    pub fn edit_attr(
        &mut self,
        label: impl AsRef<str>,
        attr_name: impl AsRef<str>,
        ui: &imgui::Ui,
    ) {
        match self
            .get_attr_mut(attr_name)
            .and_then(|a| Some(a.get_value_mut()))
        {
            Some(Value::TextBuffer(val)) => {
                ui.input_text(label.as_ref(), val).build();
            }
            Some(Value::Int(val)) => {
                ui.input_int(label.as_ref(), val).build();
            }
            Some(Value::Float(val)) => {
                ui.input_float(label.as_ref(), val).build();
            }
            Some(Value::Bool(val)) => {
                ui.checkbox(label.as_ref(), val);
            }
            None => {}
            _ => todo!(),
        }
    }

    pub fn enable_app_systems(&self) -> Self {
        let mut next = self.clone();
        next.enable_app_systems = true;
        next
    }

    pub fn enable_edit_attributes(&self) -> Self {
        let mut next = self.clone();
        next.enable_edit_attributes = true;
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

    pub fn with_bool(&mut self, name: impl AsRef<str>, init_value: bool) -> Self {
        self.update(move |next| next.add_bool_attr(name, init_value))
    }

    pub fn with_parent_entity(&mut self, id: u32) -> Self {
        self.update(move |next| next.set_parent_entity(id))
    }

    pub fn add_attribute(&mut self, attr: Attribute) {
        self.attributes.push(attr);
    }

    pub fn add_text_attr(&mut self, name: impl AsRef<str>, init_value: impl AsRef<str>) {
        self.add_attribute(Attribute::new(
            self.id,
            name.as_ref().to_string(),
            Value::TextBuffer(init_value.as_ref().to_string()),
        ));
    }

    pub fn add_int_attr(&mut self, name: impl AsRef<str>, init_value: i32) {
        self.add_attribute(Attribute::new(
            self.id,
            name.as_ref().to_string(),
            Value::Int(init_value),
        ));
    }

    pub fn add_float_attr(&mut self, name: impl AsRef<str>, init_value: f32) {
        self.add_attribute(Attribute::new(
            self.id,
            name.as_ref().to_string(),
            Value::Float(init_value),
        ));
    }

    pub fn add_bool_attr(&mut self, name: impl AsRef<str>, init_value: bool) {
        self.add_attribute(Attribute::new(
            self.id,
            name.as_ref().to_string(),
            Value::Bool(init_value),
        ));
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

impl<S: RuntimeState + App> From<S> for Section<S> {
    fn from(initial: S) -> Self {
        Section {
            id: 0,
            title: format!("{}: {}", S::name().to_string(), initial),
            editor: Edit(|section, ui| {
                S::show_editor(&mut section.state, ui);
            }),
            state: initial,
            enable_app_systems: false,
            enable_edit_attributes: false,
            attributes: vec![],
        }
    }
}

impl<S: RuntimeState> App for Section<S> {
    fn name() -> &'static str {
        "Section"
    }

    fn show_editor(&mut self, ui: &imgui::Ui) {
        if CollapsingHeader::new(&self.title).build(ui) {
            ui.indent();
            let Edit(editor) = &mut self.editor;
            editor(self, ui);

            if self.enable_edit_attributes {
                ui.new_line();
                if CollapsingHeader::new(format!("Attributes {:#4x}", self.id)).build(ui) {
                    for a in self.attributes.iter_mut() {
                        a.edit(ui);
                        ui.new_line();
                    }
                }
            }
            ui.unindent();
        }
    }
}

impl<S> Default for Section<S>
where
    S: RuntimeState,
{
    fn default() -> Self {
        Self {
            id: Default::default(),
            title: Default::default(),
            editor: Edit(|_, _| {}),
            state: Default::default(),
            attributes: Default::default(),
            enable_app_systems: Default::default(),
            enable_edit_attributes: Default::default(),
        }
    }
}

impl<S> Display for Section<S>
where
    S: RuntimeState,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Section")
    }
}

impl<S> RuntimeState for Section<S>
where
    S: RuntimeState,
{
    type Error = ();

    fn load<Str: AsRef<str> + ?Sized>(&self, _: &Str) -> Self
    where
        Self: Sized,
    {
        todo!()
    }

    fn process<Str: AsRef<str> + ?Sized>(&self, _: &Str) -> Result<Self, Self::Error> {
        todo!()
    }
}
