mod attribute_editor;
mod file_editor;
mod node_editor;
mod node_editor_graph;
mod event_editor;
mod runtime_editor;
mod section;

use atlier::system::start_editor;
use rand::Rng;
use specs::{prelude::*, Component};

pub use atlier::system::App;
pub use atlier::system::Attribute;
pub use atlier::system::Extension;
pub use atlier::system::Value;
pub use attribute_editor::AttributeEditor;
pub use file_editor::FileEditor;
pub use node_editor::NodeEditor;
pub use runtime_editor::RuntimeEditor;
pub use runtime_editor::SectionAttributes;
pub use event_editor::EventEditor;
pub use event_editor::EventComponent;
pub use section::Section;
pub use section::SectionExtension;

use crate::Runtime;
use crate::RuntimeState;

/// ShowEditor is a wrapper over a show function stored as a specs Component
#[derive(Clone, Component)]
#[storage(DenseVecStorage)]
pub struct ShowEditor<S>(pub fn(&mut S, &imgui::Ui))
where
    S: RuntimeState;

impl<S> Default for ShowEditor<S>
where
    S: RuntimeState,
{
    fn default() -> Self {
        Self(|_, _| {})
    }
}

/// Opens the runtime editor with a single section defined by S
pub fn open_simple_editor<S>()
where
    S: crate::RuntimeState + Component + App,
{
    start_runtime_editor::<S, _, _>(
        format!("Simple Runtime Editor for {}", <S as App>::name()).as_str(),
        Runtime::<S>::default(),
        |_, w, _| {
            w.register::<Section<S>>();

            w.create_entity()
                .maybe_with(Some(Section::<S>::from(S::default())))
                .build();
        },
        |_, _| {},
    );
}

pub fn open_editor<RtS, WorldInitF, SysInitF, Ext>(
    sections: Vec<Section<RtS>>,
    with_world: WorldInitF,
    with_systems: SysInitF,
    with_ext_app: Ext,
) where
    RtS: crate::RuntimeState + Component + App,
    WorldInitF: 'static + Fn(&mut World),
    SysInitF: 'static + Fn(&mut DispatcherBuilder),
    Ext: 'static + FnMut(&World, &imgui::Ui),
{
    open_editor_with(
        format!("Runtime Editor for {}", <RtS as App>::name()),
        Runtime::<RtS>::default(),
        sections,
        with_world,
        with_systems,
        with_ext_app,
    )
}

pub fn open_editor_with<RtS, WorldInitF, SysInitF, Ext>(
    title: impl AsRef<str>,
    initial_runtime: Runtime<RtS>,
    sections: Vec<Section<RtS>>,
    with_world: WorldInitF,
    with_systems: SysInitF,
    with_ext_app: Ext,
) where
    RtS: crate::RuntimeState + Component + App,
    WorldInitF: 'static + Fn(&mut World),
    SysInitF: 'static + Fn(&mut DispatcherBuilder),
    Ext: 'static + FnMut(&World, &imgui::Ui),
{
    start_runtime_editor::<RtS, _, _>(
        title.as_ref(),
        initial_runtime,
        move |_, w, d| {
            w.register::<Section<RtS>>();
            sections.iter().for_each(|s| {
                w.create_entity().maybe_with(Some(s.clone())).build();
            });
            with_world(w);
            with_systems(d);
        },
        with_ext_app,
    );
}

fn start_runtime_editor<S, F, Ext>(
    title: &str,
    initial_runtime: Runtime<S>,
    extension: F,
    ext_app: Ext,
) where
    S: crate::RuntimeState + Component + App,
    F: 'static + Fn(&mut RuntimeEditor<S>, &mut World, &mut DispatcherBuilder),
    Ext: 'static + FnMut(&World, &imgui::Ui),
{
    let &[width, height] = S::window_size();

    start_editor(
        title,
        width.into(),
        height.into(),
        RuntimeEditor::<S>::from(initial_runtime),
        extension,
        ext_app,
    )
}

pub fn unique_title(title: impl AsRef<str>) -> String {
    let mut rng = rand::thread_rng();

    format!("{}_{:#04x}", title.as_ref(), rng.gen::<u16>())
}
