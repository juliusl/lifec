use std::{fmt::Display, fs, path::Path};

use super::{unique_title, App, Attribute, ShowEditor, Value};
use crate::{RuntimeState, AttributeGraph};
use imgui::CollapsingHeader;
use serde::{Deserialize, Serialize};
use specs::{Component, HashMapStorage, Entity};

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
    pub attributes: AttributeGraph,
    /// enable to allow external systems to make changes to state,
    /// in order for systems to commit these changes, RuntimeState::merge_with must be implemented (this is set todo!() by default)
    pub enable_app_systems: bool,
    /// enable inherent attribute editor for section
    pub enable_edit_attributes: bool,
    #[serde(skip)]
    pub gen: u64,
    #[serde(skip)]
    pub state: S,
    /// main editor show function
    #[serde(skip)]
    pub show_editor: ShowEditor<Section<S>>,
}

impl<S: RuntimeState> Section<S> {
    pub fn new(
        title: impl AsRef<str>,
        attributes: AttributeGraph,
        show: fn(&mut Section<S>, &imgui::Ui),
        initial_state: S,
    ) -> Section<S> {
        let mut section = Section {
            gen: 0,
            id: 0,
            title: title.as_ref().to_string(),
            show_editor: ShowEditor(show),
            state: initial_state.clone(),
            enable_app_systems: false,
            enable_edit_attributes: false,
            attributes,
        };

        let state_attrs = initial_state.state();
        state_attrs.as_ref().iter_attributes().for_each(|a| section.attributes.copy_attribute(a) );
        section
    }

    pub fn get_gen(&self) -> u64 {
        self.gen
    }

    pub fn next_gen(&mut self) {
        self.gen += 1;
    }

    /// The parent entity of this component
    pub fn get_parent_entity(&self) -> u32 {
        self.id
    }

    pub fn show_debug(&mut self, attr_name: impl AsRef<str>, ui: &imgui::Ui) {
        if let Some(value) = self.attributes.find_attr(attr_name) {
            ui.label_text(
                format!("Debug view of: {}, Entity: {}", value.name(), value.id()),
                format!("{:?}", value),
            );
        }
    }

    pub fn is_attr_checkbox(&self, with_name: impl AsRef<str>) -> Option<bool> {
        self.attributes.is_enabled(with_name)
    }

    pub fn modify_state_with_attr(
        &mut self,
        attr_name: impl AsRef<str>,
        update: impl Fn(&Attribute, &mut S),
    ) {
        let clone = self.clone();
        let attr = clone.attributes.find_attr(attr_name);
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
        if let Some(Value::Float(width)) = self.attributes.find_attr_value("edit_width::") {
            ui.set_next_item_width(*width);
        } else {
            ui.set_next_item_width(130.0);
        }

        let label = format!("{} {}", label, self.id);
        let attr_name = attr_name.as_ref().to_string();
        match self.attributes.find_attr_value_mut(&attr_name) {
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
            _ => {
                match self.attributes.find_attr_mut(&attr_name) {
                    Some(attr) => {
                        attr.show_editor(ui);
                    },
                    None => {},
                }
            },
        }
    }

    /// This method allows you to create a custom editor for your attribute,
    /// in case the built in methods are not enough
    pub fn edit_attr_custom(&mut self, attr_name: impl AsRef<str>, show: impl Fn(&mut Attribute)) {
        if let Some(attr) = self.attributes.find_attr_mut(attr_name) {
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

    /// try to load a file into an attribute
    pub fn with_file(&mut self, file_name: impl AsRef<Path> + AsRef<str> + Display) -> &mut Self {
        self.with_file_src(&file_name, &file_name)
    }

    // /// try to load a file into an attribute
    pub fn with_file_src(&mut self, file_name: impl AsRef<str>, src: impl AsRef<Path> + AsRef<str> + Display)-> &mut Self {
        match fs::read_to_string(&src) {
            Ok(contents) => self.edit_attributes().add_binary_attr(format!("file::{}", file_name.as_ref()), contents.as_bytes().to_vec()),
            Err(err) => eprintln!(
                    "Could not load file '{}', for with_file on section '{}', entity {}. Error: {}",
                    &src, self.title, self.id, err
                )
        }
        self
    }

    pub fn with_title(&mut self, title: impl AsRef<str>) -> &mut Self {
        self.edit_attributes().add_text_attr("title::", title);
        self
    }

    pub fn with_attribute(&mut self, attribute: &Attribute) -> &mut Self {
        self.edit_attributes().copy_attribute(attribute);
        self
    }

    pub fn with_parent_entity(&mut self, entity: Entity) -> &mut Self {
        self.edit_attributes().set_parent_entity(entity);
        self
    }

    pub fn with_parent_entity_id(&mut self, entity_id: u32) -> &mut Self {
        self.edit_attributes().set_parent_entity_id(entity_id);
        self
    }

    pub fn edit_attributes(&mut self) -> &mut AttributeGraph {
        &mut self.attributes
    }
}

impl<S> From<S> for Section<S> 
    where 
    S: RuntimeState + App,
{
    fn from(initial: S) -> Self {
        Section {
            gen: 0,
            id: 0,
            title: unique_title(S::name().to_string()),
            show_editor: ShowEditor(|section, ui| {
                S::show_editor(&mut section.state, ui);
            }),
            state: initial,
            enable_app_systems: false,
            enable_edit_attributes: false,
            attributes: AttributeGraph::default(),
        }
    }
}

impl<S> App for Section<S> 
where 
    S: RuntimeState,
{
    fn name() -> &'static str {
        "Section"
    }

    fn show_editor(&mut self, ui: &imgui::Ui) {
        if CollapsingHeader::new(&self.title)
            .default_open({
                if let Some(Value::Bool(true)) = self.attributes.find_attr_value("opened::") {
                    true
                } else {
                    false || (self.id == 0)
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
                        self.attributes.add_text_attr(unique_title("Text"), "");
                    }
                    ui.same_line();
                    if ui.button(format!("Add int Section[{}]", self.id)) {
                        self.attributes.add_int_attr(unique_title("Int"), 0);
                    }


                    if ui.button(format!("Add float Section[{}]", self.id)) {
                        self.attributes.add_float_attr(unique_title("Float"), 0.0);
                    }
                    ui.same_line();
                    if ui.button(format!("Add bool Section[{}]", self.id)) {
                        self.attributes.add_bool_attr(unique_title("Bool"), false);
                    }
                    ui.new_line();
                    for a in self.attributes.iter_mut_attributes() {
                        a.edit_ui(ui);
                        ui.same_line();
                        if ui.button(format!("remove [{} {}]", a.name(), self.id)) {
                            let value = a.value_mut();
                            *value = Value::Empty;
                        }
                        ui.new_line();
                    }
                }
            }
            ui.unindent();
            if let Some(Value::Bool(val)) = self.attributes.find_attr_value_mut("opened::") {
                *val = true;
            }
        } else {
            if let Some(Value::Bool(val)) = self.attributes.find_attr_value_mut("opened::") {
                *val = false;
            }
        }

        if let Some(Value::TextBuffer(title)) = self.attributes.find_attr_value("title::") {
            self.title = title.clone();
        }

        self.cleanup_empty_attributes();
    }
}

impl<S> Section<S> 
where
    S: RuntimeState
{
    fn cleanup_empty_attributes(&mut self) {
        self.attributes.clone().iter_attributes().filter(|v| v.value() == &Value::Empty).for_each(|v| {
            self.attributes.remove(v);
        });
    }
}

impl<S> Default for Section<S>
where
    S: RuntimeState,
{
    fn default() -> Self {
        Self {
            gen: 0,
            id: Default::default(),
            title: Default::default(),
            show_editor: ShowEditor(|s, ui| {
                s.edit_attr("edit events", "enable event builder", ui);

                let label = format!("edit attributes {}", s.get_parent_entity());
                ui.checkbox(label, &mut s.enable_edit_attributes);

                s.edit_attr("save to project", "enable project", ui);

                if let Some(true) = s.is_attr_checkbox("enable project") {
                    s.edit_attr("edit project name", "project::name::", ui);
                }
            }),
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

impl<S> From<AttributeGraph> for Section<S> 
where
    S: RuntimeState
{
    fn from(_: AttributeGraph) -> Self {
        todo!();
    }
}

impl<S> RuntimeState for Section<S>
where
    S: RuntimeState,
{
    type Error = ();
    type State = AttributeGraph;

    fn dispatch(&self, _: impl AsRef<str>) -> Result<Self, Self::Error> {
        todo!()
    }

    // fn from_attributes(attributes: Vec<Attribute>) -> Self {
    //     let mut next = Self::default();

    //     let state = S::from_attributes(attributes.clone());
    //     next.state = state;

    //     let section = SectionAttributes::from(attributes);

    //     if let Some(Value::TextBuffer(title)) = section.get_attr_value("title::") {
    //         next.title = title.to_string(); 
    //     }

    //     next
    // }

    // fn into_attributes(&self) -> Vec<Attribute> {
    //     let mut attrs: Vec<Attribute> = self.attributes
    //         .iter_attributes()
    //         .map(|a| a).cloned()
    //         .collect();

    //     let mut state_attrs = self.state.into_attributes();
    //     attrs.append(&mut state_attrs);

    //     if let Some(Value::TextBuffer(_)) = self.attributes.get_attr_value("title::") {
    //         attrs.push(self.attributes.get_attr("title::").expect("just checked").clone());
    //     } else {
    //         attrs.push(Attribute::new(self.id, "title::", Value::TextBuffer(self.title.to_string())));
    //     }

    //     attrs
    // }
}
