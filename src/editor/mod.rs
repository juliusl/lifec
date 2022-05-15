// pub use atlier::system::App;

// mod event_editor;
// mod event_graph_editor;
mod event_node_editor;
mod node_editor;
mod runtime_editor;
mod section;

use std::any::Any;
use specs::{prelude::*, Component};

pub use atlier::system::{start_editor};
pub use atlier::system::App;
pub use runtime_editor::RuntimeEditor;
pub use section::Section;

/// Edit is a function wrapper over a display function that is stored as a specs Component
#[derive(Clone, Component)]
#[storage(DenseVecStorage)]
pub struct Edit<S: Any + Send + Sync + Clone>(pub fn(&mut S, &imgui::Ui));

// #[derive(Clone, Component)]
// #[storage(DenseVecStorage)]
// pub struct Show<S: Any + Send + Sync + Clone>(pub fn(&S, &imgui::Ui));

/// Event component is the the most basic data unit of the runtime
#[derive(Clone, Component)]
#[storage(DenseVecStorage)]
pub struct EventComponent {
    pub on: String,
    pub dispatch: String,
    pub call: String,
    pub transitions: Vec<String>,
}

/// Opens the runtime editor with a single section defined by S
pub fn start_simple_runtime_editor<S>()
where
    S: crate::RuntimeState + Component + App,
{
    start_editor(
        "Runtime Editor",
        1280.0,
        720.0,
        RuntimeEditor::<S>::default(),
        |_, world, _| {
            world.register::<Section<S>>();

            world
                .create_entity()
                .maybe_with(Some(Section::<S>::from(S::default())))
                .build();
        })
}
