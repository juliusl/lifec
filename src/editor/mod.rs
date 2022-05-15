// pub use atlier::system::App;

// mod event_editor;
// mod event_graph_editor;
mod event_node_editor;
mod node_editor;
mod runtime_editor;
mod section;

use rand::Rng;
use specs::{prelude::*, Component};
use std::any::Any;
use atlier::system::start_editor;

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
pub fn open_simple_editor<S>()
where
    S: crate::RuntimeState + Component + App,
{
    start_runtime_editor::<S, _>(|_, w, _| {
        w.register::<Section<S>>();

        w
            .create_entity()
            .maybe_with(Some(Section::<S>::from(S::default())))
            .build();
    });
}

pub fn open_editor<RtS, SysInitF>(sections: Vec<Section::<RtS>>, with_systems: SysInitF)
where
    RtS: crate::RuntimeState + Component + App,
    SysInitF: 'static + Fn(&mut DispatcherBuilder)
{
    start_runtime_editor::<RtS, _>(move |_, w, d| {
        w.register::<Section<RtS>>();
        sections.iter().for_each(|s| {
            w.create_entity()
                .maybe_with(Some(s.clone()))
                .build();
        });

        with_systems(d);
    });
}

fn start_runtime_editor<S, F>(extension: F)
where
    S: crate::RuntimeState + Component + App,
    F: 'static + Fn(&mut RuntimeEditor<S>, &mut World, &mut DispatcherBuilder),
{
    start_editor(
        "Runtime Editor",
        1280.0,
        720.0,
        RuntimeEditor::<S>::default(),
        move |e, w, d| extension(e, w, d),
    )
}

pub fn unique_title(title: &str) -> String {
    let mut rng = rand::thread_rng();

    format!("{}_{:#04x}", title, rng.gen::<u16>())
}
