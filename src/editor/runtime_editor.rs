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
    }

    fn on_window_event(
        &'_ mut self,
       _world: &specs::World,
        event: &'_ atlier::system::WindowEvent<'_>,
    ) {
        match event {
            WindowEvent::DroppedFile(_dropped_file_path) => {
                
            }
            WindowEvent::CloseRequested => {
                
            }
            _ => {}
        }
    }

    fn on_run(&'_ mut self, _world: &specs::World) {
        // This drives status and progress updates of running tasks
        //  List::<Task>::default().on_run(world);
    }
}

impl RuntimeEditor {
    pub fn edit_event_menu(&mut self, _app_world: &specs::World, _ui: &Ui) {

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
