use crate::editor::*;
use crate::plugins::*;
use atlier::system::Extension;
use imgui::Ui;

/// List-layout widget
pub struct List<Item>(fn(&mut ThunkContext, &mut Item, &World, &Ui))
where
    Item: Extension + Component;

impl<Item> List<Item>
where
    Item: Extension + Component 
{
    pub fn edit_attr_table() -> Self {
        List::<Item>(|context, item, world, ui| {
            context.as_mut().edit_attr_table(ui);
            item.on_ui(world, ui);
        })
    }

    pub fn edit_attr_form() -> Self {
        List::<Item>(|context, item, world, ui| {
            for attr in context.as_mut().iter_mut_attributes() {
                attr.edit_value(format!("{} {}", attr.name(), attr.id()), ui);
            }
            item.on_ui(world, ui);
        })
    }
}

impl<Item> Default for List<Item>
where
    Item: Extension + Component,
{
    fn default() -> Self {
       Self::edit_attr_form()
    }
}

impl<Item> Extension for List<Item>
where
    Item: Extension + Component,
    <Item as specs::Component>::Storage: Default
{
    fn configure_app_world(world: &mut specs::World) {
        world.register::<ThunkContext>();
        world.register::<Item>();
    }

    fn configure_app_systems(_: &mut specs::DispatcherBuilder) {
    }

    fn on_ui(&'_ mut self, app_world: &specs::World, ui: &'_ imgui::Ui<'_>) {
        let mut contexts = app_world.write_component::<ThunkContext>();
        let mut items = app_world.write_component::<Item>();

        for (context, item) in (&mut contexts, &mut items).join() {
            let List(layout) = self;
            (layout)(context, item, app_world, ui);
            ui.new_line();
        }
    }

    fn on_window_event(
        &'_ mut self,
        _: &specs::World,
        _: &'_ atlier::system::WindowEvent<'_>,
    ) {
    }

    fn on_run(&'_ mut self, _: &specs::World) {
    }
}
