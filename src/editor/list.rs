
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
            let thunk_symbol = context
                .block
                .as_ref()
                .find_text("thunk_symbol")
                .unwrap_or("entity".to_string());

            if ui.collapsing_header(
                format!(
                    "{} {} - {}",
                    thunk_symbol,
                    context.block.block_name,
                    context.as_ref().hash_code()
                ),
                TreeNodeFlags::DEFAULT_OPEN,
            ) {
                for attr in context.as_mut().iter_mut_attributes() {
                    attr.edit_value(format!("{} {}", attr.name(), attr.id()), ui);
                }
                item.on_ui(world, ui);
                ui.text(format!("stable: {}", context.as_ref().is_stable()));
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
