use std::{
    collections::{BTreeMap},
    fmt::Display,
};

use atlier::system::{App, Extension, Value};
use imgui::{Window};
use serde::{Deserialize, Serialize};
use specs::{
    storage::DenseVecStorage, Component, Entities, Join, ReadStorage, RunNow, System, WorldExt,
    WriteStorage,
};

use crate::{RuntimeState, AttributeGraph};

use super::{event_graph::EventGraph, unique_title, Section};

#[derive(Default, Clone)]
pub struct EventEditor {
    title: String,
    events: BTreeMap<u32, EventGraph>,
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
        ReadStorage<'a, AttributeGraph>,
        WriteStorage<'a, EventGraph>,
    );

    fn run(&mut self, (entities, attributes, mut event_graph): Self::SystemData) {
        for (e, attrs) in (&entities, attributes.maybe()).join() {
            if let Some(attrs) = attrs {
                match attrs.is_enabled("enable event builder") {
                    Some(true) => {
                        if let None = self.events.get(&e.id()) {
                            if let Some(graph) = event_graph.get_mut(e) {
                                println!("Loading event graph for {:?}", e);
                                self.events.insert(e.id(), graph.clone());
                            } else {
                                println!("graph not found");
                            }
                        }
                    }
                    Some(false) => {
                        if let Some(graph) = self.events.remove(&e.id()) {
                            println!("Saving event graph for {:?}.", e);
                            match event_graph.insert(e, graph) {
                                Ok(_) => {
                                    println!("Event graph saved {:?}.", e);
                                    //println!("old {:?}", v);
                                },
                                Err(err) => {
                                    println!("Could not save event graph {}", err);
                                }
                            }
                        }
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
        let mut next = self.events.clone();
        for (e, graph) in next.iter_mut() {
            Window::new(format!("{} {}", &self.title, e))
                .size([800.0, 600.0], imgui::Condition::Appearing)
                .build(ui, || {
                    if ui.button("Add Event") {
                        graph.add_event(EventComponent::new(
                            unique_title("Event"),
                            "{ new_event;; }",
                        ));
                    }

                    ui.same_line();
                    if ui.button("Refresh") {
                        self.events.clear();
                        return;
                    }

                    graph.edit_as_table(ui);
                });
        }

        self.events = next;
    }
}

impl Display for EventEditor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Event Editor")
    }
}

impl From<AttributeGraph> for EventEditor {
    fn from(_: AttributeGraph) -> Self {
        todo!();
    }
}

impl RuntimeState for EventEditor {
    type Dispatcher = AttributeGraph;
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
#[derive(Debug, Clone, Component, Hash, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
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

impl<S> Into<Section<S>> for &mut EventComponent
where
    S: RuntimeState,
{
    fn into(self) -> Section<S> {
        let section = Section::new(
            self.label.to_string(),
            AttributeGraph::default()
                .with_text("label", self.label.clone())
                .with_text("on", self.on.clone())
                .with_text("dispatch", self.dispatch.clone())
                .with_text("call", self.call.clone())
                .to_owned(),
            |s, ui| {
                s.edit_attr("edit the 'on' property", "on", ui);
                s.edit_attr("edit the 'dispatch' property", "dispatch", ui);
                s.edit_attr("edit the 'call' property", "call", ui);
            },
            S::default(),
        );

        section
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
            s.attributes.find_attr_value("label"),
            s.attributes.find_attr_value("on"),
            s.attributes.find_attr_value("dispatch"),
            s.attributes.find_attr_value("call"),
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
