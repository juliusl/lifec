use std::{
    collections::HashMap,
    fmt::Display,
};

use atlier::system::{App, Extension, Value};
use imgui::Window;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use specs::{Entities, Join, ReadStorage, RunNow, System};

use crate::{
    editor::{EventGraph, SectionAttributes},
    RuntimeState,
};

#[derive(Default, Clone)]
pub struct Project {
    documents: HashMap<u32, Document>,
}

pub enum Dispatch {
    Empty,
    Refresh,
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
    );

    fn run(&mut self, (e, attributes, event_graph): Self::SystemData) {
        for (e, a, g) in (&e, &attributes, &event_graph).join() {
            match a.is_attr_checkbox("enable project") {
                Some(true) => {
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
                Some(false) => {
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
        self.documents.clone().iter().for_each(|(_, d)| {
            Window::new(format!("Project {}", d)).build(ui, || {
                if ui.button("Refresh") {
                    self.documents.clear();
                }

                if let Some(Value::TextBuffer(project_name)) = d
                    .attributes
                    .get_attr("project::name::")
                    .and_then(|a| Some(a.value()))
                {
                    if ui.button(format!("Save state to .json file")) {
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
                    if ui.button(format!("Save state to .ron file")) {
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

impl RuntimeState for Project {
    type Error = ProjectError;

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

impl Extension for Project {
    fn configure_app_world(world: &mut specs::World) {
        world.insert(Dispatch::Empty);
    }

    fn configure_app_systems(_: &mut specs::DispatcherBuilder) {}

    fn extend_app_world(&mut self, app_world: &specs::World, ui: &imgui::Ui) {
        self.run_now(app_world);
        self.show_editor(ui);
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Document {
    attributes: SectionAttributes,
    events: EventGraph,
}

impl Display for Document {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}
