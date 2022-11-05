use imgui::{TreeNode, TreeNodeFlags, Ui, DragDropFlags};
use tracing::Level;

use crate::{prelude::{EventStatus, Thunk}, state::AttributeGraph, editor::workspace_editor::WorkspaceCommand};

use super::{CommandDispatcher, Node, NodeCommand};

/// Extension for Node to edit event nodes,
///
pub trait EventNode {
    /// Edits an event node,
    ///
    fn edit_event(&mut self, ui: &Ui, event: EventStatus);

    /// Shows event buttons,
    ///
    fn event_buttons(&mut self, ui: &Ui, event: EventStatus);
}

impl EventNode for Node {
    fn edit_event(&mut self, ui: &Ui, event: EventStatus) {
        let tree_node = match self.connection_state {
            Some(connection_state) if connection_state.is_spawned() => {
                let source = connection_state.source();
                let tree_node = TreeNode::new(format!("{:?}", event.entity()))
                    .label::<String, _>(format!("{}", self.appendix.name(&source).unwrap_or("--")))
                    .flags(TreeNodeFlags::SPAN_FULL_WIDTH | TreeNodeFlags::FRAME_PADDING)
                    .push(ui);
                // ui.table_next_column();
                // if let Some(general) = self.appendix.general(&source) {
                //     general.display_ui(ui);
                // }
                tree_node
            }
            _ => {
                let tree_node = TreeNode::new(format!("{:?}", event.entity()))
                    .label::<String, _>(format!(
                        "{}",
                        self.appendix.name(&event.entity()).unwrap_or("--")
                    ))
                    .flags(TreeNodeFlags::SPAN_FULL_WIDTH | TreeNodeFlags::FRAME_PADDING)
                    .push(ui);
                // ui.table_next_column();
                // if let Some(general) = self.appendix.general(&event.entity()) {
                //     general.display_ui(ui);
                // }
                tree_node
            }
        };

        if let Some(target) = imgui::drag_drop::DragDropTarget::new(ui) {
            match target.accept_payload::<WorkspaceCommand, _>(
                "ADD_PLUGIN",
                DragDropFlags::empty(),
            ) {
                Some(result) => match result {
                    Ok(command) => {
                        match command.data {
                            WorkspaceCommand::AddPlugin(Thunk(name, ..)) => {
                                self.custom(format!("add_plugin::{name}"), self.status.entity());
                            },
                        }
                    }
                    Err(err) => {
                        tracing::event!(
                            Level::ERROR,
                            "Error accepting workspace command, {err}"
                        );
                    }
                },
                None => {}
            }
        }

        ui.table_next_column();
        ui.text(format!("{event}"));

        ui.table_next_column();
        if let Some(transition) = self.transition.as_ref() {
            ui.text(format!("{:?}", transition));
        } else {
            ui.text_disabled("--");
        }

        ui.table_next_column();
        if let Some(cursor) = self.cursor.as_ref() {
            ui.text(format!("{}", cursor));
        }

        ui.table_next_column();
        self.event_buttons(ui, event);

        if let Some(tree_node) = tree_node {
            // Thunk state
            if let Some(sequence) = self.sequence.as_ref() {
                for (i, s) in sequence.iter_entities().enumerate() {
                    ui.table_next_row();
                    ui.table_next_column();
                    if let Some(name) = self.appendix.name(&s) {
                        let tree_node = TreeNode::new(format!("{:?}", s))
                            .label::<String, _>(format!("{name}"))
                            .flags(TreeNodeFlags::SPAN_FULL_WIDTH)
                            .push(ui);

                        if let Some(tree_node) = tree_node {
                            if let Some(state) = self.appendix.state(&s) {
                                let mut graph = self
                                    .mutations
                                    .get(&s)
                                    .cloned()
                                    .unwrap_or(state.graph.clone().unwrap());
                                let previous = graph.clone();

                                ui.table_set_column_index(4);
                                for (name, property) in
                                    graph.resolve_properties_mut().iter_properties_mut()
                                {
                                    property.edit(
                                        move |value| {
                                            AttributeGraph::edit_value(
                                                format!("{name} {i}"),
                                                value,
                                                None,
                                                ui,
                                            )
                                        },
                                        move |values| {
                                            imgui::ListBox::new(format!("{name} {i}")).build(
                                                ui,
                                                || {
                                                    for (idx, value) in
                                                        values.iter_mut().enumerate()
                                                    {
                                                        AttributeGraph::edit_value(
                                                            format!("{name} {i}-{idx}"),
                                                            value,
                                                            None,
                                                            ui,
                                                        );
                                                    }
                                                },
                                            );
                                        },
                                        || None,
                                    );
                                }

                                if graph != previous {
                                    self.mutations.insert(s, graph.clone());
                                    self.command = Some(NodeCommand::Update(graph.clone()));
                                }
                            }

                            tree_node.pop();
                        }
                    }
                }
            }

            tree_node.pop();
        }
    }

    fn event_buttons(&mut self, ui: &Ui, event: EventStatus) {
        match event {
            EventStatus::Inactive(_) => {
                if ui.small_button(format!("Start {:2}", event.entity().id())) {
                    self.activate(event.entity());
                }

                if self.is_adhoc() {
                    ui.same_line();
                    if ui.small_button(format!("Spawn {:2}", event.entity().id())) {
                        self.spawn(event.entity());
                    }
                }

                ui.same_line();
                if ui.small_button(format!("Pause {:2}", event.entity().id())) {
                    self.pause(event.entity());
                }
            }
            EventStatus::Paused(_) => {
                if ui.small_button(format!("Resume {:2}", event.entity().id())) {
                    self.resume(event.entity());
                }
                ui.same_line();
                if ui.small_button(format!("Cancel {:2}", event.entity().id())) {
                    self.pause(event.entity());
                }
            }
            EventStatus::InProgress(_) => {
                if ui.small_button(format!("Pause {:2}", event.entity().id())) {
                    self.pause(event.entity());
                }

                ui.same_line();
                if ui.small_button(format!("Cancel {:2}", event.entity().id())) {
                    self.cancel(event.entity());
                }
            }
            EventStatus::Cancelled(_) | EventStatus::Completed(_) => {
                if ui.small_button(format!("Reset {:2}", event.entity().id())) {
                    self.reset(event.entity());
                }

                if self.is_spawned() && {
                    ui.same_line();
                    ui.small_button(format!("Delete {:2}", event.entity().id()))
                } {
                    self.custom("delete_spawned", event.entity());
                }
            }
            _ => {}
        }
    }
}
