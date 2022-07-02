pub mod runtime_editor;
pub use runtime_editor::RuntimeEditor;

mod shell;
pub use shell::Shell;

mod progress;
pub use progress::ProgressStatusBar;

mod call;
pub use call::Call;

mod start_button;
pub use start_button::StartButton;

mod task;
pub use task::Task;

mod list;
pub use list::List;

use specs::prelude::*;
pub use specs::prelude::WorldExt;
pub use specs::prelude::Builder;
pub use atlier::system::App;
pub use atlier::system::Attribute;
pub use atlier::system::Extension;
pub use atlier::system::Value;
pub use atlier::system::WindowEvent;

/// Opens a new window w/ the provided App and Extension
pub fn open<A, E>(
    title: &str,
    app: A,
    extension: E
) 
where
    A: App + for<'c> System<'c>,
    E: Extension + 'static
{
    use atlier::system::open_window;
    let &[width, height] = A::window_size();

    open_window(
        title,
        width.into(),
        height.into(),
        app,
        extension
    )
}

/// Generate a unique title
pub fn unique_title(title: impl AsRef<str>) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    format!("{}_{:#04x}", title.as_ref(), rng.gen::<u16>())
}
