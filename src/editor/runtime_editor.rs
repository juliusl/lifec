use super::{Call, List, Task, unique_title};
use crate::{
    plugins::{Engine, Timer, OpenDir, OpenFile, Process, Remote, Project, Plugin, Sequence},
    Runtime
};
use atlier::system::Extension;
use imgui::{Ui, Window};
use specs::{World, WorldExt};
pub use tokio::sync::broadcast::{channel, Receiver, Sender};

/// Listener function, called when a thunk completes
type RuntimeEditorListener = fn (&mut RuntimeEditor, world: &World);

/// This struct is an environment and extension point for a lifec Runtime
pub struct RuntimeEditor {
    runtime: Runtime,
    listeners: Vec<RuntimeEditorListener>
}

impl RuntimeEditor {
    /// Returns a mutable version of the current project
    pub fn project_mut(&mut self) -> &mut Project {
        &mut self.runtime.project
    }

    /// Returns a ref to the current project
    pub fn project(&self) -> &Project {
        &self.runtime.project
    }

    /// Returns a ref to the current runtime
    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    /// Returns a mutable version of the current runtime
    pub fn runtime_mut(&mut self) -> &mut Runtime {
        &mut self.runtime
    }

    /// Listen for thunk contexts from thunks that have completed their task
    pub fn listen(&mut self, listen: RuntimeEditorListener) {
        self.listeners.push(listen);
    }
}

impl Default for RuntimeEditor {
    fn default() -> Self {
        let mut default = Self {
            runtime: Default::default(),
            listeners: vec![
                Self::on_open_dir,
                Self::on_open_file,
            ]
        };
        default.runtime.install::<Call, Timer>();
        default.runtime.install::<Call, Process>();
        default.runtime.install::<Call, Remote>();
        default.runtime.install::<Call, OpenFile>();
        default.runtime.install::<Call, OpenDir>();
        default
    }
}

impl Extension for RuntimeEditor {
    fn configure_app_world(world: &mut specs::World) {
        List::<Task>::configure_app_world(world);
        Task::configure_app_world(world);
        world.register::<Sequence>();
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
        // This drives status and progress updates of running tasks
        List::<Task>::default()
            .on_run(world);
  
        // Listen for completed thunks
        for listener in self.listeners.clone().iter() {
            (listener)(self, world);
        }
    }
}

impl RuntimeEditor {
    /// When open file is called, this will import the file to the current project
    fn on_open_file(&mut self, world: &World) {
        match self.runtime.listen::<OpenFile>(world) {
            Some(file) => {
                if self.project_mut().import(file.as_ref().clone()) {
                    eprintln!("Imported file to project");
                }
            }
            None => {}
        }
    }

    /// When open dir is called, this will schedule an open_file event for each file in the directory
    fn on_open_dir(&mut self, world: &World) {
        match self.runtime.listen::<OpenDir>(world) {
            Some(file_dir) => {
                let mut project = Project::from(file_dir.as_ref().clone());
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
            &Call::event::<Remote>(),
            |c| {
                c.block.block_name = unique_title("new_remote");
                c.as_mut()
                .with_text("thunk_symbol", Remote::symbol())
                .with_text("command", "");
            },
            Remote::description(),
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

                List::<Task>::edit_block_view().on_ui(app_world, ui);
                ui.new_line();
            });
    }
}
