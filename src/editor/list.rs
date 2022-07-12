use crate::editor::*;
use crate::plugins::*;
use crate::*;
use imgui::MenuItem;
use imgui::TableFlags;
use imgui::TreeNodeFlags;
use imgui::Ui;

/// List-layout widget for thunk_context's
#[derive(Component)]
#[storage(DefaultVecStorage)]
pub struct List<Item>(
    /// Layout fn
    fn(&mut ThunkContext, &mut Item, &World, &Ui),
    /// Sequence to follow
    Option<Sequence>,
    /// Title
    Option<String>,
    /// Column Names
    Option<Vec<&'static str>>,
    /// Show all (verbose)
    bool,
)
where
    Item: Extension + Component;

impl<Item> List<Item>
where
    Item: Extension + Component,
{
    /// Returns the current sequence set for this list
    pub fn sequence(&self) -> Option<Sequence> {
        self.1.clone()
    }

    /// Returns the title of the list
    pub fn title(&self) -> Option<String> {
        if let Some(title) = &self.2 {
            Some(title.to_string())
        } else {
            None
        }
    }

    /// Sets the title of this list
    pub fn set_title(&mut self, title: impl AsRef<str>) {
        self.2 = Some(title.as_ref().to_string());
    }

    /// Returns a simple list view
    pub fn simple(show_all: bool) -> Self {
        List::<Item>(
            |context, item, world, ui| {
                let thunk_symbol = context
                    .block
                    .as_ref()
                    .find_text("thunk_symbol")
                    .unwrap_or("entity".to_string());

                ui.text(format!(
                    "{} {} - {}:",
                    thunk_symbol,
                    context.block.block_name,
                    context.as_ref().hash_code()
                ));

                if let Some(description) = context.as_ref().find_text("description") {
                    ui.new_line();
                    ui.text_wrapped(description);
                }

                item.on_ui(world, ui);
                ui.separator();
            },
            None,
            None,
            None,
            show_all,
        )
    }

    /// Returns a table view with cols
    pub fn table(cols: &[&'static str]) -> Self {
        List::<Item>(
            |context, _, world, ui| {
                // context.as_mut().apply("previous");
                context.on_ui(world, ui);
            },
            None,
            None,
            Some(cols.to_vec()),
            true
        )
    }

    pub fn edit_attr_short_table() -> Self {
        List::<Item>(
            |context, item, world, ui| {
                context.as_mut().edit_attr_short_table(ui);
                item.on_ui(world, ui);
            },
            None,
            None,
            None,
            true
        )
    }

    pub fn edit_attr_table() -> Self {
        List::<Item>(
            |context, item, world, ui| {
                context.as_mut().edit_attr_table(ui);
                item.on_ui(world, ui);
            },
            None,
            None,
            None,
            true
        )
    }

    pub fn edit_block_view(sequence: Option<Sequence>) -> Self {
        List::<Item>(
            |context, item, world, ui| {
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

                let mut flags = TreeNodeFlags::empty();
                if item_index == 0 
                || context.as_ref().is_enabled("last_item").unwrap_or_default() 
                || context.as_ref().is_enabled("default_open").unwrap_or_default() {
                    flags |= TreeNodeFlags::DEFAULT_OPEN;
                }

                if ui.collapsing_header(
                    format!(
                        "{} {} - {}",
                        thunk_symbol,
                        context.block.block_name,
                        context.as_ref().hash_code()
                    ),
                    flags,
                ) {
                    ui.input_text(
                        format!("name {}", context.as_ref().entity()),
                        &mut context.block.block_name,
                    )
                    .build();
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
            },
            sequence,
            None,
            None,
            true,
        )
    }

    pub fn edit_attr_form() -> Self {
        List::<Item>(
            |context, item, world, ui| {
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
            },
            None,
            None,
            None,
            true,
        )
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
        world.register::<List<Item>>();

        Item::configure_app_world(world);
    }

    fn configure_app_systems(dispatcher: &mut DispatcherBuilder) {
        Item::configure_app_systems(dispatcher);
    }

    fn on_window_event(&'_ mut self, app_world: &World, event: &'_ WindowEvent<'_>) {
        let mut items = app_world.write_component::<Item>();

        for item in (&mut items).join() {
            item.on_window_event(app_world, event);
        }
    }

    fn on_ui(&'_ mut self, app_world: &specs::World, ui: &'_ imgui::Ui<'_>) {
        let mut contexts = app_world.write_component::<ThunkContext>();
        let connections = app_world.read_component::<Connection>();
        let mut items = app_world.write_component::<Item>();

        let title = self.title().clone();
        let mut item_index = 0;

        // Layout composition for the list view
        let mut layout = || {
            let List(_, sequence, ..) = self;
            if let Some(sequence) = sequence {
                let last = sequence.iter_entities().last();
                for entity in sequence.iter_entities() {
                    let row = (contexts.get_mut(entity), items.get_mut(entity));
                    if let (Some(context), Some(item)) = row {
                        let List(layout, ..) = self;
                        context
                            .as_mut()
                            .define("item", "index")
                            .edit_as(Value::Int(item_index));
                        context
                            .as_mut()
                            .add_bool_attr("last_item", last == Some(entity));

                        (layout)(context, item, app_world, ui);

                        item_index += 1;

                        ui.menu_bar(|| {
                            ui.menu("Menu", || {
                                if let Some(transpiled) = Project::from(context.as_ref().clone())
                                    .transpile_blocks()
                                    .ok()
                                {
                                    if !transpiled.trim().is_empty() {
                                        if MenuItem::new(format!(
                                            "Write {} output to console",
                                            context.block.block_name
                                        ))
                                        .build(ui)
                                        {
                                            println!("{}", transpiled);
                                        }
                                    }
                                }
                            });
                        });
                    }
                }
            } else {
                for (context, item, connection) in (&mut contexts, &mut items, connections.maybe()).join() {
                    if !self.4 && connection.is_none() && !context.as_ref().is_enabled("always_show").unwrap_or_default() {
                        continue
                    }

                    let List(layout, ..) = self;
                    context
                        .as_mut()
                        .define("item", "index")
                        .edit_as(Value::Int(item_index));

                    (layout)(context, item, app_world, ui);

                    item_index += 1;
                }
            }
        };

        if let Some(table_cols) = &self.3 {
            if let Some(token) = ui.begin_table_with_flags(
                format!("{:?}", title),
                table_cols.len(),
                TableFlags::RESIZABLE,
            ) {
                for col in table_cols {
                    ui.table_setup_column(col);
                }
                ui.table_headers_row();

                layout();
                token.end();
            }
        } else {
            layout();
        }
    }

    fn on_run(&'_ mut self, app_world: &World) {
        let entities = app_world.entities();
        let mut items = app_world.write_component::<Item>();

        for (_, item) in (&entities, &mut items).join() {
            item.on_run(app_world);
        }
    }
}
