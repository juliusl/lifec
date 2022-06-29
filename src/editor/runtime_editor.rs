use super::{Call, List, Task, Timer, unique_title, Interpret};
use crate::{
    plugins::{Engine, Listen, OpenDir, OpenFile, Process, Project, Plugin},
    Runtime, AttributeGraph, RuntimeDispatcher,
};
use atlier::system::{Extension, Value};
use imgui::{Ui, Window};
use specs::World;
pub use tokio::sync::broadcast::{channel, Receiver, Sender};

pub struct RuntimeEditor {
    runtime: Runtime,
}

impl RuntimeEditor {
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
        default.runtime.install::<Interpret, OpenFile>();

        default.project_mut().as_mut().add_text_attr("next_dispatch", "");
        default
    }
}

impl RuntimeEditor {
    pub fn edit_event_menu(&mut self, app_world: &specs::World, ui: &Ui) {
        self.runtime.create_event_menu_item(
            app_world,
            &Call::event::<Timer>(),
            |c| {
                c.block.block_name = unique_title("new_timer");
                c.as_mut()
                .with_text("thunk_symbol", Timer::symbol())
                .with_int("duration", 0);
            },
            Timer::description(),
            ui,
        );

        self.runtime.create_event_menu_item(
            app_world,
            &Call::event::<Process>(),
            |c| {
                c.block.block_name = unique_title("new_process");
                c.as_mut()
                .with_text("thunk_symbol", Process::symbol())
                .with_text("command", "");
            },
            Process::description(),
            ui,
        );

        self.runtime.create_event_menu_item(
            app_world,
            &Call::event::<OpenFile>(),
            |c| {
                c.block.block_name = unique_title("new_open_file");
                c.as_mut()
                .with_text("thunk_symbol", OpenFile::symbol())
                .with_text("file_src", "");
            },
            OpenFile::description(),
            ui,
        );

        self.runtime.create_event_menu_item(
            app_world,
            &Call::event::<OpenDir>(),
            |c| {
                c.block.block_name = unique_title("new_open_dir");
                c.as_mut()
                    .with_text("thunk_symbol", OpenDir::symbol())
                    .with_text("file_dir", "");
            },
            OpenDir::description(),
            ui,
        );

        self.runtime.create_event_menu_item(
            app_world,
            &Interpret::event::<OpenFile>(),
            |c| {
                c.block.block_name = unique_title("new_interpret_file");
                c.as_mut()
                    .with_text("thunk_symbol", "interpret_file")
                    .with_bool("interpret", true)
                    .with_text("file_src", "");
            },
            format!(
                "{}\nIf an interpreter exists for the file type, interprets the file and stores the result in an attribute.", 
                OpenFile::description()
            ),
            ui,
        );
    }

    pub fn task_window(&mut self, app_world: &specs::World, ui: &Ui) {
        Window::new("Tasks")
            .menu_bar(true)
            .size([800.0, 600.0], imgui::Condition::Appearing)
            .build(ui, || {
                ui.menu_bar(|| {
                    self.project_mut().edit_project_menu(ui);
                    self.edit_event_menu(app_world, ui);
                });

                List::<Task>::default().on_ui(app_world, ui);
                ui.new_line();
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
                    self.runtime.schedule(world, &Call::event::<OpenDir>(), |tc| {
                        tc.as_mut()
                            .add_text_attr("file_dir", &file_src.trim_matches('"'));
                    })
                    .and_then(|_| Some(()));
                } else {
                    self.runtime.schedule(world, &Call::event::<OpenFile>(), |tc| {
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
        self.on_open_dir(world);
        self.on_open_file(world);
    }
}

impl RuntimeEditor {
    fn on_open_file(&mut self, world: &World) {
        match OpenFile::listen(&mut self.runtime, world) {
            Some(file) => {
                if self.project_mut().import(file) {
                    eprintln!("Imported file to project");
                }
            }
            None => {}
        }
    }

    fn on_open_dir(&mut self, world: &World) {
        match OpenDir::listen(&mut self.runtime, world) {
            Some(file_dir) => {
                let mut project = Project::from(file_dir.clone());
                for (block_name, block) in project.iter_block_mut() {
                    eprintln!("found block {}", block_name);
                    if let Some(file) = block.get_block("file") {
                        if let (Some(file_src), Some(content)) = (
                            file.as_ref().find_text("file_src"),
                            file.as_ref().find_binary("content"),
                        ) {
                            self.runtime.schedule(world, &Call::event::<OpenFile>(), |g| {
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
    }
}
