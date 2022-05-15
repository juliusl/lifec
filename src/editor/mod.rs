// pub use atlier::system::App;

// mod event_editor;
// mod event_graph_editor;
mod event_node_editor;
mod node_editor;
mod runtime_editor;
mod section;

use std::any::Any;

pub use atlier::system::{start_editor};
pub use atlier::system::App;
pub use runtime_editor::RuntimeEditor;
pub use section::Section;

use imgui::CollapsingHeader;
use specs::{Component, Builder, DenseVecStorage};

#[derive(Clone)]
pub struct Edit<S: Any + Send + Sync + Clone>(pub fn(&mut S, &imgui::Ui));

impl<S: Any + Send + Sync + Clone> Component for Edit<S> {
    type Storage = DenseVecStorage<Self>;
}

impl<S: Any + Send + Sync + Clone> App for Section<S> {
    fn title() -> &'static str {
        "Section"
    }

    fn show_editor(&mut self, ui: &imgui::Ui) {
        if CollapsingHeader::new(&self.title).build(ui) {
            let (Edit(editor), s) = (&mut self.editor, &mut self.state);
            editor(s, ui);
        }
    }
}


pub fn start_runtime_editor<S>()
where
    S: crate::RuntimeState + Component + App,
{
    use specs::WorldExt;

    start_editor(
        "Runtime Editor",
        1280.0,
        720.0,
        RuntimeEditor::<S>::default(),
        |_, world, _| {
            world.register::<Section<S>>();

            let section = Section::<S> { 
                title: "Test Section".to_string(), 
                editor: Edit(|s, ui|{
                    S::show_editor(s, ui);
                }), 
                state: S::default()
            };

            world
                .create_entity()
                .maybe_with(Some(section))
                .build();
        })
}
