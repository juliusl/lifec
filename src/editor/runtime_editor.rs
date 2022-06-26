use std::collections::HashMap;

use imgui::{ChildWindow, MenuItem, Window};
use specs::{Component, Entities, Entity, Join, ReadStorage, System, WriteStorage};

use super::App;
use crate::{
    plugins::{BlockContext, Event, Project, Thunk, ThunkContext},
    Runtime, RuntimeState,
};

#[derive(Clone)]
pub struct RuntimeEditor<S>
where
    S: RuntimeState,
{
    _runtime: Runtime<S>,
    project: Project,
    calls: HashMap<Entity, Thunk>,
    blocks: HashMap<Entity, BlockContext>,
    events: HashMap<String, Entity>,
}

impl<S> RuntimeEditor<S>
where
    S: RuntimeState,
{
    pub fn new(runtime: Runtime<S>) -> Self {
        let state = runtime
            .clone()
            .state
            .unwrap_or_default()
            .state()
            .as_ref()
            .clone();
        Self {
            _runtime: runtime,
            project: Project::from(state),
            calls: HashMap::default(),
            blocks: HashMap::default(),
            events: HashMap::default(),
        }
    }
}

impl<'a, S> System<'a> for RuntimeEditor<S>
where
    S: RuntimeState + Component,
{
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Thunk>,
        ReadStorage<'a, BlockContext>,
        WriteStorage<'a, Event>,
    );

    /// The runtime editor maintains a vector of sections that it displays
    /// This system coordinates updates to those sections, as well as initialization
    fn run(&mut self, (entities, calls, blocks, events): Self::SystemData) {
        for (entity, call, block, event) in
            (&entities, calls.maybe(), blocks.maybe(), events.maybe()).join()
        {
            if let Some(call) = call {
                self.calls.insert(entity, call.clone());
            }

            if let Some(block) = block {
                self.blocks.insert(entity, block.clone());
            }

            if let Some(event) = event {
                self.events.insert(event.to_string(), entity);
            }
        }
    }
}

impl<S> Default for RuntimeEditor<S>
where
    S: RuntimeState + Component,
{
    fn default() -> Self {
        Self {
            _runtime: Default::default(),
            project: Default::default(),
            calls: HashMap::default(),
            blocks: HashMap::default(),
            events: HashMap::default(),
        }
    }
}

impl<S> App for RuntimeEditor<S>
where
    S: RuntimeState + Component,
{
    fn name() -> &'static str {
        "Runtime Editor"
    }

    fn window_size() -> &'static [f32; 2] {
        &[1500.0, 720.0]
    }

    fn display_ui(&self, _: &imgui::Ui) {        
    }

    fn edit_ui(&mut self, ui: &imgui::Ui) {
        Window::new(Self::name())
            .size(*Self::window_size(), imgui::Condition::Appearing)
            .menu_bar(true)
            .build(ui, || {
                let project = &mut self.project;
                ui.menu_bar(|| {
                    project.edit_project_menu(ui);

                    ui.menu("Plugins", || {
                        for (entity, call) in self.calls.iter() {
                            if let Some(block) = self.blocks.get(entity) {
                                let Thunk(symbol, thunk) = call; 
                                let label = format!(
                                    "Call thunk {} {} - entity: {}",
                                    symbol,
                                    block.block_name,
                                    entity.id()
                                );
                                if MenuItem::new(label).build(ui) {
                                    let mut context = ThunkContext::from(block.as_ref().clone());
                                    thunk(&mut context);
                                }
                                if ui.is_item_hovered() {
                                    ui.tooltip(|| {
                                        block.clone().edit_block_tooltip_view(false, ui);
                                    });
                                }
                            }
                        }
                    });
                });

                if let Some(tabbar) = ui.tab_bar("runtime_tabs") {
                    for (_, block) in project.iter_block_mut().enumerate() {
                        let (block_name, block) = block;

                        let thunk_symbol = if let Some(thunk) = block.get_block("thunk") {
                            thunk.find_text("thunk_symbol")
                        } else {
                            None
                        };

                        let thunk_symbol = thunk_symbol.unwrap_or("entity".to_string());

                        if let Some(token) = ui.tab_item(format!("{} {}", thunk_symbol, block_name))
                        {
                            ui.group(|| {
                                block.edit_block_view(true, ui);
                                ChildWindow::new(&format!("table_view_{}", block_name))
                                    .size([0.0, 0.0])
                                    .build(ui, || {
                                        block.edit_block_table_view(ui);
                                    });
                            });

                            token.end();
                        }
                    }
                    tabbar.end();
                }
            });
    }
}
