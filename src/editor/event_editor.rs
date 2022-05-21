use std::{collections::HashMap, fmt::Display};

use atlier::system::{Value, Extension, App};
use imgui::CollapsingHeader;
use specs::{Component, storage::DenseVecStorage, System, ReadStorage, Entities, Join, RunNow};

use crate::RuntimeState;

use super::{Section, SectionAttributes, unique_title};

#[derive(Default, Clone)]
pub struct EventEditor {
    events: HashMap<u32, Vec<EventComponent>>,
}

impl EventEditor {
    pub fn new() -> Self {
        Self { events: HashMap::new() }
    }
}

impl<'a> System<'a> for EventEditor {
    type SystemData = (Entities<'a>, ReadStorage<'a, SectionAttributes>);

    fn run(&mut self, (entities, attributes): Self::SystemData) {
        for e in entities.join() {
            if let Some(attrs) = attributes.get(e) {
                if let Some(true) = attrs.is_attr_checkbox("enable event builder") {
                    if let None = self.events.get(&e.id()) {
                        self.events.insert(e.id(), vec![]);
                    }
                }
            }
        }
    }
}

impl App for EventEditor {
    fn name() -> &'static str {
        "Event Editor"
    }

    fn show_editor(&mut self, ui: &imgui::Ui) {
        for (e, events) in self.events.iter_mut() {
            if CollapsingHeader::new(format!("Events for Entity: {}", e)).build(ui) {
                if ui.button("Add Event") {
                    events.push(EventComponent::new(unique_title("Event"), "{ new_event;; }"));
                }
        
                for (id, e) in events.iter_mut().enumerate() {
                    let mut section: Section::<EventEditor> = e.into();
                    let section = &mut section;
                    section.with_parent_entity(id as u32).show_editor(ui);
                    *e = EventComponent::from(section);
                }
            }
        }
    }
}

pub struct EventEditorError;

impl Display for EventEditor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Event Editor")
    }
}

impl RuntimeState for EventEditor {
    type Error = EventEditorError;

    fn load<S: AsRef<str> + ?Sized>(&self, _: &S) -> Self
    where
        Self: Sized {
        todo!()
    }

    fn process<S: AsRef<str> + ?Sized>(&self, _: &S) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl Extension for EventEditor {
    fn configure_app_world(_: &mut specs::World) {
    }

    fn configure_app_systems(_: &mut specs::DispatcherBuilder) {
    }

    fn extend_app_world(&mut self, app_world: &specs::World, ui: &imgui::Ui) {
        self.run_now(app_world);
        self.show_editor(ui);
    }
}

/// Event component is the the most basic data unit of the runtime
#[derive(Clone, Component)]
#[storage(DenseVecStorage)]
pub struct EventComponent {
    pub label: String,
    pub on: String,
    pub dispatch: String,
    pub call: String,
    pub transitions: Vec<String>,
    // pub flags: BTreeMap<String, String>,
    // pub variales: BTreeMap<String, String>,
}

impl EventComponent {
    pub fn new(label: impl AsRef<str>, on: impl AsRef<str>) -> Self {
        Self {
            label: label.as_ref().to_string(),
            on: on.as_ref().to_string(),
            dispatch: String::default(),
            call: String::default(),
            transitions: vec![]
        }
    }
}

impl<S> Into<Section<S>> for &mut EventComponent where S: RuntimeState {
    fn into(self) -> Section<S> {
        Section::<S>::new(
            self.label.to_string(),
            |s, ui| {
                s.edit_attr("edit the 'on' property", "on", ui);
                s.edit_attr("edit the 'dispatch' property", "dispatch", ui);
                s.edit_attr("edit the 'call' property", "call", ui);
            },
            S::default(),
        )
        .with_text("label", self.label.clone())
        .with_text("on", self.on.clone())
        .with_text("dispatch", self.dispatch.clone())
        .with_text("call", self.call.clone())
    }
}

impl<S> From<&mut Section<S>> for EventComponent where S: RuntimeState {
    fn from(s: &mut Section<S>) -> Self {
        if let (
            Some(Value::TextBuffer(label)), 
            Some(Value::TextBuffer(on)), 
            Some(Value::TextBuffer(dispatch)), 
            Some(Value::TextBuffer(call))) = (
            s.get_attr_value("label"), 
            s.get_attr_value("on"),
            s.get_attr_value("dispatch"),
            s.get_attr_value("call"),  
        ){
            let label = label.to_string();
            let on = on.to_string();
            let dispatch = dispatch.to_string();
            let call = call.to_string();
            Self {
                label,
                on,
                dispatch,
                call,
                transitions: Vec::default()
            }
        } else {
            Self {
                label: String::default(),
                on: String::default(),
                dispatch: String::default(),
                call: String::default(),
                transitions: Vec::default(),
            }
        }
    }
}