use atlier::system::Extension;
use specs::{Component};
use crate::{
    RuntimeState, Runtime,
};

#[derive(Clone, Default)]
pub struct RuntimeEditor<S>
where
    S: RuntimeState,
{
    _runtime: Runtime<S>
}

impl<S> Extension for RuntimeEditor<S> 
where
    S: RuntimeState + Component,
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
