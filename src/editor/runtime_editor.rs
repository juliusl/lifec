use atlier::system::Extension;
use imgui::Window;
use crate::{Runtime, plugins::{Process, OpenFile}};
use super::{List, Task, Call, Timer};

#[derive(Clone)]
pub struct RuntimeEditor
{
    runtime: Runtime,
    files: Vec<String>
}

impl Default for RuntimeEditor {
    fn default() -> Self {
        let mut default = Self { runtime: Default::default(), files: Default::default() };
        default.runtime.install::<Call, Timer>();
        default.runtime.install::<Call, Process>();
        default.runtime.install::<Call, OpenFile>();
        default
    }
}

impl Extension for RuntimeEditor 
{
    fn configure_app_world(world: &mut specs::World) {
        List::<Task>::configure_app_world(world);
        Task::configure_app_world(world);
    }

    fn configure_app_systems(dispatcher: &mut specs::DispatcherBuilder) {
        Task::configure_app_systems(dispatcher);
    }

    fn on_ui(&'_ mut self, app_world: &specs::World, ui: &'_ imgui::Ui<'_>) {
        Window::new("Tasks").size([800.0, 600.0], imgui::Condition::Appearing).build(ui, ||{
            List::<Task>::default().on_ui(app_world, ui);
        });

        Window::new("Files").size([800.0, 600.0], imgui::Condition::Appearing).build(ui, ||{
            for file in self.files.iter() {
                ui.text(file);
            }
        });
    }

    fn on_window_event(&'_ mut self, _: &specs::World, event: &'_ atlier::system::WindowEvent<'_>) {
        match event {
            atlier::system::WindowEvent::DroppedFile(file) => {
                self.files.push(format!("{:?}", &file));
            },
            _ => {}
        }
    }

    fn on_run(&'_ mut self, _: &specs::World) {
    }
}
