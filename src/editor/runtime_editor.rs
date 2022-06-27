use atlier::system::Extension;
use crate::Runtime;

#[derive(Clone, Default)]
pub struct RuntimeEditor
{
    _runtime: Runtime
}

impl Extension for RuntimeEditor 
{
    fn configure_app_world(_: &mut specs::World) {
       
    }

    fn configure_app_systems(_: &mut specs::DispatcherBuilder) {
    }

    fn on_ui(&'_ mut self, _: &specs::World, _: &'_ imgui::Ui<'_>) {
    }

    fn on_window_event(&'_ mut self, _: &specs::World, event: &'_ atlier::system::WindowEvent<'_>) {
        match event {
            atlier::system::WindowEvent::DroppedFile(_) => {

            },
            _ => {}
        }
    }

    fn on_run(&'_ mut self, _: &specs::World) {
    }
}
