use std::{
    collections::{BTreeMap, VecDeque},
    ops::Deref,
};

use atlier::system::App;
use imgui::{TreeNode, Ui};
use specs::Entity;
use tokio::sync::mpsc::Sender;
use tracing::{event, Level};

use crate::{
    engine::{Completion, Yielding},
    prelude::{Appendix, ErrorContext, Listener, NodeCommand, Plugins, StatusUpdate},
    state::AttributeGraph,
};

/// Struct for engine debugger,
///
#[derive(Default, Clone)]
pub struct Debugger {
    /// Appendix to look up metadata,
    ///
    appendix: Appendix,
    /// Map of completions,
    ///
    completions: VecDeque<((Entity, Entity), Completion)>,
    /// Status updates,
    ///
    status_updates: BTreeMap<Entity, VecDeque<StatusUpdate>>,
    /// Errors,
    ///
    errors: Vec<ErrorContext>,
    /// Number of status updates to keep,
    /// 
    status_update_limits: Option<usize>,
    /// Number of completions to keep,
    /// 
    completion_limits: Option<usize>,
    /// Command dispatcher,
    ///
    _command_dispatcher: Option<Sender<(NodeCommand, Option<Yielding>)>>,
    /// Update notification,
    /// 
    updated: Option<()>,
}

impl App for Debugger {
    fn name() -> &'static str {
        "lifec_debugger"
    }

    fn edit_ui(&mut self, ui: &imgui::Ui) {
        imgui::ChildWindow::new("Completion Tree")
            .size([600.0, 0.0])
            .build(ui, || {
                self.completion_tree(ui);
            });

        ui.same_line();
        imgui::ChildWindow::new("Status Updates")
            .border(true)
            .size([0.0, 0.0])
            .build(ui, || {
                self.updates_log(ui);
            });
    }

    fn display_ui(&self, _: &imgui::Ui) {}
}

impl Debugger {
    /// Propagates updated,
    ///
    pub fn propagate_update(&mut self) -> Option<()> {
        self.updated.take()
    }

    /// Set the update notification,
    /// 
    pub fn set_update(&mut self) {
        self.updated = Some(());
    }

    /// Returns an iterator over completions,
    ///
    pub fn completions(&self) -> impl Iterator<Item = &Completion> {
        self.completions.iter().map(|(_, c)| c)
    }

    /// Returns the status logs for an entity,
    /// 
    pub fn status_log(&self, entity: Entity) -> Option<VecDeque<StatusUpdate>> {
        self.status_updates.get(&entity).cloned()
    }

    /// Returns all status logs,
    /// 
    pub fn status_logs(&self) -> BTreeMap<Entity, VecDeque<StatusUpdate>> {
        self.status_updates.clone()
    }

    /// Returns the debugger's appendix,
    ///
    pub fn appendix(&self) -> &Appendix {
        &self.appendix
    }

    /// Sets the appendix,
    ///
    pub fn set_appendix(&mut self, appendix: Appendix) {
        self.appendix = appendix;
    }

    /// Displays a tree view of completion history,
    ///
    pub fn completion_tree(&self, ui: &Ui) {
        let mut groups = BTreeMap::<String, Vec<Completion>>::default();
        for (_, completion) in self.completions.iter() {
            let control_symbol = if let Some(name) = self.appendix.control_symbol(&completion.event)
            {
                name.to_string()
            } else if let Some(id) = completion.query.property("event_id").and_then(|p| p.int()) {
                self.appendix
                    .name_by_id(id as u32)
                    .and_then(|n| Some(n.to_string()))
                    .unwrap_or_default()
            } else {
                String::default()
            };

            if let Some(group) = groups.get_mut(&control_symbol) {
                group.push(completion.clone());
            } else {
                groups.insert(control_symbol, vec![completion.clone()]);
            }
        }

        for (group, completions) in groups {
            imgui::TreeNode::new(group).build(ui, || {
                for Completion {
                    timestamp,
                    event,
                    thunk,
                    control_values,
                    query,
                    returns,
                } in completions.iter()
                {
                    imgui::TreeNode::new(format!("{:?}{:?}", event, thunk))
                        .label::<String, _>(format!(
                            "{} Completion {} {}.{}",
                            timestamp,
                            self.appendix.control_symbol(&event).unwrap_or_default(),
                            self.appendix.name(&event).unwrap_or_default(),
                            self.appendix.name(thunk).unwrap_or_default()
                        ))
                        .build(ui, || {
                            ui.new_line();
                            if !control_values.is_empty() {
                                ui.text("Control Values");
                                ui.disabled(false, || {
                                    for (name, value) in control_values.iter() {
                                        AttributeGraph::edit_value(
                                            format!("{name}"),
                                            &mut value.clone(),
                                            None,
                                            ui,
                                        );
                                    }
                                });
                            }

                            ui.text(format!("Input - {}", query.name()));
                            ui.disabled(false, || {
                                for (i, (name, property)) in
                                    query.clone().iter_properties_mut().enumerate()
                                {
                                    property.edit(
                                        move |value| {
                                            AttributeGraph::edit_value(
                                                format!("{name} {i}.{}.{}", event.id(), thunk.id()),
                                                value,
                                                None,
                                                ui,
                                            )
                                        },
                                        move |values| {
                                            ui.indent();
                                            ui.group(|| {
                                                for (idx, value) in values.iter_mut().enumerate() {
                                                    AttributeGraph::edit_value(
                                                        format!(
                                                            "{name} {i}-{idx}.{}.{}",
                                                            event.id(),
                                                            thunk.id()
                                                        ),
                                                        value,
                                                        None,
                                                        ui,
                                                    );
                                                }
                                            });
                                            ui.unindent();
                                        },
                                        || None,
                                    );
                                }
                            });

                            ui.new_line();
                            if let Some(returns) = returns {
                                ui.text(format!("Output - {}", returns.name()));
                                ui.disabled(false, || {
                                    for (i, (name, property)) in
                                        returns.clone().iter_properties_mut().enumerate()
                                    {
                                        property.edit(
                                            move |value| {
                                                AttributeGraph::edit_value(
                                                    format!(
                                                        "{name} c_{i}.{}.{}",
                                                        event.id(),
                                                        thunk.id()
                                                    ),
                                                    value,
                                                    None,
                                                    ui,
                                                )
                                            },
                                            move |values| {
                                                imgui::ListBox::new(format!(
                                                    "{name} c_{i}.{}.{}",
                                                    event.id(),
                                                    thunk.id()
                                                ))
                                                .build(ui, || {
                                                    for (idx, value) in
                                                        values.iter_mut().enumerate()
                                                    {
                                                        AttributeGraph::edit_value(
                                                            format!(
                                                                "{name} c_{i}-{idx}.{}.{}",
                                                                event.id(),
                                                                thunk.id()
                                                            ),
                                                            value,
                                                            None,
                                                            ui,
                                                        );
                                                    }
                                                });
                                            },
                                            || None,
                                        );
                                    }
                                });
                            }
                            ui.new_line();
                            ui.separator();
                        });
                }
            });
        }
    }

    /// Dispalys logs in a tree format,
    ///
    pub fn updates_log(&mut self, ui: &Ui) {
        let mut logs = BTreeMap::<String, BTreeMap<Entity, &VecDeque<StatusUpdate>>>::default();

        for (e, status_updates) in self.status_updates.iter() {
            let control = self.appendix().control_symbol(e).unwrap_or_default();

            if !logs.contains_key(&control) {
                logs.insert(control.clone(), BTreeMap::default());
            }

            if let Some(updates) = logs.get_mut(&control) {
                updates.insert(*e, status_updates);
            }
        }

        for (log, status_updates) in logs {
            TreeNode::new(format!("Logs {}", log)).build(ui, || {
                for (idx, (entity, updates)) in status_updates.iter().enumerate() {
                    TreeNode::new(format!("{} {}", idx, entity.id()))
                        .label::<String, _>(format!(
                            "{}: {}",
                            entity.id(),
                            self.appendix().name(entity).unwrap_or_default()
                        ))
                        .build(ui, || {
                            let p = updates
                                .iter()
                                .map(|(_, p, _)| *p)
                                .last()
                                .unwrap_or_default();
                            if ui.small_button(format!("Copy to clipboard {}.{}", idx, entity.id()))
                            {
                                let message = updates
                                    .iter()
                                    .map(|(_, _, m)| m.to_string())
                                    .collect::<Vec<_>>()
                                    .join("\n");
                                ui.set_clipboard_text(message);
                            }
                            for (_, _, message) in updates.iter() {
                                if message.starts_with("1:") {
                                    ui.text_colored([0.0, 0.8, 0.8, 1.0], message.trim_start_matches("1:"));
                                } else {
                                    ui.text(message);
                                }
                            }

                            if p > 0.0 {
                                imgui::ProgressBar::new(p).build(ui);
                            }
                        });
                }
            });
        }
    }
}

impl PartialEq for Debugger {
    fn eq(&self, other: &Self) -> bool {
        self.appendix == other.appendix && self.completions == other.completions
    }
}

impl Listener for Debugger {
    fn create(world: &specs::World) -> Self {
        let command_dispatcher = world
            .system_data::<Plugins>()
            .features()
            .broker()
            .command_dispatcher();

        Self {
            appendix: world.fetch::<Appendix>().deref().clone(),
            _command_dispatcher: Some(command_dispatcher),
            ..Default::default()
        }
    }

    fn on_status_update(&mut self, status_update: &crate::prelude::StatusUpdate) {
        if !self.status_updates.contains_key(&status_update.0) {
            self.status_updates
                .insert(status_update.0, Default::default());
        }

        if let Some(status_updates) = self.status_updates.get_mut(&status_update.0) {
            if status_updates.len() > self.status_update_limits.unwrap_or(10) {
                status_updates.pop_front();
            }

            status_updates.push_back(status_update.clone());
        }
    }

    fn on_completion(&mut self, completion: crate::engine::Completion) {
        if self.completions.len() > self.completion_limits.unwrap_or(1000) {
            event!(Level::TRACE, "Discarding old results");
            self.completions.pop_front();
        }

        self.completions
            .push_back(((completion.event, completion.thunk), completion));
    }

    fn on_error_context(&mut self, error: &crate::prelude::ErrorContext) {
        self.errors.push(error.clone());
    }

    fn on_operation(&mut self, _: crate::prelude::Operation) {}

    fn on_completed_event(&mut self, _: &specs::Entity) {}
}
