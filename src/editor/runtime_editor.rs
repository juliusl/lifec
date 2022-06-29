use super::{Call, List, Task, Timer};
use crate::{
    plugins::{Engine, Event, Listen, OpenDir, OpenFile, Process, Project, ThunkContext},
    Runtime,
};
use atlier::system::Extension;
use imgui::{Ui, Window};
use specs::{Entity, World, WorldExt};
pub use tokio::sync::broadcast::{channel, Receiver, Sender};

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

impl RuntimeEditor {
    pub fn task_window(&mut self, app_world: &specs::World, ui: &Ui) {
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
        self.task_window(app_world, ui);
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
        match OpenDir::listen(&mut self.runtime, world) {
            Some(file_dir) => {
                for (block_name, block) in Project::from(file_dir).iter_block_mut() {
                    eprintln!("found block {}", block_name);

                    if let Some(file) = block.get_block("file") {
                        if let (Some(file_src), Some(content)) = (
                            file.as_ref().find_text("file_src"),
                            file.as_ref().find_binary("content"),
                        ) {
                            self.schedule(world, &Call::event::<OpenFile>(), |g| {
                                // Setting content will skip reading the file_src, unless refresh is enabled
                                g.as_mut()
                                    .with_binary("content", content)
                                    .add_text_attr("file_src", file_src);
                            });
                        }
                    }
                }
            }
            None => {}
        }

        match OpenFile::listen(&mut self.runtime, world) {
            Some(file) => {
                if self.project_mut().import(file) {
                    eprintln!("Imported file to project");
                }
            },
            None => {
            },
        }
    }
}
