use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    ops::Deref,
};

use atlier::system::App;
use imgui::Ui;
use specs::Entity;
use tokio::sync::mpsc::Sender;

use crate::{
    engine::{Completion, Yielding},
    prelude::{Appendix, ErrorContext, Listener, NodeCommand, Plugins, StatusUpdate},
    state::AttributeGraph,
};

pub mod wire;

/// Struct for engine debugger,
///
#[derive(Default, Clone)]
pub struct Debugger {
    /// Appendix to look up metadata,
    ///
    appendix: Appendix,
    /// Map of completions,
    ///
    completions: BTreeMap<(Entity, Entity), Completion>,
    /// Status updates,
    ///
    status_updates: VecDeque<StatusUpdate>,
    /// Errors,
    ///
    errors: Vec<ErrorContext>,
    /// Command dispatcher,
    ///
    command_dispatcher: Option<Sender<(NodeCommand, Option<Yielding>)>>,
    /// Updated
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

    pub fn set_update(&mut self) {
        self.updated = Some(());
    }

    /// Returns an iterator over completions,
    ///
    pub fn completions(&self) -> impl Iterator<Item = &Completion> {
        self.completions.iter().map(|(_, c)| c)
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
        let mut groups = BTreeMap::<String, BTreeSet<Completion>>::default();
        for (_, completion) in self.completions.iter() {
            let event_name = if let Some(name) = self.appendix.name_by_id(completion.event.id()) {
                name.to_string()
            } else if let Some(id) = completion.query.property("event_id").and_then(|p| p.int()) {
                self.appendix
                    .name_by_id(id as u32)
                    .and_then(|n| Some(n.to_string()))
                    .unwrap_or_default()
            } else {
                String::default()
            };

            if let Some(group) = groups.get_mut(&event_name) {
                group.insert(completion.clone());
            } else {
                let mut set = BTreeSet::<Completion>::default();
                set.insert(completion.clone());
                groups.insert(event_name, set);
            }
        }

        for (group, completions) in groups {
            imgui::TreeNode::new(group).build(ui, || {
                for Completion {
                    event,
                    thunk,
                    control_values,
                    query,
                    returns,
                } in completions.iter()
                {
                    imgui::TreeNode::new(format!("{:?}{:?}", event, thunk))
                        .label::<String, _>(format!(
                            "Completion of {} {}.{}",
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

    pub fn updates_log(&mut self, ui: &Ui) {
        for (e, p, message) in self.status_updates.iter() {
            ui.text(format!("{:?} ", e));
            ui.same_line();
            ui.text(message);

            if *p > 0.0 {
                ui.same_line();
                imgui::ProgressBar::new(*p).build(ui);
            }
        }

        if let Some(_) = self.updated.take() {
            ui.set_scroll_here_y();
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
            command_dispatcher: Some(command_dispatcher),
            ..Default::default()
        }
    }

    fn on_status_update(&mut self, status_update: &crate::prelude::StatusUpdate) {
        if self.status_updates.len() > 1000 {
            self.status_updates.pop_front();
        }

        if let Some((e, p, _)) = self.status_updates.back() {
            if status_update.0 == *e  && *p > 0.0 {
                self.status_updates.pop_back();
            }
        }

        self.status_updates.push_back(status_update.clone());
        self.updated = Some(());
    }

    fn on_operation(&mut self, _: crate::prelude::Operation) {
        // TODO -- Can implement "break points" here ?
    }

    fn on_completion(&mut self, completion: crate::engine::Completion) {
        self.completions
            .insert((completion.event, completion.thunk), completion);
    }

    fn on_error_context(&mut self, error: &crate::prelude::ErrorContext) {
        self.errors.push(error.clone());
    }

    fn on_completed_event(&mut self, _: &specs::Entity) {}
}
