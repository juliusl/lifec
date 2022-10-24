use super::{List, Task};
use crate::prelude::*;

use atlier::system::{WindowEvent, Extension};
use imgui::{Condition, Slider, StyleVar, Ui, Window};
use specs::{Join, WorldExt};
pub use tokio::sync::broadcast::{channel, Receiver, Sender};

/// This struct is an environment and extension point for a lifec Runtime
///
pub struct RuntimeEditor {
    /// Increase or decrease font-scaling of tool windows
    /// 
    font_scale: f32,
    enable_complex: bool,
    show_all_engines: bool,
    task_window_size: [f32; 2],
}

impl Default for RuntimeEditor {
    fn default() -> Self {
        Self {
            font_scale: 1.0,
            enable_complex: false,
            show_all_engines: false,
            task_window_size: [580.0, 700.0],
        }
    }
}

impl Extension for RuntimeEditor {
    fn configure_app_world(world: &mut specs::World) {
        List::<Task>::configure_app_world(world);
        world.register::<Connection>();
        world.register::<Sequence>();
    }

    fn configure_app_systems(dispatcher: &mut specs::DispatcherBuilder) {
        List::<Task>::configure_app_systems(dispatcher);
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
            // self.task_window(app_world, &mut List::<Task>::edit_block_view(None), ui);
        } else {
            self.task_window(app_world, &mut List::<Task>::simple(false), ui);
        }

        // if self.show_all_engines {
        //     // These are each active engines
        //     let mut sequence_lists = app_world.write_component::<List<Task>>();
        //     for sequence in (&mut sequence_lists).join() {
        //         self.task_window(app_world, sequence, ui);
        //     }
        // }
    }

    fn on_window_event(
        &'_ mut self,
        world: &specs::World,
        event: &'_ atlier::system::WindowEvent<'_>,
    ) {
        match event {
            WindowEvent::DroppedFile(dropped_file_path) => {
                // if dropped_file_path.is_dir() {
                //     let path = dropped_file_path.join(".runmd");
                //     if path.exists() {
                //         if let Some(_) = self.load_project(path.to_str().unwrap_or_default()) {
                //             self.create_default(world);
                //         }
                //     }
                // } else if "runmd" == dropped_file_path.extension().unwrap_or_default() {
                //     if let Some(_) =
                //         self.load_project(dropped_file_path.to_str().unwrap_or_default())
                //     {
                //         self.create_engine_parts(world);
                //     }
                // }
            }
            WindowEvent::CloseRequested => {
                // let mut cancel_source = world.write_component::<CancelThunk>();
                // for cancel_thunk in (&world.entities()).join() {
                //     if let Some(cancel_thunk) = cancel_source.remove(cancel_thunk) {
                //         cancel_thunk.0.send(()).ok();
                //     }
                // }
            }
            _ => {}
        }
    }

    fn on_run(&'_ mut self, world: &specs::World) {
        // This drives status and progress updates of running tasks
        //  List::<Task>::default().on_run(world);
    }
}

impl RuntimeEditor {
    pub fn edit_event_menu(&mut self, app_world: &specs::World, ui: &Ui) {
        // self.runtime.create_event_menu_item(
        //     app_world,
        //     &Call::event::<Timer>(),
        //     |c| {
        //         let title = unique_title("new_timer");
        //         c.block.block_name = title.to_string();
        //         c.as_mut()
        //             .with_text("node_title", title)
        //             .with_text("thunk_symbol", Timer::symbol())
        //             .with_bool("default_open", true)
        //             .with_int("duration", 0);
        //     },
        //     Timer::description(),
        //     ui,
        // );

        // self.runtime.create_event_menu_item(
        //     app_world,
        //     &Call::event::<Process>(),
        //     |c| {
        //         let title = unique_title("new_process");
        //         c.block.block_name = title.to_string();
        //         c.as_mut()
        //             .with_text("node_title", title)
        //             .with_text("thunk_symbol", Process::symbol())
        //             .with_bool("default_open", true)
        //             .with_text("command", "");
        //     },
        //     Process::description(),
        //     ui,
        // );

        // self.runtime.create_event_menu_item(
        //     app_world,
        //     &Call::event::<Remote>(),
        //     |c| {
        //         let title = unique_title("new_remote");
        //         c.block.block_name = title.to_string();
        //         c.as_mut()
        //             .with_text("node_title", title)
        //             .with_text("thunk_symbol", Remote::symbol())
        //             .with_bool("default_open", true)
        //             .with_text("command", "");
        //     },
        //     Remote::description(),
        //     ui,
        // );

        // self.runtime.create_event_menu_item(
        //     app_world,
        //     &Call::event::<OpenFile>(),
        //     |c| {
        //         let title = unique_title("new_open_file");
        //         c.block.block_name = title.to_string();
        //         c.as_mut()
        //             .with_text("node_title", title)
        //             .with_text("thunk_symbol", OpenFile::symbol())
        //             .with_bool("default_open", true)
        //             .with_text("file_src", "");
        //     },
        //     OpenFile::description(),
        //     ui,
        // );

        // self.runtime.create_event_menu_item(
        //     app_world,
        //     &Call::event::<OpenDir>(),
        //     |c| {
        //         let title = unique_title("new_open_dir");
        //         c.block.block_name = title.to_string();
        //         c.as_mut()
        //             .with_text("node_title", title)
        //             .with_text("thunk_symbol", OpenDir::symbol())
        //             .with_bool("default_open", true)
        //             .with_text("file_dir", "");
        //     },
        //     OpenDir::description(),
        //     ui,
        // );

        // self.runtime.create_event_menu_item(
        //     app_world,
        //     &Call::event::<WriteFile>(),
        //     |c| {
        //         let title = unique_title("new_write_file");
        //         c.block.block_name = title.to_string();
        //         c.as_mut()
        //             .with_text("node_title", title)
        //             .with_text("thunk_symbol", WriteFile::symbol())
        //             .with_bool("default_open", true)
        //             .add_text_attr("file_dst", "");
        //     },
        //     WriteFile::description(),
        //     ui,
        // );

        // self.runtime.create_event_menu_item(
        //     app_world,
        //     &Call::event::<Println>(),
        //     |c| {
        //         let title = unique_title("new_println");
        //         c.block.block_name = title.to_string();
        //         c.as_mut()
        //             .with_text("node_title", title)
        //             .with_text("thunk_symbol", Println::symbol())
        //             .with_bool("default_open", true)
        //             .add_text_attr("file_dst", "");
        //     },
        //     Println::description(),
        //     ui,
        // );
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
                        // self.project_mut().edit_project_menu(ui);
                        // ui.separator();

                        self.edit_event_menu(app_world, ui);
                        ui.separator();

                        // self.runtime.menu(ui);
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
