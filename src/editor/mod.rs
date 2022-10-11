 mod runtime_editor;
pub use runtime_editor::RuntimeEditor;

mod progress;
pub use progress::ProgressStatusBar;

mod start_button;
pub use start_button::StartButton;

mod task;
pub use task::Task;

mod list;
pub use list::List;


/// Generate a unique title
pub fn unique_title(title: impl AsRef<str>) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    format!("{}_{:#04x}", title.as_ref(), rng.gen::<u16>())
}
