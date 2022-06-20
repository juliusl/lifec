use std::collections::HashMap;

use imgui::{Window, MenuItem, ChildWindow};
use specs::{Component, Entities, System, ReadStorage, Join, Entity};

use super::App;
use crate::{Runtime, RuntimeState, plugins::{Project, Call, ThunkContext, BlockContext}};

pub struct RuntimeEditor<S>
where
    S: RuntimeState,
{
    _runtime: Runtime<S>,
    project: Project,
    calls: HashMap<Entity, Call>,
    blocks: HashMap<Entity, BlockContext>,
}

impl<S> RuntimeEditor<S>
where
    S: RuntimeState,
{
    pub fn new(runtime: Runtime<S>) -> Self {
        let state = runtime.clone().state.unwrap_or_default().state().as_ref().clone();
        Self {
            _runtime: runtime,
            project: Project::from(state),
            calls: HashMap::default(),
            blocks: HashMap::default(),
        }
    }
}

impl<'a, S> System<'a> for RuntimeEditor<S>
where
    S: RuntimeState + Component,
{
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Call>,
        ReadStorage<'a, BlockContext>,
    );

    /// The runtime editor maintains a vector of sections that it displays
    /// This system coordinates updates to those sections, as well as initialization
    fn run(&mut self, (entities, calls, blocks): Self::SystemData) {
        for (entity, call, block) in (&entities, calls.maybe(), blocks.maybe()).join() {
            if let Some(call) = call {
                self.calls.insert(entity, call.clone());
            }

            if let Some(block) = block {
                self.blocks.insert(entity, block.clone());
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

    fn show_editor(&mut self, ui: &imgui::Ui) {
        Window::new(Self::name())
            .size(*Self::window_size(), imgui::Condition::Appearing)
            .menu_bar(true)
            .build(ui, || {
                let project = &mut self.project;
                ui.menu_bar(|| {
                    project.edit_project_menu(ui);

                    ui.menu("Thunks", ||{
                        for (entity, call) in self.calls.iter() {
                            if let Some(block) = self.blocks.get(entity) {
                                let label = format!("Call thunk {} - {:?}", call.symbol().as_ref(), entity);
                                if MenuItem::new(label).build(ui) {
                                    let mut context = ThunkContext(block.clone());
                                    call.call(&mut context);
                                }
                                if ui.is_item_hovered() {
                                    ui.tooltip(||{
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
                        if let Some(token) = ui.tab_item(format!("Block entity: {}", block_name)) {
                            ui.group(|| {
                                block.edit_block_view(true, ui);
                                ChildWindow::new(&format!("table_view_{}", block_name)).size([0.0, 0.0]).build(ui, ||{
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

