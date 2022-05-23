use std::{fmt::Display, fs, path::Path, collections::BTreeMap};

use super::{unique_title, App, Attribute, ShowEditor, Value};
use crate::RuntimeState;
use imgui::CollapsingHeader;
use serde::{Deserialize, Serialize};
use specs::{Component, HashMapStorage};

/// This trait allows others to author extensions using Section<S> as the main runtime-state
/// for the extension
pub trait SectionExtension<S>
where
    S: RuntimeState,
{
    /// To consume this method must be called in the edit fn for the section
    fn show_extension(section: &mut Section<S>, ui: &imgui::Ui);
}

/// Section is a component of the runtime editor
/// it displays a collapsable section header, and renders it's editor in it's body
/// The section also maintains a set of Attributes that can be published to systems running on
/// the app_world. Using attributes, you can use a section to create different types of forms and widgets in a
/// uniform manner.
#[derive(Clone, Component, Serialize, Deserialize)]
#[storage(HashMapStorage)]
pub struct Section<S>
where
    S: RuntimeState,
{
    /// id is the id of the parent entity of the section
    id: u32,
    /// title of this section, will be the header
    pub title: String,
    /// attributes are properties that this section owns and are editable
    pub attributes: BTreeMap<String, Attribute>,
    /// enable to allow external systems to make changes to state,
    /// in order for systems to commit these changes, RuntimeState::merge_with must be implemented (this is set todo!() by default)
    pub enable_app_systems: bool,
    /// enable inherent attribute editor for section
    pub enable_edit_attributes: bool,
    #[serde(skip)]
    pub state: S,
    /// main editor show function
    #[serde(skip)]
    pub show_editor: ShowEditor<Section<S>>,
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
            show_editor: ShowEditor(show),
            state: initial_state,
            enable_app_systems: false,
            enable_edit_attributes: false,
            attributes: BTreeMap::new(),
        }
    }

    /// The parent entity of this component
    pub fn get_parent_entity(&self) -> u32 {
        self.id
    }

    pub fn get_attr_value(&self, with_name: impl AsRef<str>) -> Option<&Value> {
        self.get_attr(with_name).and_then(|a| Some(a.value()))
    }

    pub fn get_attr_value_mut(&mut self, with_name: impl AsRef<str>) -> Option<&mut Value> {
        self.get_attr_mut(with_name)
            .and_then(|a| Some(a.get_value_mut()))
    }

    pub fn get_attr(&self, with_name: impl AsRef<str>) -> Option<&Attribute> {
        self.attributes
            .iter()
            .find(|(_, attr)| attr.name() == with_name.as_ref())
            .and_then(|(_, a)|Some(a))
    }

    pub fn get_attr_mut(&mut self, with_name: impl AsRef<str>) -> Option<&mut Attribute> {
        self.attributes
            .iter_mut()
            .find(|(_, attr)| attr.name() == with_name.as_ref())
            .and_then(|(_, a)|Some(a))
    }

    pub fn show_debug(&mut self, attr_name: impl AsRef<str>, ui: &imgui::Ui) {
        if let Some(value) = self.get_attr(attr_name) {
            ui.label_text(
                format!("Debug view of: {}, Entity: {}", value.name(), value.id()),
                format!("{:?}", value),
            );
        }
    }

    pub fn is_attr_checkbox(&self, with_name: impl AsRef<str>) -> Option<bool> {
        if let Some(Value::Bool(value)) = self.get_attr(with_name).and_then(|a| Some(a.value())) {
            Some(*value)
        } else {
            None
        }
    }

    pub fn modify_state_with_attr(
        &mut self,
        attr_name: impl AsRef<str>,
        update: impl Fn(&Attribute, &mut S),
    ) {
        let clone = self.clone();
        let attr = clone.get_attr(attr_name);
        if let Some(attr) = attr {
            let state = &mut self.state;
            update(attr, state);
        }
    }

    pub fn edit_state_string(
        &mut self,
        label: impl AsRef<str> + Display,
        attr_name: impl AsRef<str>,
        select: impl Fn(&mut S) -> Option<&mut String>,
        ui: &imgui::Ui,
    ) {
        self.edit_attr(label.as_ref(), attr_name.as_ref(), ui);
        self.modify_state_with_attr(attr_name.as_ref(), |a, s| {
            if let Value::TextBuffer(arg_value) = a.value() {
                if let Some(to_update) = select(s) {
                    *to_update = arg_value.to_string();
                }
            }
        });
    }

    /// This method allows you to edit an attribute from this section
    /// You can use a label that is different from the actual attribute name
    /// This allows attribute re-use
    pub fn edit_attr(
        &mut self,
        label: impl AsRef<str> + Display,
        attr_name: impl AsRef<str>,
        ui: &imgui::Ui,
    ) {
        let label = format!("{} {}", label, self.id);
        match self.get_attr_value_mut(attr_name) {
            Some(Value::TextBuffer(val)) => {
                ui.input_text(label, val).build();
            }
            Some(Value::Int(val)) => {
                ui.input_int(label, val).build();
            }
            Some(Value::Float(val)) => {
                ui.input_float(label, val).build();
            }
            Some(Value::Bool(val)) => {
                ui.checkbox(label, val);
            }
            Some(Value::FloatPair(f1, f2)) => {
                let clone = &mut [*f1, *f2];
                ui.input_float2(label, clone).build();
                *f1 = clone[0];
                *f2 = clone[1];
            }
            Some(Value::IntPair(i1, i2)) => {
                let clone = &mut [*i1, *i2];
                ui.input_int2(label, clone).build();
                *i1 = clone[0];
                *i2 = clone[1];
            }
            Some(Value::IntRange(i, i_min, i_max)) => {
                imgui::Slider::new(label, *i_min, *i_max).build(ui, i);
            }
            Some(Value::FloatRange(f, f_min, f_max)) => {
                imgui::Slider::new(label, *f_min, *f_max).build(ui, f);
            }
            None => {}
            _ => todo!(),
        }
    }

    /// This method allows you to create a custom editor for your attribute,
    /// in case the built in methods are not enough
    pub fn edit_attr_custom(&mut self, attr_name: impl AsRef<str>, show: impl Fn(&mut Attribute)) {
        if let Some(attr) = self.get_attr_mut(attr_name) {
            show(attr);
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

    pub fn with_symbol(&mut self, name: impl AsRef<str>) -> Self {
        self.update(move |next| next.add_empty_attr(name))
    }

    /// try to load a file into an attribute
    pub fn with_file(&mut self, file_name: impl AsRef<Path> + AsRef<str> + Display) -> Self {
        match fs::read_to_string(&file_name) {
            Ok(contents) => self.update(move |next| {
                next.add_binary_attr(format!("file::{}", file_name), contents.as_bytes().to_vec())
            }),
            Err(err) => {
                eprintln!(
                    "Could not load file '{}', for with_file on section '{}', entity {}. Error: {}",
                    &file_name, self.title, self.id, err
                );
                self.update(|_| {})
            }
        }
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

    pub fn with_float_pair(&mut self, name: impl AsRef<str>, init_value: &[f32; 2]) -> Self {
        self.update(move |next| next.add_float_pair_attr(name, init_value))
    }

    pub fn with_int_pair(&mut self, name: impl AsRef<str>, init_value: &[i32; 2]) -> Self {
        self.update(move |next| next.add_int_pair_attr(name, init_value))
    }

    pub fn with_int_range(&mut self, name: impl AsRef<str>, init_value: &[i32; 3]) -> Self {
        self.update(move |next| next.add_int_range_attr(name, init_value))
    }

    pub fn with_float_range(&mut self, name: impl AsRef<str>, init_value: &[f32; 3]) -> Self {
        self.update(move |next| next.add_float_range_attr(name, init_value))
    }

    pub fn with_parent_entity(&mut self, id: u32) -> Self {
        self.update(move |next| next.set_parent_entity(id))
    }

    pub fn add_empty_attr(&mut self, name: impl AsRef<str>) {
        self.add_attribute(Attribute::new(
            self.id,
            name.as_ref().to_string(),
            Value::Empty,
        ));
    }

    pub fn add_binary_attr(&mut self, name: impl AsRef<str>, init_value: impl Into<Vec<u8>>) {
        self.add_attribute(Attribute::new(
            self.id,
            name.as_ref().to_string(),
            Value::BinaryVector(init_value.into()),
        ));
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

    pub fn add_float_pair_attr(&mut self, name: impl AsRef<str>, init_value: &[f32; 2]) {
        self.add_attribute(Attribute::new(
            self.id,
            name.as_ref().to_string(),
            Value::FloatPair(init_value[0], init_value[1]),
        ));
    }

    pub fn add_int_pair_attr(&mut self, name: impl AsRef<str>, init_value: &[i32; 2]) {
        self.add_attribute(Attribute::new(
            self.id,
            name.as_ref().to_string(),
            Value::IntPair(init_value[0], init_value[1]),
        ));
    }

    pub fn add_int_range_attr(&mut self, name: impl AsRef<str>, init_value: &[i32; 3]) {
        self.add_attribute(Attribute::new(
            self.id,
            name.as_ref().to_string(),
            Value::IntRange(init_value[0], init_value[1], init_value[2]),
        ));
    }

    pub fn add_float_range_attr(&mut self, name: impl AsRef<str>, init_value: &[f32; 3]) {
        self.add_attribute(Attribute::new(
            self.id,
            name.as_ref().to_string(),
            Value::FloatRange(init_value[0], init_value[1], init_value[2]),
        ));
    }

    pub fn add_attribute(&mut self, attr: Attribute) {
        self.attributes.insert(format!("{}", attr), attr);
    }

    pub fn update(&mut self, func: impl FnOnce(&mut Self)) -> Self {
        let next = self;

        (func)(next);

        next.to_owned()
    }

    pub fn set_parent_entity(&mut self, id: u32) {
        self.id = id;
        for (_, a) in self.attributes.iter_mut() {
            a.set_id(id);
        }
    }
}

impl<S: RuntimeState + App> From<S> for Section<S> {
    fn from(initial: S) -> Self {
        Section {
            id: 0,
            title: unique_title(S::name().to_string()),
            show_editor: ShowEditor(|section, ui| {
                S::show_editor(&mut section.state, ui);
            }),
            state: initial,
            enable_app_systems: false,
            enable_edit_attributes: false,
            attributes: BTreeMap::new(),
        }
    }
}

impl<S: RuntimeState> App for Section<S> {
    fn name() -> &'static str {
        "Section"
    }

    fn show_editor(&mut self, ui: &imgui::Ui) {
        if CollapsingHeader::new(&self.title)
            .default_open({
                if let Some(Value::Bool(true)) = self.get_attr_value("opened::") {
                    true
                } else {
                    false
                }
            })
            .build(ui)
        {
            ui.indent();
            let ShowEditor(editor) = &mut self.show_editor;
            editor(self, ui);

            if self.enable_edit_attributes {
                ui.new_line();
                if CollapsingHeader::new(format!("Attributes {:#4x}", self.id)).build(ui) {
                    if ui.button(format!("Add text Section[{}]", self.id)) {
                        self.add_text_attr(unique_title("Text"), "");
                    }
                    ui.same_line();
                    if ui.button(format!("Add int Section[{}]", self.id)) {
                        self.add_int_attr(unique_title("Int"), 0);
                    }
                    ui.same_line();
                    if ui.button(format!("Add float Section[{}]", self.id)) {
                        self.add_float_attr(unique_title("Float"), 0.0);
                    }
                    ui.same_line();
                    if ui.button(format!("Add bool Section[{}]", self.id)) {
                        self.add_bool_attr(unique_title("Bool"), false);
                    }
                    ui.new_line();
                    for (_, a) in self.attributes.iter_mut() {
                        a.edit(ui);
                        ui.new_line();
                    }
                }
            }
            ui.unindent();
            if let Some(Value::Bool(val)) = self.get_attr_value_mut("opened::") {
                *val = true;
            }
        } else {
            if let Some(Value::Bool(val)) = self.get_attr_value_mut("opened::") {
                *val = false;
            }
        }

        if let Some(Value::TextBuffer(title)) = self.get_attr_value("title::") {
            self.title = title.clone();
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
            show_editor: ShowEditor(|s, ui| {
                let label = format!("edit attributes for {}", s.title);
                ui.checkbox(label, &mut s.enable_edit_attributes);
            }),
            state: Default::default(),
            attributes: Default::default(),
            enable_app_systems: Default::default(),
            enable_edit_attributes: Default::default(),
        }
        .with_bool("opened::", false)
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

    fn process<Str: AsRef<str> + ?Sized>(&self, _: &Str) -> Result<Self, Self::Error> {
        todo!()
    }

    fn from_attributes(_: Vec<Attribute>) -> Self {
        todo!()
    }

    fn into_attributes(&self) -> Vec<Attribute> {
        todo!()
    }
}
