use std::{path::PathBuf, str::from_utf8};

use super::{Call, List, Task, Timer};
use crate::{
    plugins::{Engine, Event, OpenDir, OpenFile, Process, Project, ThunkContext},
    AttributeGraph, Runtime, RuntimeDispatcher,
};
use atlier::system::{Extension, Value};
use imgui::Window;
use specs::{Entity, World, WorldExt};
pub use tokio::sync::broadcast::{channel, Receiver, Sender};

#[derive(Clone)]
pub struct RuntimeEditor {
    runtime: Runtime,
}

impl RuntimeEditor {
    /// Schedule an event on the runtime
    pub fn schedule(
        &mut self,
        world: &World,
        event: &Event,
        config: impl FnOnce(&mut ThunkContext),
    ) -> Option<Entity> {
        if let Some(entity) = self.runtime.create(world, event, |_| {}) {
            let mut contexts = world.write_component::<ThunkContext>();
            let mut events = world.write_component::<Event>();
            if let Some(tc) = contexts.get_mut(entity) {
                config(tc);
                if let Some(event) = events.get_mut(entity) {
                    event.fire(tc.clone());
                    return Some(entity);
                }
            }
        }

        None
    }

    /// returns the project for updating
    pub fn project_mut(&mut self) -> &mut Project {
        &mut self.runtime.project
    }

    pub fn project(&self) -> &Project {
        &self.runtime.project
    }
}

impl Default for RuntimeEditor {
    fn default() -> Self {
        let mut default = Self {
            runtime: Default::default(),
        };
        default.runtime.install::<Call, Timer>();
        default.runtime.install::<Call, Process>();
        default.runtime.install::<Call, OpenFile>();
        default.runtime.install::<Call, OpenDir>();
        default
    }
}

impl Extension for RuntimeEditor {
    fn configure_app_world(world: &mut specs::World) {
        List::<Task>::configure_app_world(world);
        Task::configure_app_world(world);
    }

    fn configure_app_systems(dispatcher: &mut specs::DispatcherBuilder) {
        Task::configure_app_systems(dispatcher);
    }

    fn on_ui(&'_ mut self, app_world: &specs::World, ui: &'_ imgui::Ui<'_>) {
        Window::new("Tasks")
            .menu_bar(true)
            .size([800.0, 600.0], imgui::Condition::Appearing)
            .build(ui, || {
                ui.menu_bar(|| {
                    self.project_mut().edit_project_menu(ui);
                });

                List::<Task>::default().on_ui(app_world, ui);
            });
    }

    fn on_window_event(
        &'_ mut self,
        world: &specs::World,
        event: &'_ atlier::system::WindowEvent<'_>,
    ) {
        match event {
            atlier::system::WindowEvent::DroppedFile(file) => {
                let file_src = format!("{:?}", &file);

                if file.is_dir() {
                    self.schedule(world, &Call::event::<OpenDir>(), |tc| {
                        tc.as_mut()
                            .add_text_attr("file_dir", &file_src.trim_matches('"'));
                    })
                    .and_then(|_| Some(()));
                } else {
                    self.schedule(world, &Call::event::<OpenFile>(), |tc| {
                        tc.as_mut()
                            .add_text_attr("file_src", &file_src.trim_matches('"'));
                    })
                    .and_then(|_| Some(()));
                }
            }
            _ => {}
        }
    }

    fn on_run(&'_ mut self, world: &specs::World) {
        if let Some(next) = Event::receive(world) {
            // listen for file_dir events, and ingest files
            if let Some(file_dir) = next.as_ref().find_text("file_dir") {
                let mut file_src = PathBuf::from(file_dir);
                for (file_name, content) in next.as_ref().find_symbol_values("file") {
                    let file_name = file_name.trim_end_matches("::file");
                    file_src.set_file_name(file_name);
                    
                    let file_src = file_src.to_str().unwrap_or_default();
                    if let Value::BinaryVector(vec) = content {
                        if let Some(content) = from_utf8(&vec).ok() {
                            let mut unwrapping = AttributeGraph::from(0);
                            if unwrapping.batch_mut(content).is_ok() {
                                if let Some(content) = unwrapping.find_file("content") {
                                    self.schedule(world, &Call::event::<OpenFile>(), |g| {
                                        // Setting content will skip reading the file_src, unless refresh is enabled
                                        g.as_mut().add_binary_attr("content", content);
                                        g.as_mut().add_text_attr("file_src", file_src);
                                    });
                                }
                            }
                        }
                    }
                }
            }

            self.project_mut()
                .import_block(next.block);
        }
    }
}
