use imgui::{CollapsingHeader, Window};
use knot::store::Store;
use serde::{Deserialize, Serialize};
use specs::{
    storage::DenseVecStorage, storage::DefaultVecStorage, Component, Entities, Join, ReadStorage, System, Write,
    WriteStorage, Read,
};
use std::{
    collections::BTreeMap,
    fmt::Display,
    ops::{Deref, DerefMut},
    time::Instant,
};

use super::{
    event_graph::EventGraph, section::Section, unique_title, App, Attribute, EventComponent, Value,
};
use crate::{Action, Runtime, RuntimeState};

#[derive(Clone)]
pub struct RuntimeEditor<S>
where
    S: RuntimeState,
{
    pub runtime: Runtime<S>,
    pub events: Vec<EventComponent>,
    pub sections: BTreeMap<u32, Section<S>>,
    pub running: (Option<bool>, Option<Instant>, Option<Instant>),
    pub dispatch_snapshot: Option<()>,
    pub dispatch_remove: Option<u32>,
}

impl<S: RuntimeState> From<Runtime<S>> for RuntimeEditor<S> {
    fn from(runtime: Runtime<S>) -> Self {
        let events = runtime
            .get_listeners()
            .iter()
            .enumerate()
            .filter_map(|(id, l)| match (&l.action, &l.next) {
                (Action::Dispatch(msg), Some(transition)) => Some(EventComponent {
                    label: format!("Event {}", id),
                    on: l.event.to_string(),
                    dispatch: msg.to_string(),
                    call: String::default(),
                    transitions: vec![transition.to_string()],
                    // flags: parse_flags(l.extensions.get_args()),
                    // variales: parse_variables(l.extensions.get_args()),
                }),
                (Action::Call(call), _) => Some(EventComponent {
                    label: format!("Event {}", id),
                    on: l.event.to_string(),
                    call: call.to_string(),
                    dispatch: String::default(),
                    transitions: l
                        .extensions
                        .tests
                        .iter()
                        .map(|(_, t)| t.to_owned())
                        .collect(),
                    //     flags: parse_flags(l.extensions.get_args()),
                    //     variales: parse_variables(l.extensions.get_args()),
                }),
                _ => None,
            })
            .collect();

        let mut sections: BTreeMap<u32, Section<S>> = BTreeMap::new();
        let sections = &mut sections;
        runtime.attributes.iter().for_each(|a| {
            if let Some(section) = sections.get_mut(&a.id()) {
                section.add_attribute(a.clone());
            } else {
                sections.insert(
                    a.id(),
                    Section::<S>::default()
                        .with_attribute(a.clone())
                        .with_title(format!("Runtime Entity {}", a.id()))
                        .with_parent_entity(a.id()),
                );
            }
        });

        let sections = sections.clone();
        let next = Self {
            runtime,
            events,
            sections,
            running: (None, None, None),
            dispatch_snapshot: None,
            dispatch_remove: None,
        };
        next
    }
}

#[derive(Default, Component, Clone, Serialize, Deserialize)]
#[storage(DenseVecStorage)]
pub struct SectionAttributes(Vec<Attribute>);

impl From<Vec<Attribute>> for SectionAttributes {
    fn from(attrs: Vec<Attribute>) -> Self {
        Self(attrs)
    }
}

impl Display for SectionAttributes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

impl SectionAttributes {
    pub fn with_parent_entity(&mut self, id: u32) -> Self {
        self.update(move |next| {
            for a in next.get_attrs_mut() {
                a.set_id(id);
            }
        })
    }

    pub fn get_attrs(&self) -> Vec<&Attribute> {
        self.0.iter().collect()
    }

    pub fn clone_attrs(&self) -> Vec<Attribute> {
        self.0.iter().cloned().collect()
    }

    pub fn get_attr(&self, name: impl AsRef<str>) -> Option<&Attribute> {
        let SectionAttributes(attributes) = self;

        attributes.iter().find(|a| a.name() == name.as_ref())
    }

    pub fn get_attr_mut(&mut self, name: impl AsRef<str>) -> Option<&mut Attribute> {
        let SectionAttributes(attributes) = self;

        attributes.iter_mut().find(|a| a.name() == name.as_ref())
    }

    pub fn get_attr_value(&self, with_name: impl AsRef<str>) -> Option<&Value> {
        self.get_attr(with_name).and_then(|a| Some(a.value()))
    }

    pub fn get_attr_value_mut(&mut self, with_name: impl AsRef<str>) -> Option<&mut Value> {
        self.get_attr_mut(with_name)
            .and_then(|a| Some(a.get_value_mut()))
    }

    pub fn get_attrs_mut(&mut self) -> &mut Vec<Attribute> {
        &mut self.0
    }

    pub fn is_attr_checkbox(&self, name: impl AsRef<str>) -> Option<bool> {
        if let Some(Value::Bool(val)) = self.get_attr(name).and_then(|a| Some(a.value())) {
            Some(*val)
        } else {
            None
        }
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

    pub fn add_empty_attr(&mut self, name: impl AsRef<str>) {
        self.add_attribute(Attribute::new(0, name.as_ref().to_string(), Value::Empty));
    }

    pub fn add_binary_attr(&mut self, name: impl AsRef<str>, init_value: impl Into<Vec<u8>>) {
        self.add_attribute(Attribute::new(
            0,
            name.as_ref().to_string(),
            Value::BinaryVector(init_value.into()),
        ));
    }

    pub fn add_text_attr(&mut self, name: impl AsRef<str>, init_value: impl AsRef<str>) {
        self.add_attribute(Attribute::new(
            0,
            name.as_ref().to_string(),
            Value::TextBuffer(init_value.as_ref().to_string()),
        ));
    }

    pub fn add_int_attr(&mut self, name: impl AsRef<str>, init_value: i32) {
        self.add_attribute(Attribute::new(
            0,
            name.as_ref().to_string(),
            Value::Int(init_value),
        ));
    }

    pub fn add_float_attr(&mut self, name: impl AsRef<str>, init_value: f32) {
        self.add_attribute(Attribute::new(
            0,
            name.as_ref().to_string(),
            Value::Float(init_value),
        ));
    }

    pub fn add_bool_attr(&mut self, name: impl AsRef<str>, init_value: bool) {
        self.add_attribute(Attribute::new(
            0,
            name.as_ref().to_string(),
            Value::Bool(init_value),
        ));
    }

    pub fn add_float_pair_attr(&mut self, name: impl AsRef<str>, init_value: &[f32; 2]) {
        self.add_attribute(Attribute::new(
            0,
            name.as_ref().to_string(),
            Value::FloatPair(init_value[0], init_value[1]),
        ));
    }

    pub fn add_int_pair_attr(&mut self, name: impl AsRef<str>, init_value: &[i32; 2]) {
        self.add_attribute(Attribute::new(
            0,
            name.as_ref().to_string(),
            Value::IntPair(init_value[0], init_value[1]),
        ));
    }

    pub fn add_int_range_attr(&mut self, name: impl AsRef<str>, init_value: &[i32; 3]) {
        self.add_attribute(Attribute::new(
            0,
            name.as_ref().to_string(),
            Value::IntRange(init_value[0], init_value[1], init_value[2]),
        ));
    }

    pub fn add_float_range_attr(&mut self, name: impl AsRef<str>, init_value: &[f32; 3]) {
        self.add_attribute(Attribute::new(
            0,
            name.as_ref().to_string(),
            Value::FloatRange(init_value[0], init_value[1], init_value[2]),
        ));
    }

    pub fn add_attribute(&mut self, attr: Attribute) {
        self.0.push(attr);
    }

    pub fn update(&mut self, func: impl FnOnce(&mut Self)) -> Self {
        let next = self;

        (func)(next);

        next.to_owned()
    }
}

impl<'a, S> System<'a> for RuntimeEditor<S>
where
    S: RuntimeState + Component,
{
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Section<S>>,
        WriteStorage<'a, Loader>,
        Write<'a, RuntimeEditor<S>>,
        Write<'a, Dispatch>,
    );

    /// The runtime editor maintains a vector of sections that it displays
    /// This system coordinates updates to those sections, as well as initialization
    fn run(
        &mut self,
        (entities, read_sections, mut loader, mut runtime_editor, mut dispatcher): Self::SystemData,
    ) {
        if let Some(_) = self.dispatch_snapshot.take() {
            if let Some(state) = &self.runtime.state {
                let next = self.sections.len() as u32;
                let mut section = Section::new(
                    unique_title(format!("{}", self.runtime.context())),
                    |s, ui| {
                        s.edit_attr("edit events", "enable event builder", ui);

                        let label = format!("edit attributes {}", s.get_parent_entity());
                        ui.checkbox(label, &mut s.enable_edit_attributes);

                        s.edit_attr("save to project", "enable project", ui);

                        if let Some(true) = s.is_attr_checkbox("enable project") {
                            s.edit_attr("edit project name", "project::name::", ui);
                        }
                    },
                    state.clone(),
                )
                .enable_app_systems()
                .with_text("context::", format!("{}", self.runtime.context()))
                .with_bool("enable event builder", false)
                .with_text("project::name::", unique_title("snapshot"))
                .with_bool("enable project", false)
                .with_parent_entity(next);

                let section_attrs = SectionAttributes(section.into_attributes());

                let next = entities.create();
                match loader.insert(next, Loader::LoadSection(section_attrs)) {
                    Ok(_) => {
                        self.sections.insert(
                            next.id(),
                            section.with_parent_entity(next.id())
                        );
        
                        println!("Loading {:?}", next);
                    },
                    Err(_) => {},
                }
            }
        }

        if let Some(to_remove) = self.dispatch_remove.take() {
            let msg = dispatcher.deref_mut();
            self.sections.remove(&to_remove);
            *msg = Dispatch::RemoveSnapshot(to_remove);
            return;
        }

        for (e, s) in (&entities, &read_sections).join() {
            match self.sections.get(&e.id()) {
                None => {
                    match loader.insert(e, Loader::LoadSection(SectionAttributes(s.into_attributes()))) {
                        Ok(_) => {
                            println!("Loading {:?}", e);
                        },
                        Err(_) => {},
                    }
                }
                Some(section) => {
                    let Section {
                        title,
                        attributes,
                        enable_app_systems,
                        enable_edit_attributes,
                        state,
                        ..
                    } = section;

                    if *enable_app_systems {
                        let title = title.to_string();
                        let state = state.merge_with(&s.state);
                        let attributes = attributes.clone();
                        let enable_edit_attributes = *enable_edit_attributes;
                        self.sections.insert(e.id(), {
                            let mut s = s.clone().with_parent_entity(e.id());
                            s.title = title;
                            s.state = state;
                            s.attributes = attributes;
                            s.enable_edit_attributes = enable_edit_attributes;
                            s.enable_app_systems = true;
                            s
                        });
                    }
                }
            }
        }

        for section in read_sections.join() {
            if let None = self.sections.get(&section.get_parent_entity()) {
                println!("runtime editor inserting section {}", section.get_parent_entity());
                self.sections
                    .insert(section.get_parent_entity(), section.clone());
            }
        }

        let runtime_editor = runtime_editor.deref_mut();
        *runtime_editor = self.clone();
    }
}

#[derive(Default)]
pub struct RuntimeDispatcher<S>
where
    S: RuntimeState + Component,
{
    runtime: Option<RuntimeEditor<S>>,
}

impl<S> From<RuntimeEditor<S>> for RuntimeDispatcher<S>
where
    S: RuntimeState + Component,
{
    fn from(runtime: RuntimeEditor<S>) -> Self {
        Self {
            runtime: Some(runtime),
        }
    }
}

pub enum Dispatch {
    Empty,
    RemoveSnapshot(u32),
}

#[derive(Component)]
#[storage(DefaultVecStorage)]
pub enum Loader {
    Empty,
    LoadSection(SectionAttributes),
}

impl Default for Loader {
    fn default() -> Self {
        Loader::Empty
    }
}

impl Default for Dispatch {
    fn default() -> Self {
        Self::Empty
    }
}

impl<'a, S> System<'a> for RuntimeDispatcher<S>
where
    S: RuntimeState + Component,
{
    type SystemData = (
        Entities<'a>,
        Read<'a, RuntimeEditor<S>>,
        Write<'a, Dispatch>,
        WriteStorage<'a, Loader>,
        WriteStorage<'a, Section<S>>,
        WriteStorage<'a, SectionAttributes>,
        WriteStorage<'a, EventGraph>,
    );

    fn run(
        &mut self,
        (
            entities,
            runtime_editor,
            mut msg,
            mut loader,
            mut sections,
            mut section_attributes,
            mut event_graph,
        ): Self::SystemData,
    ) {
        if let Dispatch::RemoveSnapshot(id) = msg.deref() {
            let to_remove = entities.entity(*id);
            match entities.delete(to_remove) {
                Ok(_) => {
                    let msg = msg.deref_mut();
                    *msg = Dispatch::Empty;
                    sections.remove(to_remove);
                    section_attributes.remove(to_remove);
                    event_graph.remove(to_remove);
                    return;
                }
                Err(err) => eprintln!("RuntimeDispatcher Error {}", err),
            }
        }

        for e in entities.join() {
            if let Some(Loader::LoadSection(attributes)) = loader.get(e) {
                let id = e.id();
                println!("Load section {}", id);
    
                let entity = entities.entity(id);
    
                let attributes: Vec<Attribute> = attributes
                    .get_attrs()
                    .iter()
                    .cloned()
                    .map(|a| a.to_owned())
                    .collect();
                let initial = S::from_attributes(attributes.clone());
    
                let mut section = Section::<S>::default();
                section.state = initial;
    
                attributes.iter().cloned().for_each(|a| {
                    section.add_attribute(a.clone());
                });
    
                if let Some(Value::TextBuffer(title)) = section.clone().get_attr_value("title::") {
                    let section = section.with_title(title.to_string()).with_parent_entity(id);
                    match sections.insert(entity, section.clone()) {
                        Ok(_) => {
                            println!("RuntimeDispatcher added Section {}, {}", id, &section.title);
                        }
                        Err(err) => {
                            println!("section could not be loaded {}", err);
                        }
                    }
                }
    
                if let Some(v) = loader.get_mut(e) {
                    *v = Loader::Empty;
                }
            }
        }

        self.runtime = Some(runtime_editor.clone());
        if let Some(runtime) = &self.runtime {
            for e in entities.join() {
                if let Some(section) = runtime.sections.get(&e.id()) {
    
                    match sections.insert(e, section.clone()) {
                        Ok(_) => {
                            let mut section = section.clone().with_parent_entity(e.id());
                            if let Some(state) = runtime.runtime.current() {
                                section.state = state.clone();
                            }
    
                            match section_attributes
                                .insert(e, SectionAttributes(section.into_attributes()))
                            {
                                Ok(_) => {
                                    let mut store = Store::<EventComponent>::default();
                                    runtime.events.iter().cloned().for_each(|e| {
                                        store = store.node(e);
                                    });
    
                                    match event_graph.insert(e, EventGraph(store)) {
                                        Ok(_) => {
                                        }
                                        Err(err) => {
                                            eprintln!("RuntimeDispatcher Eror {}", err);
                                        }
                                    }
                                }
                                Err(err) => eprintln!("RuntimeDispatcher Error {}", err),
                            }
                        }
                        Err(err) => eprintln!("RuntimeDispatcher Error {}", err),
                    }
                }
            }
        }
    }
}

impl<S> Default for RuntimeEditor<S>
where
    S: RuntimeState + Component,
{
    fn default() -> Self {
        Self {
            runtime: Default::default(),
            events: Default::default(),
            sections: Default::default(),
            running: (None, None, None),
            dispatch_snapshot: None,
            dispatch_remove: None,
        }
    }
}

impl<S> App for RuntimeEditor<S>
where
    S: RuntimeState + Component,
{
    fn name() -> &'static str {
        "Runtime Editor"
    }

    fn window_size() -> &'static [f32; 2] {
        &[1500.0, 720.0]
    }

    fn show_editor(&mut self, ui: &imgui::Ui) {
        Window::new(Self::name())
            .size(*Self::window_size(), imgui::Condition::Appearing)
            .build(ui, || {
                if CollapsingHeader::new(format!("Current Runtime"))
                    .leaf(true)
                    .begin(ui)
                {
                    ui.separator();
                    match self.running {
                        (Some(v), elapsed, stopped) => {
                            if ui.button("Stop") {
                                self.dispatch_remove = None;
                                self.dispatch_snapshot = None;
                                self.running = (None, None, None);
                            }

                            match (v, elapsed, stopped) {
                                (true, Some(elapsed), None) => {
                                    ui.same_line();
                                    if ui.button("Pause") {
                                        self.running =
                                            (Some(false), Some(elapsed), Some(Instant::now()));
                                    }

                                    ui.text(format!("Running {:#?}", elapsed.elapsed()));

                                    if self.runtime.can_continue() {
                                        self.runtime = self.runtime.step();
                                    } else {
                                        self.running = (None, Some(elapsed), Some(Instant::now()));
                                    }
                                }
                                (false, Some(elapsed), Some(stopped)) => {
                                    ui.same_line();
                                    if ui.button("Continue") {
                                        self.running = (Some(true), Some(elapsed), None);
                                    }

                                    ui.text(format!("Paused {:#?}", stopped.elapsed()));
                                }
                                _ => {}
                            }
                        }
                        (None, Some(elapsed), Some(stopped)) => {
                            if ui.button("Clear") {
                                self.dispatch_remove = None;
                                self.dispatch_snapshot = None;
                                self.running = (None, None, None);
                            }
                            ui.text(format!("Ran for {:#?}", stopped - elapsed));
                        }
                        _ => {}
                    };

                    let context = self.runtime.context();
                    ui.text(format!("Sections Loaded: {}", self.sections.len()));
                    ui.label_text(format!("Current Event"), format!("{}", context));
                    ui.disabled(self.running.0.is_some(), || {
                        if ui.button("Setup") {
                            self.runtime.parse_event("{ setup;; }");
                        }
                        ui.same_line();
                        if ui.button("Start") {
                            self.running = (Some(true), Some(Instant::now()), None);
                        }

                        ui.same_line();
                        if ui.button("Step") {
                            self.runtime = self.runtime.step();
                        }

                        ui.same_line();
                        if ui.button("Clear Attributes") {
                            self.runtime.attributes.clear();
                        }
                    });
                    ui.new_line();
                    ui.separator();
                    if let Some(state) = self.runtime.current() {
                        ui.input_text_multiline(
                            format!("Current State"),
                            &mut format!("{}", state),
                            [0.0, 0.0],
                        )
                        .read_only(true)
                        .build();
                        ui.new_line();
                    }

                    if !self.runtime.attributes.is_empty() {
                        if CollapsingHeader::new(format!("Runtime Attributes"))
                            .leaf(true)
                            .build(ui)
                        {
                            self.runtime.attributes.iter().for_each(|a| {
                                ui.text(format!("{}", a));
                            });
                        }
                    }
                }

                ui.new_line();
                if CollapsingHeader::new("Snapshots").leaf(true).begin(ui) {
                    if ui.button("Take Snapshot of Runtime") {
                        self.dispatch_snapshot = Some(());
                        return;
                    }
                    ui.new_line();

                    self.show_snapshots(ui);
                }
            });
    }
}

impl<S> RuntimeEditor<S>
where
    S: RuntimeState + Component,
{
    fn show_snapshots(&mut self, ui: &imgui::Ui) {
        for (id, section) in self.sections.iter_mut() {
            ui.text(format!("{}: ", id));
            ui.same_line();
            ui.indent();

            section.show_editor(ui);
            if ui.button(format!("Apply {}", section.title)) {
                // This will apply the sections current state and attributes to the current runtime
                let mut clone = self.runtime.clone();
                clone.state = Some(S::from_attributes(section.into_attributes()));
                section.attributes.values().for_each(|a| {
                    clone.attribute(a.clone());
                });
                if let Some(Value::TextBuffer(event)) = section.get_attr_value("context::") {
                    clone = clone.parse_event(event);
                }
                let next = RuntimeEditor::from(clone);
                self.runtime = next.runtime;

                return;
            }

            ui.same_line();
            if ui.button(format!("Remove {}", section.title)) {
                self.dispatch_remove = Some(*id);
                return;
            }
            ui.new_line();
            ui.separator();
            ui.unindent();
        }
    }
}
