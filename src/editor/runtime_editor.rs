use imgui::{Window, TreeNodeFlags};
use specs::{Component, Entities, System};

use super::App;
use crate::{Runtime, RuntimeState, plugins::Project};

#[derive(Clone)]
pub struct RuntimeEditor<S>
where
    S: RuntimeState,
{
    runtime: Runtime<S>,
}

impl<S> RuntimeEditor<S>
where
    S: RuntimeState,
{
    pub fn new(runtime: Runtime<S>) -> Self {
        Self {
            runtime,
        }
    }
}

impl<'a, S> System<'a> for RuntimeEditor<S>
where
    S: RuntimeState + Component,
{
    type SystemData = (Entities<'a>,);

    /// The runtime editor maintains a vector of sections that it displays
    /// This system coordinates updates to those sections, as well as initialization
    fn run(&mut self, _: Self::SystemData) {}
}

impl<S> Default for RuntimeEditor<S>
where
    S: RuntimeState + Component,
{
    fn default() -> Self {
        Self {
            runtime: Default::default(),
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
                if let Some(state) = &mut self.runtime.state {
                    let graph = state.dispatcher().as_ref();
                    let mut project = Project::from(graph.clone());
                    ui.menu_bar(|| {
                        project.edit_project_menu(ui);
                    });
                    
                   for (entity, block) in project.iter_block_mut().enumerate() {
                        let (block_name, block) = block;
                        let mut flags = TreeNodeFlags::empty();
                        if entity == 0 {
                            flags |= TreeNodeFlags::DEFAULT_OPEN;
                        }
                        if ui.collapsing_header(format!("Block entity: {}", block_name), flags) {
                            ui.indent();
                            block.edit_block_view(ui);
                            ui.new_line();
                            block.edit_block_table_view(ui);
                            ui.unindent();
                        }
                   }
                }
            });
    }
}

