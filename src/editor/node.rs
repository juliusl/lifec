use std::{hash::Hash, collections::HashMap};
use std::sync::Arc;

use atlier::system::App;
use imgui::{TreeNode, Ui};
use specs::Entity;

use crate::{
    prelude::{Connection, Cursor, EventStatus, Sequence, Transition},
    state::AttributeGraph,
};

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
                let buckets = histogram
                    .iter_linear(100)
                    .map(|h| h.percentile() as f32)
                    .collect::<Vec<_>>();
                imgui::PlotHistogram::new(
                    ui,
                    format!(
                        "Performance Buckets (ms) for {} -> {}",
                        incoming.id(),
                        connection.entity().id()
                    ),
                    buckets.as_slice(),
                )
                .overlay_text(format!("# of buckets {} (100ms)", buckets.len()))
                .graph_size([0.0, 75.0])
                .build();

                ui.spacing();
                let group = ui.begin_group();
                let percentile = histogram.value_at_percentile(50.0);
                ui.text(format!(
                    "~ {:3}% <= {:6} ms",
                    histogram.percentile_below(percentile) as u64,
                    percentile
                ));
                ui.spacing();
                let percentile = histogram.value_at_percentile(75.0);
                ui.text(format!(
                    "~ {:3}% <= {:6} ms",
                    histogram.percentile_below(percentile) as u64,
                    percentile
                ));
                let percentile = histogram.value_at_percentile(90.0);
                ui.text(format!(
                    "~ {:3}% <= {:6} ms",
                    histogram.percentile_below(percentile) as u64,
                    percentile
                ));
                let percentile = histogram.value_at_percentile(99.0);
                ui.text(format!(
                    "~ {:3}% <= {:6} ms",
                    histogram.percentile_below(percentile) as u64,
                    histogram.value_at_percentile(99.0)
                ));
                group.end();

                ui.same_line();
                ui.spacing();
                ui.same_line();
                let group = ui.begin_group();
                ui.text(format!("total samples: {:10}", histogram.len()));
                group.end();
                ui.new_line();
                drawn = true;
            }
        }
        drawn
    }

    /// Dispatch a command to activate entity,
    /// 
    pub fn activate(&mut self, entity: Entity) {
        self.command = Some(NodeCommand::Activate(entity));
    }

    /// Dispatch a command to pause entity,
    /// 
    pub fn pause(&mut self, entity: Entity) {
        self.command = Some(NodeCommand::Pause(entity));
    }

    /// Dispatch a command to reset entity,
    /// 
    pub fn reset(&mut self, entity: Entity) {
        self.command = Some(NodeCommand::Reset(entity));
    }

    /// Dispatch a command to resume entity,
    /// 
    pub fn resume(&mut self, entity: Entity) {
        self.command = Some(NodeCommand::Resume(entity));
    }

    /// Dispatch a command to cancel entity,
    /// 
    pub fn cancel(&mut self, entity: Entity) {
        self.command = Some(NodeCommand::Cancel(entity));
    }

    /// Dispatch a command to update a graph,
    /// 
    pub fn update(&mut self, graph: AttributeGraph) {
        self.command = Some(NodeCommand::Update(graph));
    }

    /// Dispatch a custom command,
    /// 
    pub fn custom(&mut self, name: &'static str, entity: Entity) {
        self.command = Some(NodeCommand::Custom(name, entity));
    }
}

/// Enumeration of node statuses,
///
#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub enum NodeStatus {
    /// These are event nodes
    Event(EventStatus),
    /// This is a termination point for event nodes that are adhoc operations
    Profiler,
}

impl NodeStatus {
    /// Returns the entity,
    /// 
    pub fn entity(&self) -> Entity {
        match self {
            NodeStatus::Event(status) => status.entity(),
            NodeStatus::Profiler => panic!("Not implemented"),
        }
    }
}

/// Enumeration of node commands,
///
#[derive(PartialEq, Eq, PartialOrd, Hash)]
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
    /// Command to update state,
    ///
    Update(AttributeGraph),
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
            match self.status {
                NodeStatus::Profiler => {}
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
                                self.activate(status.entity());
                            }

                            ui.same_line();
                            if ui.button(format!("Pause {}", status.entity().id())) {
                                self.pause(status.entity());
                            }
                        }
                        EventStatus::Paused(_) => {
                            if ui.button(format!("Resume {}", status.entity().id())) {
                                self.resume(status.entity());
                            }
                            ui.same_line();
                            if ui.button(format!("Cancel {}", status.entity().id())) {
                                self.pause(status.entity());
                            }
                        }
                        EventStatus::InProgress(_) => {
                            if ui.button(format!("Pause {}", status.entity().id())) {
                                self.pause(status.entity());
                            }

                            ui.same_line();
                            if ui.button(format!("Cancel {}", status.entity().id())) {
                                self.cancel(status.entity());
                            }
                            // TODO: Can add a progress/status bar here
                        }
                        EventStatus::Cancelled(_) | EventStatus::Completed(_) => {
                            if ui.button(format!("Reset {}", status.entity().id())) {
                                self.reset(status.entity());
                            }
                        }
                        _ => {}
                    }

                    // Thunk state
                    if let Some(sequence) = self.sequence.as_ref() {
                        TreeNode::new(format!("Thunks {}", status.entity().id())).build(ui, || {
                            for (i, s) in sequence.iter_entities().enumerate() {
                                if let Some(name) = self.appendix.name(&s) {
                                    TreeNode::new(format!("{i} - {name}")).build(ui, || {
                                        if let Some(state) = self.appendix.state(&s) {
                                            let mut graph =
                                                self.mutations.get(&s).cloned().unwrap_or(state.graph.clone());
                                            let previous = graph.clone();
                                            for (name, property) in
                                                graph.resolve_properties_mut().iter_properties_mut()
                                            {
                                                property.edit(
                                                    move |value| {
                                                        AttributeGraph::edit_value(
                                                            format!("{name} {i}"),
                                                            value,
                                                            ui,
                                                        )
                                                    },
                                                    move |values| {
                                                        imgui::ListBox::new(format!("{name} {i}"))
                                                            .build(ui, || {
                                                                for (idx, value) in
                                                                    values.iter_mut().enumerate()
                                                                {
                                                                    AttributeGraph::edit_value(
                                                                        format!("{name} {i}-{idx}"),
                                                                        value,
                                                                        ui,
                                                                    );
                                                                }
                                                            });
                                                    },
                                                    || None,
                                                );
                                            }

                                            if graph != previous {
                                                self.mutations.insert(s, graph.clone());
                                                self.command =
                                                    Some(NodeCommand::Update(graph.clone()));
                                            }
                                        }
                                    });
                                }
                            }
                        });
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
