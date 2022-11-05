use std::{fmt::Display, ops::Deref, collections::{HashMap, BTreeMap}};

use atlier::system::Extension;
use copypasta::{ClipboardContext, ClipboardProvider};
use imgui::{TableColumnFlags, TableColumnSetup, TableFlags, Ui, Window};
use specs::{Join, RunNow, World, WorldExt};
pub use tokio::sync::broadcast::{channel, Receiver, Sender};
use tracing::{event, Level};

use crate::{
    engine::{Runner, NodeCommandHandler},
    prelude::{Runtime, Thunk},
};

use super::Appendix;

/// Extension
///
#[derive(Default)]
pub struct WorkspaceEditor {
    /// Enables the imgui demo window
    enable_demo: bool,
    /// Appendix
    appendix: Appendix,
    /// Clipboard context to enable copy/paste
    clipboard: Option<ClipboardContext>,
}

impl WorkspaceEditor {
    pub fn plugins(&mut self, world: &World, ui: &Ui) {
        let runtime = world.read_resource::<Runtime>();

        let table_flags =
            TableFlags::BORDERS_INNER_V | TableFlags::RESIZABLE | TableFlags::HIDEABLE;

        if let Some(token) = ui.begin_table_with_flags(format!("Plugins"), 2, table_flags) {
            fn name_column(ui: &Ui) {
                let mut table_column_setup = TableColumnSetup::new("Plugin");
                table_column_setup.flags =
                    TableColumnFlags::NO_HIDE | TableColumnFlags::WIDTH_FIXED;
                ui.table_setup_column_with(table_column_setup);
            }

            name_column(ui);
            ui.table_setup_column("Description");
            ui.table_headers_row();

            for thunk in runtime.deref().iter_thunks() {
                ui.table_next_row();
                ui.table_next_column();
                let name = thunk.0;
                ui.button_with_size(format!("{name}"), [140.0, 0.0]);
                if let Some(tooltip) = imgui::drag_drop::DragDropSource::new("ADD_PLUGIN")
                    .begin_payload(ui, WorkspaceCommand::AddPlugin(thunk))
                {
                    ui.text(format!("Thunk - {name}"));
                    tooltip.end();
                }

                ui.table_next_column();
                ui.text(thunk.1);
            }

            token.end();
        }
    }

    pub fn custom_node_handlers(&mut self, world: &World, ui: &Ui) {
        let handlers = world.fetch::<BTreeMap<String, NodeCommandHandler>>();

        let table_flags =
            TableFlags::BORDERS_INNER_V | TableFlags::RESIZABLE | TableFlags::HIDEABLE;

        if let Some(token) = ui.begin_table_with_flags(format!("Custom node handlers"), 2, table_flags) {
            fn name_column(ui: &Ui) {
                let mut table_column_setup = TableColumnSetup::new("Custom command");
                table_column_setup.flags =
                    TableColumnFlags::NO_HIDE | TableColumnFlags::WIDTH_FIXED;
                ui.table_setup_column_with(table_column_setup);
            }

            name_column(ui);
            ui.table_setup_column("Description");
            ui.table_headers_row();

            for (name, _) in handlers.iter() {
                ui.table_next_row();
                ui.table_next_column();
                ui.text(name);

                ui.table_next_column();
            }

            token.end();
        }
    }

    pub fn workspace_window(&mut self, world: &World, ui: &Ui) {
        Window::new("Workspace editor")
            .size([700.0, 600.0], imgui::Condition::Appearing)
            .build(ui, || {
                ui.checkbox("enable imgui demo window", &mut self.enable_demo);
                if self.enable_demo {
                    ui.show_demo_window(&mut self.enable_demo);
                }
                ui.spacing();
                ui.separator();

                if imgui::CollapsingHeader::new("Plugins").build(ui) {
                    self.plugins(world, ui);
                }

                if imgui::CollapsingHeader::new("Node command handlers").build(ui) {
                    self.custom_node_handlers(world, ui);
                }
            });
    }
}

/// Enumeration of workspace commands,
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceCommand {
    AddPlugin(Thunk),
}

impl Display for WorkspaceCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkspaceCommand::AddPlugin(Thunk(name, _, _)) => write!(f, "add plugin {name}"),
        }
    }
}

impl Extension for WorkspaceEditor {
    fn on_ui(&'_ mut self, world: &specs::World, ui: &'_ imgui::Ui<'_>) {
        if let Some(text) = ui.clipboard_text() {
            if let Some(clipboard_context) = self.clipboard.as_mut() {
                match clipboard_context.set_contents(text) {
                    Ok(_) => {
                        ui.set_clipboard_text(String::default());
                    }
                    Err(err) => {
                        event!(Level::ERROR, "Could not set clipboard contents, {err}");
                    }
                }
            } else {
                match ClipboardContext::new() {
                    Ok(ctx) => {
                        self.clipboard = Some(ctx);
                    }
                    Err(err) => {
                        event!(Level::ERROR, "Error creating clipboard context {err}");
                    }
                }
            }
        }

        self.workspace_window(world, ui);

        {
            let runner = world.system_data::<Runner>();
            for guest in runner.guests() {
                let mut guest_editor = guest.guest_editor();

                guest_editor.events_window(format!("Guest {}", guest.owner.id()), ui);

                guest_editor.run_now(guest.host().world());
            }
        }
    }

    fn on_run(&'_ mut self, world: &specs::World) {
        {
            let Runner {
                entities,
                mut guests,
                ..
            } = world.system_data::<Runner>();

            for (_, guest) in (&entities, &mut guests).join() {
                guest.run();
                guest.host_mut().world_mut().maintain();
            }
        }
    }
}

impl From<Appendix> for WorkspaceEditor {
    fn from(appendix: Appendix) -> Self {
        Self {
            enable_demo: false,
            appendix,
            clipboard: None,
        }
    }
}
