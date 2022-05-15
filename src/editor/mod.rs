// pub use atlier::system::App;

// mod event_editor;
// mod event_graph_editor;
mod event_node_editor;
mod node_editor;
mod runtime_editor;

use atlier::system::{start_editor, Extension};
pub use runtime_editor::RuntimeEditor;
use specs::{Component, Builder};

use crate::{Runtime, editor::runtime_editor::Section};

// struct EditorProperties {
//     visible: bool
// }

// fn setup() -> impl App {
//     event_graph_editor::EventGraphEditor::default()
// }

pub fn start_runtime_editor<S>()
where
    S: crate::RuntimeState + Component,
{
    use specs::WorldExt;

    start_editor(
        "Runtime Editor",
        1920.0,
        1080.0,
        RuntimeEditor::<S>::default(),
        S::default(),
        |editor, world, dispatcher| {
            world.register::<Section>();

            let test_section = world
                .create_entity()
                .maybe_with(Some(Section { title: "Test Section".to_string() }))
                .build();

        },
    )
}
