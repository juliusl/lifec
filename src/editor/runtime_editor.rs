use super::{unique_title, Call, List, Task};
use crate::plugins::*;
use crate::*;

use imgui::{Condition, Slider, StyleVar, Ui, Window};
use specs::{Join, World, WorldExt};
pub use tokio::sync::broadcast::{channel, Receiver, Sender};

/// Listener function, called when a thunk completes
type RuntimeEditorListener = fn(&mut RuntimeEditor, world: &World);

/// This struct is an environment and extension point for a lifec Runtime
pub struct RuntimeEditor {
    runtime: Runtime,
    listeners: Vec<RuntimeEditorListener>,
    font_scale: f32,
    enable_complex: bool,
    show_all_engines: bool,
    task_window_size: [f32; 2],
}

/// Allows runtime editor to use `crate::start` method
impl AsRef<Runtime> for RuntimeEditor {
    fn as_ref(&self) -> &Runtime {
        &self.runtime
    }
}

impl RuntimeEditor {
    pub fn new(runtime: Runtime) -> Self {
        let mut new = Self::default();
        new.runtime = runtime;
        new
    }
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
        eprintln!("Current runtime editor listeners {}", self.listeners.len());
    }
}

impl Default for RuntimeEditor {
    fn default() -> Self {
        let mut default = Self {
            runtime: Default::default(),
            listeners: vec![],
            font_scale: 1.0,
            show_all_engines: false,
            enable_complex: false,
            task_window_size: [580.0, 700.0],
        };
        default.runtime.install::<Call, Timer>();
        default.runtime.install::<Call, Remote>();
        default.runtime.install::<Call, Process>();
        default.runtime.install::<Call, OpenDir>();
        default.runtime.install::<Call, OpenFile>();
        default.runtime.install::<Call, WriteFile>();
        default.runtime.install::<Call, Runtime>();
        default.runtime.install::<Call, Println>();
        default.runtime.install::<Call, Expect>();
        default.listen(Self::on_open_file);
        //default.listen(Self::on_open_dir);
        default
    }
}

impl Extension for RuntimeEditor {
    fn configure_app_world(world: &mut specs::World) {
        List::<Task>::configure_app_world(world);
        Task::configure_app_world(world);
        world.register::<Connection>();
        world.register::<Sequence>();
    }

    fn configure_app_systems(dispatcher: &mut specs::DispatcherBuilder) {
        Task::configure_app_systems(dispatcher);
    }

    fn on_ui(&'_ mut self, app_world: &specs::World, ui: &'_ imgui::Ui<'_>) {
        ui.main_menu_bar(|| {
            ui.menu("Windows", || {
                Slider::new("Font scale", 0.5, 4.0).build(ui, &mut self.font_scale);
                ui.separator();

                ui.menu("Tasks Window", || {
                    ui.checkbox("Enable complex view", &mut self.enable_complex);
                    ui.checkbox("Show all engines", &mut self.show_all_engines);
                    ui.separator();

                    let [width, height] = &mut self.task_window_size;
                    Slider::new("Width", 500.0, 1000.0).build(ui, width);
                    Slider::new("Height", 500.0, 1000.0).build(ui, height);
                });
            });
        });

        if self.enable_complex {
            self.task_window(app_world, &mut List::<Task>::edit_block_view(None), ui);
        } else {
            self.task_window(app_world, &mut List::<Task>::simple(false), ui);
        }

        if self.show_all_engines {
            // These are each active engines
            let mut sequence_lists = app_world.write_component::<List<Task>>();
            for sequence in (&mut sequence_lists).join() {
                self.task_window(app_world, sequence, ui);
            }
        }

        // self.runtime.edit_ui(ui);
        // self.runtime.display_ui(ui);
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
                    self.runtime
                        .schedule(world, &Call::event::<OpenDir>(), |tc| {
                            tc.as_mut()
                                .add_text_attr("file_dir", &file_src.trim_matches('"'));
                        })
                        .and_then(|_| Some(()));
                } else {
                    self.runtime
                        .schedule(world, &Call::event::<OpenFile>(), |tc| {
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
        List::<Task>::default().on_run(world);

        // Listen for completed thunks
        for listener in self.listeners.clone().iter() {
            (listener)(self, world);
        }

        let mut rx = world.write_resource::<tokio::sync::mpsc::Receiver<AttributeGraph>>();
        if let Some(graph) = rx.try_recv().ok() {
            let project = Project::from(graph);
            for (block_name, config_block) in project.iter_block() {
                for (symbol, config) in config_block.to_blocks() {
                    if let Some(installed_plugin) = self.runtime.find_plugin::<Call>(&symbol) {
                        if let Some(created) = self.runtime().find_config_block_and_create(
                            world,
                            block_name,
                            BlockContext::from(config),
                            installed_plugin,
                        ) {
                            eprintln!("Received dispatch for `{block_name} {symbol}`, created {:?}", created);
                        }
                    }
                }
            }
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

    // When open dir is called, this will schedule an open_file event for each file in the directory
    // fn on_open_dir(&mut self, world: &World) {
    //     match self.runtime.listen::<OpenDir>(world) {
    //         Some(file_dir) => {
    //             let mut project = Project::from(file_dir.as_ref().clone());
    //             for (block_name, _) in project.iter_block_mut() {
    //                 // TODO, this seems to cause a slight issue
    //                 // if let Some(file) = block.get_block("file") {
    //                 //     if let (Some(file_src), Some(content)) = (
    //                 //         file.as_ref().find_text("file_src"),
    //                 //         file.as_ref().find_binary("content"),
    //                 //     ) {
    //                 //         self.runtime.schedule(world, &Call::event::<OpenFile>(), |g| {
    //                 //             // Setting content will skip reading the file_src, unless refresh is enabled
    //                 //             g.as_mut()
    //                 //                 .with_binary("content", content)
    //                 //                 .add_text_attr("file_src", file_src);
    //                 //         });
    //                 //     }
    //                 // }
    //             }
    //         }
    //         None => {}
    //     }
    // }
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
                    .with_bool("default_open", true)
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
                    .with_bool("default_open", true)
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
                    .with_bool("default_open", true)
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
                    .with_bool("default_open", true)
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
                    .with_bool("default_open", true)
                    .with_text("file_dir", "");
            },
            OpenDir::description(),
            ui,
        );

        self.runtime.create_event_menu_item(
            app_world,
            &Call::event::<WriteFile>(),
            |c| {
                c.block.block_name = unique_title("new_write_file");
                c.as_mut()
                    .with_text("thunk_symbol", WriteFile::symbol())
                    .with_bool("default_open", true)
                    .add_text_attr("file_dst", "");
            },
            WriteFile::description(),
            ui,
        );
    }

    pub fn task_window(&mut self, app_world: &specs::World, task_list: &mut List<Task>, ui: &Ui) {
        let title = task_list.title().unwrap_or("(All)".to_string());

        Window::new(format!("Tasks, engine: {}", title))
            .menu_bar(true)
            .size(self.task_window_size, imgui::Condition::Always)
            .position([1380.0, 400.0], Condition::Appearing)
            .position_pivot([0.5, 0.5])
            .resizable(false)
            .build(ui, || {
                ui.menu_bar(|| {
                    ui.menu("Menu", || {
                        let frame_padding = ui.push_style_var(StyleVar::FramePadding([8.0, 5.0]));
                        self.project_mut().edit_project_menu(ui);
                        ui.separator();

                        self.edit_event_menu(app_world, ui);
                        ui.separator();

                        self.runtime.menu(ui);
                        ui.separator();
                        frame_padding.end();
                    });
                });

                let frame_padding = ui.push_style_var(StyleVar::FramePadding([8.0, 5.0]));

                let window_padding = ui.push_style_var(StyleVar::WindowPadding([16.0, 16.0]));
                ui.set_window_font_scale(self.font_scale);
                task_list.on_ui(app_world, ui);
                ui.new_line();
                frame_padding.end();
                window_padding.end();
            });
    }
}
