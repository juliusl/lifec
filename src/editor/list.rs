use crate::editor::*;
use crate::plugins::*;
use atlier::system::Extension;
use imgui::TreeNodeFlags;
use imgui::Ui;

/// List-layout widget for thunk_context's
pub struct List<Item>(fn(&mut ThunkContext, &mut Item, &World, &Ui))
where
    Item: Extension + Component;

impl<Item> List<Item>
where
    Item: Extension + Component,
{
    pub fn edit_attr_short_table() -> Self {
        List::<Item>(|context, item, world, ui| {
            context.as_mut().edit_attr_short_table(ui);
            item.on_ui(world, ui);
        })
    }

    pub fn edit_attr_table() -> Self {
        List::<Item>(|context, item, world, ui| {
            context.as_mut().edit_attr_table(ui);
            item.on_ui(world, ui);
        })
    }

    pub fn edit_attr_form() -> Self {
        List::<Item>(|context, item, world, ui| {
            if ui.collapsing_header(
                format!("Task: {}", context.as_ref().hash_code()),
                TreeNodeFlags::DEFAULT_OPEN,
            ) {
                for attr in BlockContext::iter_block_attrs_mut(context.as_mut()) {
                    attr.edit_value(format!("{} {}", attr.name(), attr.id()), ui);
                }
                item.on_ui(world, ui);
                ui.separator();
            }
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
    <Item as specs::Component>::Storage: Default,
{
    fn configure_app_world(world: &mut specs::World) {
        world.register::<ThunkContext>();
        world.register::<Item>();
    }

    fn on_ui(&'_ mut self, app_world: &specs::World, ui: &'_ imgui::Ui<'_>) {
        let mut contexts = app_world.write_component::<ThunkContext>();
        let mut items = app_world.write_component::<Item>();

        for (context, item) in (&mut contexts, &mut items).join() {
            let List(layout) = self;
            (layout)(context, item, app_world, ui);
        }
    }
}
