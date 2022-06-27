use atlier::system::Extension;
use imgui::Window;
use specs::{WorldExt, World, Entity};
use crate::{Runtime, plugins::{Process, OpenFile, Engine, ThunkContext, Event}};
use super::{List, Task, Call, Timer};

#[derive(Clone)]
pub struct RuntimeEditor
{
    runtime: Runtime,
}

impl RuntimeEditor {
    /// Schedule an event on the runtime
    pub fn schedule(&mut self, world: &World, event: &Event, config: impl FnOnce(&mut ThunkContext)) -> Option<Entity> {
        if let Some(entity) = self.runtime.create(world, event, |_|{}) {
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
}

impl Default for RuntimeEditor {
    fn default() -> Self {
        let mut default = Self { runtime: Default::default() };
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
    }

    fn on_window_event(&'_ mut self, world: &specs::World, event: &'_ atlier::system::WindowEvent<'_>) {
        match event {
            atlier::system::WindowEvent::DroppedFile(file) => {
                let file_src = format!("{:?}", &file);

                self.schedule(world,&Call::event::<OpenFile>(),  |tc| {
                    tc.as_mut().add_text_attr("file_src", file_src.trim_matches('"'));
                }).and_then(|e| 
                    // TODO retrieve result and push as message
                    Some(println!("open file scheduled for {:?}", e))
                );
            },
            _ => {}
        }
    }

    fn on_run(&'_ mut self, _: &specs::World) {
    }
}
