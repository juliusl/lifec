use std::{collections::BTreeMap, ops::Deref};

use atlier::system::App;
use imgui::Ui;
use specs::Entity;
use tokio::sync::mpsc::Sender;

use crate::{
    engine::{Completion, Yielding},
    prelude::{Appendix, Listener, NodeCommand, Plugins},
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
    /// TODO: This could get massive, so probably need a way to clear it out
    ///
    completions: BTreeMap<(Entity, Entity), Completion>,
    /// Command dispatcher,
    ///
    command_dispatcher: Option<Sender<(NodeCommand, Option<Yielding>)>>,
}

impl App for Debugger {
    fn name() -> &'static str {
        "lifec_debugger"
    }

    fn edit_ui(&mut self, _: &imgui::Ui) {
        //
    }

    fn display_ui(&self, ui: &imgui::Ui) {
        self.completion_tree(ui);
    }
}

impl Debugger {
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
        for (
            _,
            Completion {
                event,
                thunk,
                query,
                returns,
                control_values,
            },
        ) in self.completions.iter()
        {
            imgui::TreeNode::new(format!("{:?}{:?}", event, thunk))
                .label::<String, _>(format!(
                    "Completion of {} {}.{}",
                    self.appendix.control_symbol(event).unwrap_or_default(),
                    self.appendix.name(event).unwrap_or_default(),
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

                    ui.text("Input");
                    ui.disabled(false, || {
                        for (i, (name, property)) in query.clone().iter_properties_mut().enumerate()
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
                    ui.text("Output");
                    if let Some(returns) = returns {
                        ui.disabled(false, || {
                            for (i, (name, property)) in
                                returns.clone().iter_properties_mut().enumerate()
                            {
                                property.edit(
                                    move |value| {
                                        AttributeGraph::edit_value(
                                            format!("{name} c_{i}.{}.{}", event.id(), thunk.id()),
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
                                            for (idx, value) in values.iter_mut().enumerate() {
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

    fn on_status_update(&mut self, _: &crate::prelude::StatusUpdate) {
        // TODO -- Display a "log" window here
    }

    fn on_operation(&mut self, _: crate::prelude::Operation) {
        // TODO -- Can implement "break points" here ?
    }

    fn on_completion(&mut self, completion: crate::engine::Completion) {
        // TODO -- If the appendix doesn't have the event entity, that means it is probably a spawned entity
        self.completions
            .insert((completion.event, completion.thunk), completion);
    }

    fn on_error_context(&mut self, _: &crate::prelude::ErrorContext) {
        // TODO -- Display errors and fixes
    }

    fn on_completed_event(&mut self, _: &specs::Entity) {
        // TODO -- Could use this to bring completions to the top?
    }
}
