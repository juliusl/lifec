use std::hash::Hash;
use std::sync::Arc;

use atlier::system::App;
use imgui::Ui;
use specs::Entity;

use crate::prelude::{Connection, Cursor, EventStatus, Transition};

use super::Appendix;

/// Type alias for an edit node ui function,
///
pub type EditNode = fn(&mut Node, &Ui);

/// Type alias for a display node ui function,
///
pub type DisplayNode = fn(&Node, &Ui);

/// Struct for visualizing entities w/ a cursor and/or connection component,
///
/// Can also signal actions to take,
///
#[derive(Eq)]
pub struct Node {
    /// Status of the current node,
    ///
    pub status: NodeStatus,
    /// Appendix to look up descriptions for related entities,
    /// 
    pub appendix: Arc<Appendix>,
    /// The Cursor component stores entities this node points to
    ///
    pub cursor: Option<Cursor>,
    /// The conenction component stores entities that point to this node,
    ///
    pub connection: Option<Connection>,
    /// The transition behavior for this node,
    ///
    pub transition: Option<Transition>,
    /// Command for this node,
    /// 
    pub command: Option<NodeCommand>,
    /// Edit node ui function,
    ///
    pub edit: Option<EditNode>,
    /// Display node ui function,
    ///
    pub display: Option<DisplayNode>,
}

/// Enumeration of node statuses,
///
#[derive(Hash, PartialEq, Eq)]
pub enum NodeStatus {
    Event(EventStatus),
}

/// Enumeration of node commands,
/// 
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NodeCommand {
    /// Command to activate this node,
    /// 
    Activate(Entity),
    /// Command to reset this node,
    /// 
    Reset(Entity),
}

impl App for Node {
    fn name() -> &'static str {
        "node"
    }

    fn edit_ui(&mut self, ui: &imgui::Ui) {
        if let Some(edit) = self.edit {
            edit(self, ui);
        } else {
            match &self.status {
                NodeStatus::Event(status) => {
                    if let Some(general) = self.appendix.general(&status.entity()) {
                        general.display_ui(ui);
                    }
                    ui.text(format!("id: {}", status.entity().id()));
                    ui.text(format!("status: {status}"));

                    if let Some(cursor) = self.cursor.as_ref() {
                        ui.text(format!("cursor - {}", cursor));
                    }
                    if let Some(transition) = self.transition.as_ref() {
                        ui.text(format!("transition: {:?}", transition));
                    }

                    match status {
                        EventStatus::Inactive(_) => {
                            if ui.button(format!("Start {}", status.entity().id())) {
                                self.command = Some(NodeCommand::Activate(status.entity()));
                            }
                        }
                        EventStatus::Cancelled(_) | EventStatus::Completed(_) => {
                            if ui.button(format!("Reset {}", status.entity().id())) {
                                self.command = Some(NodeCommand::Reset(status.entity()));
                            }
                        }
                        _ => {}
                    }
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
