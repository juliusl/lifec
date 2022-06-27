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
