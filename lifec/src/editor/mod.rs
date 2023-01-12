mod workspace_editor;
pub use workspace_editor::WorkspaceEditor;

mod host_editor;
pub use host_editor::HostEditor;

mod canvas;
pub use canvas::Canvas;

mod form;
pub use form::Form;

mod node;
pub use node::Node;
pub use node::NodeStatus;
pub use node::DisplayNode;
pub use node::EditNode;
pub use node::EventNode;
pub use node::Profiler;

/// Generate a unique title
pub fn unique_title(title: impl AsRef<str>) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    format!("{}_{:#04x}", title.as_ref(), rng.gen::<u16>())
}
