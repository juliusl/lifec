mod workspace_editor;
pub use workspace_editor::WorkspaceEditor;

mod host_editor;
pub use host_editor::HostEditor;

mod node;
pub use node::Node;
pub use node::NodeStatus;
pub use node::NodeCommand;
pub use node::DisplayNode;
pub use node::EditNode;
pub use node::CommandDispatcher;
pub use node::EventNode;
pub use node::Profiler;

mod appendix;
pub use appendix::Appendix;
pub use appendix::General;
pub use appendix::State;

/// Generate a unique title
pub fn unique_title(title: impl AsRef<str>) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    format!("{}_{:#04x}", title.as_ref(), rng.gen::<u16>())
}
