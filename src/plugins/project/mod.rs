use std::collections::BTreeMap;
use std::fmt::Display;
use std::ops::{Deref, DerefMut};

use atlier::system::{App, Attribute, Extension, Value};
use imgui::Window;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use specs::storage::HashMapStorage;
use specs::{
    Component, Entities,  ReadStorage, RunNow, System, WorldExt, Write, WriteStorage, Join,
};

use crate::editor::{EventComponent, Loader};
use crate::{
    editor::EventGraph,
    AttributeGraph,
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
        WriteStorage<'a, Loader>,
        WriteStorage<'a, EventGraph>,
        WriteStorage<'a, Document>,
    );

    fn run(
        &mut self,
        (entities, mut dispatcher, mut loader, mut event_graphs, mut documents): Self::SystemData,
    ) {
        match dispatcher.deref() {
            Dispatch::Empty => {}
            Dispatch::Load(project) => {
                self.project = Some(project.clone());
            }
        }

        if let Some(project) = &self.project {
            for (id, doc) in project.documents.iter() {
                if !entities.is_alive(entities.entity(*id)) {
                    println!("entity wasn't alive yet");
                }

                let ent = entities.create();

                if let Some(_) = event_graphs.insert(ent, doc.events.clone()).ok() {
                    println!("Project inserted graph {:?}", ent);
                }

                let mut attrs = doc.attributes.clone();
                attrs.set_parent_entity(ent);

                if let Some(_) = loader.insert(ent, Loader::LoadSection(attrs)).ok() {
                    println!("Project inserted loader {:?}", ent);
                }

                match documents.insert(ent, doc.clone()) {
                    Ok(_) => {
                        println!("Project inserted document {:?}", ent);
                    }
                    Err(_) => {
                        eprintln!("Error loading document {:?}", ent);
                    }
                }
            }

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
        ReadStorage<'a, AttributeGraph>,
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
            return;
        }

        for (e, a, g) in (&e, &attributes, &event_graph).join() {
            match a.is_enabled("enable project") {
                Some(true) => {
                    if let None = write_documents.get(e) {
                        match write_documents.insert(e, Document::new(e.id(), a.clone(), g.clone()))
                        {
                            Ok(_) => {
                                if let None = self.documents.get(&e.id()) {
                                    self.documents.insert(
                                        e.id(),
                                        Document::new(e.id(), a.clone(), g.clone()),
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
            if ui.button(format!("Save project to .json")) {
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
            if ui.button(format!("Load project from .json")) {
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

            self.documents.clone().iter().for_each(|(id, d)| {
                if let Some(Value::TextBuffer(project_name)) = d
                    .attributes
                    .find_attr_value("project::name::")
                {
                    if ui.button(format!("Save state to .json file {}", project_name)) {
                        match serde_json::to_string(&d.sanitize(*id)) {
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
                        match ron::ser::to_string_pretty(&d.sanitize(*id), PrettyConfig::new()) {
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

    fn add_attribute(&mut self, attribute: &Attribute) {
        if let Some(document) = self.get_document_mut(attribute.id()) {
            document.attributes.copy_attribute(attribute);
        }
    }
}

impl From<AttributeGraph> for Project
{
    fn from(attribute_graph: AttributeGraph) -> Self {
        todo!();
    }
}

impl RuntimeState for Project {
    type Error = ProjectError;

    fn process(&self, _: impl AsRef<str>) -> Result<Self, Self::Error> {
        todo!()
    }

    // fn from_attributes(attrs: Vec<atlier::system::Attribute>) -> Self {
    //     let mut project = Self::default();

    //     for a in attrs.iter().filter(|a| a.name().starts_with("Event")) {
    //         if let Value::BinaryVector(b) = a.value() {
    //             if let Some(event) = ron::de::from_bytes::<EventComponent>(b).ok() {
    //                 project.add_event_node(a.id(), event);
    //             }
    //         }
    //     }

    //     attrs
    //         .iter()
    //         .filter(|a| !a.name().starts_with("Event"))
    //         .for_each(|a| project.add_attribute(a.clone()));

    //     project
    // }

    // fn into_attributes(&self) -> Vec<atlier::system::Attribute> {
    //     let mut attrs = vec![];
    //     for (e, doc) in self.documents.iter() {
    //         if let Some(true) = doc.attributes.is_attr_checkbox("enable project") {
    //             let mut events = doc.events.into_attributes();
    //             events.iter_mut().for_each(|a| {
    //                 a.set_id(*e);
    //             });

    //             attrs.append(&mut events);

    //             doc.attributes
    //                 .get_attrs()
    //                 .iter()
    //                 .cloned()
    //                 .filter(|a| a.id() == *e)
    //                 .for_each(|a| {
    //                     attrs.push(a.clone());
    //                 });
    //         }
    //     }

    //     attrs
    // }

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
            &[],
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
    attributes: AttributeGraph,
    events: EventGraph,
}

impl Document {
    pub fn new(id: u32, attributes: AttributeGraph, events: EventGraph) -> Self {
        Self {
            attributes,
            events,
        }
    }

    /// ensure all attributes are from the same parent id
    fn sanitize(&self, id: u32) -> Self {
        let mut attributes = self.attributes.clone();
        attributes.set_parent_entity_id(id); 

        Self {
            attributes,
            events: self.events.clone(),
        }
    }
}

impl Display for Document {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}
