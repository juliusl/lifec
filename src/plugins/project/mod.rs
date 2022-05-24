use std::collections::BTreeMap;
use std::fmt::Display;
use std::ops::{Deref, DerefMut};

use atlier::system::{App, Attribute, Extension, Value};
use imgui::Window;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use specs::storage::HashMapStorage;
use specs::{
    Component, Entities, Join, ReadStorage, RunNow, System, WorldExt, Write, WriteStorage,
};

use crate::editor::{EventComponent, Loader};
use crate::{
    editor::{EventGraph, SectionAttributes},
    RuntimeState,
};

#[derive(Default, Clone)]
pub struct Project {
    documents: BTreeMap<u32, Document>,
    dispatch_load_project: Option<()>,
}

pub struct ProjectDispatcher {
    project: Option<Project>,
}

impl<'a> System<'a> for ProjectDispatcher {
    type SystemData = (
        Entities<'a>,
        Write<'a, Dispatch>,
        Write<'a, Loader>,
        WriteStorage<'a, SectionAttributes>,
        WriteStorage<'a, EventGraph>,
        WriteStorage<'a, Document>,
    );

    fn run(&mut self, (entities, mut dispatcher, mut loader, mut attributes, mut event_graphs, mut documents): Self::SystemData) {
        match dispatcher.deref() {
            Dispatch::Empty => {}
            Dispatch::Load(project) => {
                self.project = Some(project.clone());
            }
        }

        if let Some(project) = &self.project {
            for (id, doc) in project.documents.iter() {
                let ent = entities.entity(*id);

                if let Some(_) = attributes.insert(ent, doc.attributes.clone()).ok() {
                    if let Some(_) = event_graphs.insert(ent, doc.events.clone()).ok() {
                        println!("loaded project {}", id);
                    }
                }

                match documents.insert(ent, doc.clone()) {
                    Ok(_) => {
                        println!("Document loaded {}", id);
                    },
                    Err(err) => {
                        eprintln!("Error loading document {}", err)
                    },
                }
            }

            let set_loader = loader.deref_mut(); 
            *set_loader = Loader::LoadSection(project.documents.len() as u32);

            self.project = None;
            let unset = dispatcher.deref_mut();
            *unset = Dispatch::Empty;
        }
    }
}

pub enum Dispatch {
    Empty,
    Load(Project),
}

impl Default for Dispatch {
    fn default() -> Self {
        Dispatch::Empty
    }
}

impl<'a> System<'a> for Project {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, SectionAttributes>,
        ReadStorage<'a, EventGraph>,
        WriteStorage<'a, Document>,
        Write<'a, Dispatch>,
    );

    fn run(
        &mut self,
        (e, attributes, event_graph, mut write_documents, mut dispatcher): Self::SystemData,
    ) {
        if let Some(()) = self.dispatch_load_project.take() {
            let dispatch = dispatcher.deref_mut();

            *dispatch = Dispatch::Load(self.clone());
        }

        for (e, a, g) in (&e, &attributes, &event_graph).join() {
            match a.is_attr_checkbox("enable project") {
                Some(true) => {
                    if let None = write_documents.get(e) {
                        match write_documents.insert(
                            e,
                            Document {
                                attributes: a.clone(),
                                events: g.clone(),
                            },
                        ) {
                            Ok(_) => {
                                if let None = self.documents.get(&e.id()) {
                                    self.documents.insert(
                                        e.id(),
                                        Document {
                                            attributes: a.clone(),
                                            events: g.clone(),
                                        },
                                    );
                                }
                            }
                            Err(_) => todo!(),
                        }
                    }

                    if let Some(doc) = self.documents.get_mut(&e.id()) {
                        doc.attributes = a.clone();
                        doc.events = g.clone();

                        if let Some(doc) = write_documents.get_mut(e) {
                            doc.attributes = a.clone();
                            doc.events = g.clone();   
                        }
                    } else {
                        if let Some(doc) = write_documents.get(e) {
                            self.documents.insert(e.id(), doc.clone());
                        } else {
                            self.documents.insert(e.id(), Document::default());
                        }
                    }
                }
                Some(false) => {
                    write_documents.remove(e);
                    self.documents.remove(&e.id());
                }
                _ => (),
            }
        }
    }
}

impl App for Project {
    fn name() -> &'static str {
        "Project"
    }

    fn show_editor(&mut self, ui: &imgui::Ui) {
        Window::new(format!("Projects Enabled: {}", self.documents.len())).build(ui, || {
            ui.same_line();
            if ui.button(format!("Save state")) {
                match self.save() {
                    Some(serialized) => {
                        match std::fs::write(format!("{}.json", "projects"), serialized) {
                            Ok(_) => {
                                println!("saved")
                            }
                            Err(_) => {}
                        }
                    }
                    None => {}
                }
            }

            ui.same_line();
            if ui.button(format!("Load state")) {
                match std::fs::read_to_string(format!("{}.json", "projects")) {
                    Ok(serialized) => {
                        println!("opened");

                        let next = self.load(serialized);
                        *self = self.merge_with(&next);
                        self.dispatch_load_project = Some(());
                    }
                    Err(_) => {}
                }
            }

            self.documents.clone().iter().for_each(|(_, d)| {
                if let Some(Value::TextBuffer(project_name)) = d
                    .attributes
                    .get_attr("project::name::")
                    .and_then(|a| Some(a.value()))
                {
                    if ui.button(format!("Save state to .json file {}", project_name)) {
                        match serde_json::to_string(d) {
                            Ok(serialized) => {
                                match std::fs::write(format!("{}.json", project_name), serialized) {
                                    Ok(_) => {
                                        println!("saved")
                                    }
                                    Err(_) => todo!(),
                                }
                            }
                            Err(_) => todo!(),
                        }
                    }

                    ui.same_line();
                    if ui.button(format!("Save state to .ron file {}", project_name)) {
                        match ron::ser::to_string_pretty(d, PrettyConfig::new()) {
                            Ok(serialized) => {
                                match std::fs::write(format!("{}.ron", project_name), serialized) {
                                    Ok(_) => {
                                        println!("saved")
                                    }
                                    Err(_) => todo!(),
                                }
                            }
                            Err(_) => todo!(),
                        }
                    }
                }
            });
        });
    }
}

pub struct ProjectError;

impl Display for Project {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

impl Project {
    fn get_document_mut(&mut self, id: u32) -> Option<&mut Document> {
        if let None = self.documents.get(&id) {
            self.documents.insert(id, Document::default());
        }

        self.documents.get_mut(&id)
    }

    fn add_event_node(&mut self, id: u32, event: EventComponent) {
        if let Some(document) = self.get_document_mut(id) {
            let EventGraph(store) = &document.events;
            document.events = EventGraph(store.node(event));
        }
    }

    fn add_attribute(&mut self, attribute: Attribute) {
        if let Some(document) = self.get_document_mut(attribute.id()) {
            document.attributes.add_attribute(attribute);
        }
    }
}

impl RuntimeState for Project {
    type Error = ProjectError;

    fn process<S: AsRef<str> + ?Sized>(&self, _: &S) -> Result<Self, Self::Error> {
        todo!()
    }

    fn from_attributes(attrs: Vec<atlier::system::Attribute>) -> Self {
        let mut project = Self::default();

        for a in attrs.iter().filter(|a| a.name().starts_with("Event")) {
            if let Value::BinaryVector(b) = a.value() {
                if let Some(event) = ron::de::from_bytes::<EventComponent>(b).ok() {
                    project.add_event_node(a.id(), event);
                }
            }
        }

        attrs
            .iter()
            .filter(|a| !a.name().starts_with("Event"))
            .for_each(|a| project.add_attribute(a.clone()));

        project
    }

    fn into_attributes(&self) -> Vec<atlier::system::Attribute> {
        let mut attrs = vec![];
        for (e, doc) in self.documents.iter() {
            let mut events = doc.events.into_attributes();
            events.iter_mut().for_each(|a| {
                a.set_id(*e);
            });

            attrs.append(&mut events);

            doc.attributes.get_attrs().iter().cloned().for_each(|a| {
                attrs.push(a.clone());
            });
        }

        attrs
    }

    fn merge_with(&self, other: &Self) -> Self {
        Self {
            documents: other.documents.clone(),
            dispatch_load_project: None,
        }
    }
}

impl Extension for Project {
    fn configure_app_world(world: &mut specs::World) {
        world.register::<Document>();
        world.insert(Dispatch::Empty);
    }

    fn configure_app_systems(builder: &mut specs::DispatcherBuilder) {
        builder.add(
            ProjectDispatcher { project: None },
            "project_dispatcher",
            &["runtime_dispatcher"],
        );
    }

    fn extend_app_world(&mut self, app_world: &specs::World, ui: &imgui::Ui) {
        self.run_now(app_world);
        self.show_editor(ui);
    }
}

#[derive(Default, Clone, Serialize, Deserialize, Component)]
#[storage(HashMapStorage)]
pub struct Document {
    attributes: SectionAttributes,
    events: EventGraph,
}

impl Display for Document {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}