use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
};

use atlier::system::{App, Extension, Value};
use imgui::Window;
use serde::{Deserialize, Serialize};
use specs::{
    storage::DenseVecStorage, Component, Entities, Join, ReadStorage, RunNow, System, WorldExt,
    WriteStorage,
};

use crate::{Event, RuntimeState};

use super::{event_graph::EventGraph, unique_title, Section, SectionAttributes};

#[derive(Default, Clone)]
pub struct EventEditor {
    title: String,
    events: BTreeMap<u32, BTreeSet<EventComponent>>,
}

impl EventEditor {
    pub fn new() -> Self {
        Self {
            title: unique_title(Self::name()),
            events: BTreeMap::new(),
        }
    }
}

impl<'a> System<'a> for EventEditor {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, SectionAttributes>,
        WriteStorage<'a, EventGraph>,
    );

    fn run(&mut self, (entities, attributes, mut event_graph): Self::SystemData) {
        for e in entities.join() {
            if let Some(attrs) = attributes.get(e) {
                match attrs.is_attr_checkbox("enable event builder") {
                    Some(true) => {
                        if let None = self.events.get(&e.id()) {
                            if let Some(EventGraph(graph)) = event_graph.get_mut(e) {
                                let mut events = BTreeSet::new();
                                graph.nodes().iter().cloned().for_each(|e| {
                                    events.insert(e.to_owned());
                                });

                                if events.len() > 0 {
                                    self.events.insert(e.id(), events);
                                }
                            }
                        }
                    }
                    Some(false) => {
                        self.events.remove(&e.id());
                    }
                    _ => (),
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
        for (e, events) in self.events.clone().iter_mut() {
            Window::new(format!("{} {}", &self.title, e))
                .size([800.0, 600.0], imgui::Condition::Appearing)
                .build(ui, || {
                    if ui.button("Add Event") {
                        events.insert(EventComponent::new(
                            unique_title("Event"),
                            "{ new_event;; }",
                        ));
                    }

                    ui.same_line();
                    if ui.button("Refresh") {
                        self.events.clear();
                        return;
                    }

                    let mut next_set = BTreeSet::new();
                    for (id, mut e) in events.iter().cloned().enumerate() {
                        let e = &mut e;
                        let mut section: Section<EventEditor> = e.into();
                        let section = &mut section;
                        section.with_parent_entity(id as u32).show_editor(ui);
                        let e = EventComponent::from(section);
                        next_set.insert(e);
                    }

                    self.events.insert(*e, next_set);
                });
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
        Self: Sized,
    {
        todo!()
    }

    fn process<S: AsRef<str> + ?Sized>(&self, _: &S) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl Extension for EventEditor {
    fn configure_app_world(w: &mut specs::World) {
        w.register::<EventGraph>();
    }

    fn configure_app_systems(_: &mut specs::DispatcherBuilder) {}

    fn extend_app_world(&mut self, app_world: &specs::World, ui: &imgui::Ui) {
        self.run_now(app_world);
        self.show_editor(ui);
    }
}

/// Event component is the the most basic data unit of the runtime
#[derive(Clone, Component, Hash, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
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
            transitions: vec![],
        }
    }
}

impl From<Event> for EventComponent {
    fn from(_: Event) -> Self {
        todo!()
    }
}

impl<S> Into<Section<S>> for &mut EventComponent
where
    S: RuntimeState,
{
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

impl<S> From<&mut Section<S>> for EventComponent
where
    S: RuntimeState,
{
    fn from(s: &mut Section<S>) -> Self {
        if let (
            Some(Value::TextBuffer(label)),
            Some(Value::TextBuffer(on)),
            Some(Value::TextBuffer(dispatch)),
            Some(Value::TextBuffer(call)),
        ) = (
            s.get_attr_value("label"),
            s.get_attr_value("on"),
            s.get_attr_value("dispatch"),
            s.get_attr_value("call"),
        ) {
            let label = label.to_string();
            let on = on.to_string();
            let dispatch = dispatch.to_string();
            let call = call.to_string();
            Self {
                label,
                on,
                dispatch,
                call,
                transitions: Vec::default(),
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
