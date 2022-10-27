use atlier::system::App;
use imgui::{Ui, TreeNode};

use crate::{prelude::EventStatus, state::AttributeGraph};

use super::{Node, NodeCommand, CommandDispatcher};

/// Extension for Node to edit event nodes,
/// 
pub trait EventNode {
    /// Edits an event node,
    /// 
    fn edit_event(&mut self, ui: &Ui, event: EventStatus);
}

impl EventNode for Node {
    fn edit_event(&mut self, ui: &Ui, event: EventStatus) {
        if let Some(general) = self.appendix.general(&event.entity()) {
            general.display_ui(ui);
        }

        ui.text(format!("id: {}", event.entity().id()));
        ui.text(format!("status: {event}"));

        if let Some(cursor) = self.cursor.as_ref() {
            ui.text(format!("cursor - {}", cursor));
        }
        if let Some(transition) = self.transition.as_ref() {
            ui.text(format!("transition: {:?}", transition));
        }

        match event {
            EventStatus::Inactive(_) => {
                if ui.button(format!("Start {}", event.entity().id())) {
                    self.activate(event.entity());
                }

                ui.same_line();
                if ui.button(format!("Pause {}", event.entity().id())) {
                    self.pause(event.entity());
                }
            }
            EventStatus::Paused(_) => {
                if ui.button(format!("Resume {}", event.entity().id())) {
                    self.resume(event.entity());
                }
                ui.same_line();
                if ui.button(format!("Cancel {}", event.entity().id())) {
                    self.pause(event.entity());
                }
            }
            EventStatus::InProgress(_) => {
                if ui.button(format!("Pause {}", event.entity().id())) {
                    self.pause(event.entity());
                }

                ui.same_line();
                if ui.button(format!("Cancel {}", event.entity().id())) {
                    self.cancel(event.entity());
                }
                // TODO: Can add a progress/status bar here
            }
            EventStatus::Cancelled(_) | EventStatus::Completed(_) => {
                if ui.button(format!("Reset {}", event.entity().id())) {
                    self.reset(event.entity());
                }
            }
            _ => {}
        }

        // Thunk state
        if let Some(sequence) = self.sequence.as_ref() {
            TreeNode::new(format!("Thunks {}", event.entity().id())).build(ui, || {
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
