
use crate::engine::Connection;
use crate::plugins::*;
use atlier::system::WindowEvent;
use imgui::TableFlags;
use imgui::Ui;
use crate::prelude::*;

/// List-layout widget for thunk_context's
#[derive(Component)]
#[storage(DenseVecStorage)]
pub struct List<Item>(
    /// Layout fn
    fn(&mut AttributeGraph, &mut Item, &World, &Ui),
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

    /// Returns a simple view for the thunk context, w/ editing
    /// off by default.
    ///
    /// If there is a "form to fill out" for a plugin, then by enabling
    /// `edit_form`, transient attributes w/ a symbol transient will add editing for the attribute named by that symbol
    ///
    /// For example,
    ///
    /// ```no_run
    /// ``` test example
    /// define edit_name example .symbol name
    ///
    /// add name .text Cool Name  
    /// ```
    /// ```
    ///
    /// Would display a single input for editing the `name` attribute. The underlying plugin does not need to add any special
    /// logic in order to use this feature.
    ///
    /// Note: If trying to edit a transient attribute instead of a stable one, then the format of the attribute name is
    /// {name}::{symbol}, so for example to edit edit_name defined above, the symbol value would need to be `edit_name::example`
    ///
    pub fn simple(show_all: bool) -> Self {
        List::<Item>(
            |graph, item, world, ui| {
                let plugin_symbol = graph
                    .find_symbol("plugin_symbol")
                    .unwrap_or("entity".to_string());

                ui.text(format!(
                    "{} - {}:",
                    plugin_symbol,
                    graph.hash_code()
                ));

                if let Some(description) = graph.find_symbol("description") {
                    ui.new_line();
                    ui.text_wrapped(description);
                    if let Some(caveats) = graph.find_symbol("caveats") {
                        if ui.is_item_hovered() {
                            ui.tooltip_text(caveats);
                        }
                    }
                }

                // let entity = .entity_id();
                // let clone = context.clone();
                // if clone.is_enabled("edit_form") {
                //     for (_, value) in context.state().() {
                //         if let Value::Symbol(symbol) = value {
                //             if let Some(attr) = context.as_mut().find_attr_mut(&symbol) {
                //                 attr.edit_value(format!("{symbol} {}", entity), ui);
                //             }
                //         }
                //     }
                // }

                item.on_ui(world, ui);
                ui.separator();
            },
            None,
            None,
            None,
            show_all,
        )
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
        let mut graphs = app_world.write_component::<AttributeGraph>();
        let connections = app_world.read_component::<Connection>();
        let mut items = app_world.write_component::<Item>();

        let title = self.title().clone();
        let mut item_index = 0;

        // Layout composition for the list view
        let mut layout = || {
            let List(_, sequence, ..) = self;
            if let Some(sequence) = sequence {
                for entity in sequence.iter_entities() {
                    let row = (graphs.get_mut(entity), items.get_mut(entity));
                    if let (Some(context), Some(item)) = row {
                        let List(layout, ..) = self;

                        (layout)(context, item, app_world, ui);

                        item_index += 1;
                    }
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
