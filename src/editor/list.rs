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

    pub fn edit_block_view() -> Self {
        List::<Item>(|context, item, world, ui| {
            let thunk_symbol = context
                .block
                .as_ref()
                .find_text("thunk_symbol")
                .unwrap_or("entity".to_string());
            let item_index = context
                .as_ref()
                .find_attr("item::index")
                .and_then(|h| h.transient())
                .and_then(|(_, v)| match v {
                    Value::Int(v) => Some(*v),
                    _ => None,
                })
                .unwrap_or_default();
            
            if ui.collapsing_header(
                format!(
                    "{} {} - {}",
                    thunk_symbol,
                    context.block.block_name,
                    context.as_ref().hash_code()
                ),
                TreeNodeFlags::DEFAULT_OPEN,
            ) {
                ui.input_text(format!("name {}", context.as_ref().entity()), &mut context.block.block_name).build();
                let mut current_id = context.as_ref().entity();

                let clone = context.as_ref().clone();
                for attr in context.as_mut().iter_mut_attributes() {
                    if current_id != attr.id() {
                        ui.new_line();

                        if let Some(next_block) = clone.find_imported_graph(attr.id()) {
                            let thunk_symbol =
                                next_block.find_text("thunk_symbol").unwrap_or_default();
                            let block_symbol =
                                next_block.find_text("block_symbol").unwrap_or_default();
                            ui.text(format!("{} - {}", thunk_symbol, block_symbol));
                        }

                        current_id = attr.id();
                    }

                    if attr.is_stable() {
                        attr.edit_value(
                            format!("{} {}:{:#04x}", attr.name(), attr.id(), item_index as u16),
                            ui,
                        );
                    }
                }
                item.on_ui(world, ui);
                ui.text(format!("stable: {}", context.as_ref().is_stable()));
                ui.separator();
            }
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

        let mut item_index = 0;

        for (context, item) in (&mut contexts, &mut items).join() {
            let List(layout) = self;
            context
                .as_mut()
                .define("item", "index")
                .edit_as(Value::Int(item_index));

            (layout)(context, item, app_world, ui);

            item_index += 1;
        }
    }

    fn on_run(&'_ mut self, app_world: &World) {
        let mut items = app_world.write_component::<Item>();

        for item in (&mut items).join() {
            item.on_run(app_world);
        }
    }
}
