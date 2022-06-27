mod runtime_editor;
pub use runtime_editor::RuntimeEditor;

mod progress;
pub use progress::ProgressStatusBar;

mod start;
pub use start::StartButton;
pub use start::Start;

use atlier::system::start_editor_from;
use rand::Rng;
use specs::prelude::*;
pub use specs::prelude::WorldExt;
pub use specs::prelude::Builder;
pub use atlier::system::App;
pub use atlier::system::Attribute;
pub use atlier::system::Extension;
pub use atlier::system::Value;
pub use atlier::system::WindowEvent;

/// open a runtime editor for an attribute graph, and extension
pub fn open<A, E>(
    title: &str,
    app: A,
    extension: E
) 
where
    A: App + Clone + for<'c> System<'c>,
    E: Extension + 'static
{
    let &[width, height] = A::window_size();

    start_editor_from(
        title,
        width.into(),
        height.into(),
        app,
        extension
    )
}

/// Generate a unique title
pub fn unique_title(title: impl AsRef<str>) -> String {
    let mut rng = rand::thread_rng();

    format!("{}_{:#04x}", title.as_ref(), rng.gen::<u16>())
}

/// encapsulates common widget pattern
#[derive(Clone)]
pub struct Widget<A>(Entity, Option<A>)
where
    A: App + Component;

impl<A> From<Entity> for Widget<A>
where
    A: App + Component
{
    fn from(entity: Entity) -> Self {
       Widget(entity, None)
    }
}

impl<A> Extension for Widget<A> 
where
    A: App + Component + Clone,
    <A as Component>::Storage: Default
{
    fn configure_app_world(world: &mut World) {
       world.register::<A>();
    }

    fn configure_app_systems(_: &mut DispatcherBuilder) {
    }

    fn on_ui(&'_ mut self, app_world: &World, ui: &'_ imgui::Ui<'_>) {
        let Self(entity, ..) = self;
        let mut components =  app_world.write_component::<A>();
        if let Some(app) = components.get_mut(*entity) {
            app.edit_ui(ui);
            app.display_ui(ui);
        }
    }

    fn on_window_event(&'_ mut self, _: &World, _: &'_ WindowEvent<'_>) {
    }

    fn on_run(&'_ mut self, _: &World) {
    }
}