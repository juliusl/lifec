use std::{hash::Hash, collections::HashMap};
use std::sync::Arc;

use atlier::system::App;
use imgui::Ui;
use specs::Entity;

use crate::{prelude::{Connection, Cursor, Sequence, Transition}, state::AttributeGraph};
use super::Appendix;

mod event;
pub use event::EventNode;

mod commands;
pub use commands::NodeCommand;
pub use commands::CommandDispatcher;

mod status;
pub use status::NodeStatus;

mod performance;
pub use performance::Profiler;

/// Type alias for an edit node ui function,
///
pub type EditNode = fn(&mut Node, &Ui);

/// Type alias for a display node ui function,
///
pub type DisplayNode = fn(&Node, &Ui);

/// Struct for visualizing and commanding node-like entities,
///
pub struct Node {
    /// Status of the current node,
    ///
    pub status: NodeStatus,
    /// Appendix to look up descriptions for related entities,
    ///
    pub appendix: Arc<Appendix>,
    /// The cursor component stores entities this node points to
    ///
    pub cursor: Option<Cursor>,
    /// The conenction component stores entities that point to this node,
    ///
    pub connection: Option<Connection>,
    /// The transition behavior for this node,
    ///
    pub transition: Option<Transition>,
    /// The internal sequence this node represents,
    ///
    pub sequence: Option<Sequence>,
    /// Command for this node,
    ///
    pub command: Option<NodeCommand>,
    /// If this node has been edited, then this will be set.
    ///
    pub mutations: HashMap<Entity, AttributeGraph>,
    /// Edit node ui function,
    ///
    pub edit: Option<EditNode>,
    /// Display node ui function,
    ///
    pub display: Option<DisplayNode>,
}

impl App for Node {
    fn name() -> &'static str {
        "node"
    }

    fn edit_ui(&mut self, ui: &imgui::Ui) {
        if let Some(edit) = self.edit {
            edit(self, ui);
        } else {
            match self.status {
                // TODO:
                NodeStatus::Engine(_) | NodeStatus::Profiler => {}
                NodeStatus::Event(status) => {
                    self.edit_event(ui, status);
                }
            }
        }
    }

    fn display_ui(&self, ui: &imgui::Ui) {
        if let Some(display) = self.display {
            display(self, ui);
        }
    }
}

impl Hash for Node {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.status.hash(state);
        self.cursor.hash(state);
        self.connection.hash(state);
        self.transition.hash(state);
        self.appendix.hash(state);
        self.command.hash(state);
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.status == other.status
            && self.cursor == other.cursor
            && self.connection == other.connection
            && self.transition == other.transition
            && self.appendix == other.appendix
            && self.command == other.command
    }
}
