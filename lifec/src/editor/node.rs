use std::sync::Arc;
use std::{collections::HashMap, hash::Hash};

use atlier::system::App;
use imgui::Ui;
use specs::Entity;

use crate::appendix::Appendix;
use crate::engine::{Adhoc, ConnectionState, NodeCommand};
use crate::guest::RemoteProtocol;
use crate::{
    prelude::{Connection, Cursor, Sequence, Transition},
    state::AttributeGraph,
};

mod event;
pub use event::EventNode;

mod engine;
pub use engine::EngineNode;

mod status;
pub use status::NodeStatus;

mod performance;
pub use performance::Profiler;

pub mod commands;
pub mod wire;

/// Type alias for an edit node ui function,
///
pub type EditNode = fn(&mut Node, &Ui) -> bool;

/// Type alias for a display node ui function,
///
pub type DisplayNode = fn(&Node, &Ui) -> bool;

/// Struct for visualizing and commanding node-like entities,
///
#[derive(Clone, Default)]
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
    /// Connection state for this node, which is also the key used to reference this within a connection,
    ///
    pub connection_state: Option<ConnectionState>,
    /// Adhoc config,
    ///
    pub adhoc: Option<Adhoc>,
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
    /// Suspended edit node ui,
    /// 
    pub suspended_edit: Option<EditNode>,
    /// Suspended display node ui,
    /// 
    pub suspended_display: Option<DisplayNode>,
    /// Remote protocol,
    /// 
    pub remote_protocol: Option<RemoteProtocol>,
}

impl Node {
    /// Returns true if this node was spawned,
    ///
    /// Spawned means that this node is ephemeral and can be deleted after it has transitioned to either Complete or Cancelled,
    ///
    pub fn is_spawned(&self) -> bool {
        if let Some(state) = self.connection_state {
            state.is_spawned()
        } else {
            false
        }
    }

    /// Returns true if this node has an adhoc config,
    ///
    pub fn is_adhoc(&self) -> bool {
        self.adhoc.is_some()
    }

    /// Returns the control block symbol, if empty the control is the root block,
    ///
    pub fn control_symbol(&self) -> String {
        if let Some(state) = self.appendix.state(&self.status.entity()) {
            state.control_symbol.to_string()
        } else {
            String::default()
        }
    }
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
                NodeStatus::Engine(_) | NodeStatus::Profiler(_) | NodeStatus::Custom(_) | NodeStatus::Empty => {}
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
        self.connection_state.hash(state);
        for (e, g) in self.mutations.iter() {
            e.hash(state);
            g.hash(state);
        }
        self.adhoc.hash(state);
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
            && self.adhoc == other.adhoc
            && self.connection_state == other.connection_state
            && self.mutations == other.mutations
    }
}
