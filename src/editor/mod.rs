mod attribute_editor;
mod node_editor;
mod node_editor_graph;
mod runtime_editor;

use atlier::system::start_editor;
use rand::Rng;
use specs::{prelude::*, Component};
pub use specs::prelude::WorldExt;
pub use specs::prelude::Builder;

pub use atlier::system::App;
pub use atlier::system::Attribute;
pub use atlier::system::Extension;
pub use atlier::system::Value;
pub use attribute_editor::AttributeEditor;
pub use node_editor::NodeEditor;
pub use node_editor_graph::NodeEditorGraph;
pub use runtime_editor::RuntimeEditor;
pub use runtime_editor::Loader;

use crate::AttributeGraph;
use crate::Runtime;

/// open a runtime editor for an attribute graph, and extension
pub fn open(
    title: &str, 
    extend: impl FnOnce(&mut RuntimeEditor<AttributeGraph>, &mut World, &mut DispatcherBuilder) + Clone  + 'static, 
    mut extension: impl for<'a, 'ui> Extension<'a, 'ui> + 'static
) 
{
    start_runtime_editor(
        title,
        Runtime::<AttributeGraph>::default(),
        move |app, world, dispatcher| { 
            extend(app, world, dispatcher);
        },
        move |app_world, ui| {
            let extension = &mut extension;
            extension.on_ui(app_world, ui);
        }
    )
}

/// Starts a runtime editor for some runtime state S
pub fn start_runtime_editor<S, F, Ext>(
    title: &str,
    initial_runtime: Runtime<S>,
    extension: F,
    on_ui: Ext,
) where
    S: crate::RuntimeState + Component + App,
    F: FnOnce(&mut RuntimeEditor<S>, &mut World, &mut DispatcherBuilder) + Clone  + 'static, 
    Ext: 'static + FnMut(&World, &imgui::Ui),
{
    let &[width, height] = S::window_size();

    start_editor(
        title,
        width.into(),
        height.into(),
        RuntimeEditor::<S>::new(initial_runtime),
        extension,
        on_ui,
    )
}

pub fn unique_title(title: impl AsRef<str>) -> String {
    let mut rng = rand::thread_rng();

    format!("{}_{:#04x}", title.as_ref(), rng.gen::<u16>())
}
