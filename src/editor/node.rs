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

/// Struct for visualizing and commanding node-like entities,
///
#[derive(Eq)]
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

impl Node {
    /// Displays performance of nodes connected to this node,
    ///
    /// Returns true if something was drawn
    ///
    pub fn histograms(&self, ui: &Ui) -> bool {
        let mut drawn = false;
        if let Some(connection) = self.connection.as_ref() {
            for (incoming, histogram) in connection
                .performance()
                .filter(|(_, h)| !h.is_empty() && h.len() > 1)
            {
                // TODO: Can use appendix to look up stuff
                // TODO: Add view-options
                imgui::PlotLines::new(
                    ui,
                    format!(
                        "Performance (ms) for {} -> {}",
                        incoming.id(),
                        connection.entity().id()
                    ),
                    histogram
                        .iter_all()
                        .map(|h| h.percentile() as f32)
                        .collect::<Vec<_>>()
                        .as_slice(),
                )
                .graph_size([0.0, 75.0])
                .build();

                ui.spacing();
                let group = ui.begin_group();
                let percentile = histogram.value_at_percentile(50.0);
                
                ui.text(format!(
                    "50th ({:5}): {:5} ms",
                    histogram.percentile_below(percentile) as u64,
                    percentile
                ));

                ui.spacing();
                let percentile = histogram.value_at_percentile(75.0);
                ui.text(format!(
                    "75th ({:5}): {:5} ms",
                    histogram.percentile_below(percentile) as u64,
                    percentile
                ));
                let percentile = histogram.value_at_percentile(90.0);
                ui.text(format!(
                    "90th ({:5}): {:5} ms",
                    histogram.percentile_below(percentile) as u64,
                    percentile
                ));
                let percentile = histogram.value_at_percentile(99.0);
                ui.text(format!(
                    "99th ({:5}): {:5} ms",
                    histogram.percentile_below(percentile) as u64,
                    histogram.value_at_percentile(99.0)
                ));
                group.end();
                
                ui.new_line();
                drawn = true;
            }
        }
        drawn
    }
}

/// Enumeration of node statuses,
///
#[derive(Hash, PartialEq, Eq)]
pub enum NodeStatus {
    /// These are event nodes
    Event(EventStatus),
    /// This is a termination point for event nodes that are adhoc operations
    Profiler,
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
    /// Command to pause this node,
    ///
    Pause(Entity),
    /// Command to resume a paused node,
    ///
    Resume(Entity),
    /// Command to cancel this node,
    ///
    Cancel(Entity),
    /// Custom command for this node,
    ///
    /// This allows for extending capabilities of the node,
    ///
    Custom(&'static str, Entity),
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
                NodeStatus::Profiler => {},
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

                            ui.same_line();
                            if ui.button(format!("Set breakpoint {}", status.entity().id())) {
                                self.command = Some(NodeCommand::Pause(status.entity()));
                            }
                        }
                        EventStatus::Paused(_) => {
                            if ui.button(format!("Resume {}", status.entity().id())) {
                                self.command = Some(NodeCommand::Resume(status.entity()));
                            }
                            ui.same_line();
                            if ui.button(format!("Cancel {}", status.entity().id())) {
                                self.command = Some(NodeCommand::Cancel(status.entity()));
                            }
                        }
                        EventStatus::InProgress(_) => {
                            if ui.button(format!("Pause {}", status.entity().id())) {
                                self.command = Some(NodeCommand::Pause(status.entity()));
                            }

                            ui.same_line();
                            if ui.button(format!("Cancel {}", status.entity().id())) {
                                self.command = Some(NodeCommand::Cancel(status.entity()));
                            }
                            // TODO: Can add a progress/status bar here
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
