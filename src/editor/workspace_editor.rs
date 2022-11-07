use std::{collections::BTreeMap, ops::Deref};

use atlier::system::Extension;
use copypasta::{ClipboardContext, ClipboardProvider};
use imgui::{TableColumnFlags, TableColumnSetup, TableFlags, TreeNode, TreeNodeFlags, Ui, Window};
use specs::{Join, RunNow, World, WorldExt};
pub use tokio::sync::broadcast::{channel, Receiver, Sender};
use tracing::{event, Level};

use crate::{
    editor::node::WorkspaceCommand,
    engine::{NodeCommandHandler, Runner, Yielding},
    prelude::{Runtime, State}, guest::Guest,
};

use super::{Appendix, NodeCommand};

/// Extension to display workspace editing tools,
///
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
    /// Handle the clipboard for tools,
    ///
    pub fn handle_clipboard(&mut self, ui: &Ui) {
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
    }

    /// Plugins table,
    ///
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

    /// Custom node handlers,
    ///
    pub fn custom_node_handlers(&mut self, world: &World, ui: &Ui) {
        let handlers = world.fetch::<BTreeMap<String, NodeCommandHandler>>();

        let table_flags =
            TableFlags::BORDERS_INNER_V | TableFlags::RESIZABLE | TableFlags::HIDEABLE;

        if let Some(token) =
            ui.begin_table_with_flags(format!("Custom node command handlers"), 1, table_flags)
        {
            fn name_column(ui: &Ui) {
                let mut table_column_setup = TableColumnSetup::new("Custom command");
                table_column_setup.flags =
                    TableColumnFlags::NO_HIDE | TableColumnFlags::WIDTH_FIXED;
                ui.table_setup_column_with(table_column_setup);
            }

            name_column(ui);
            // ui.table_setup_column("Description");
            ui.table_headers_row();

            for (name, _) in handlers.iter() {
                ui.table_next_row();
                ui.table_next_column();
                ui.text(name);
                // TODO -- Should just special case these descriptions for now
            }

            token.end();
        }
    }

    /// Opens a workspace window,
    ///
    pub fn workspace_window(&mut self, world: &World, ui: &Ui) {
        Window::new("Workspace editor")
            .size([700.0, 600.0], imgui::Condition::Appearing)
            .menu_bar(true)
            .build(ui, || {
                ui.menu_bar(|| {
                    ui.menu("Windows", || {
                        let enable_demo_window = self.enable_demo;
                        if imgui::MenuItem::new("Imgui demo window")
                            .selected(enable_demo_window)
                            .build(ui)
                        {
                            self.enable_demo = !enable_demo_window;
                        }
                    })
                });

                if self.enable_demo {
                    ui.show_demo_window(&mut self.enable_demo);
                }
                ui.separator();

                if imgui::CollapsingHeader::new("Hosts")
                    .flags(TreeNodeFlags::NO_TREE_PUSH_ON_OPEN | TreeNodeFlags::DEFAULT_OPEN)
                    .build(ui)
                {
                    if let Some(token) = ui.begin_table("hosts", 2) {
                        ui.table_setup_column("Name");
                        ui.table_setup_column("Controls");
                        ui.table_headers_row();

                        ui.table_next_row();
                        ui.table_next_column();
                        let tree_node = TreeNode::new("main_host")
                            .flags(TreeNodeFlags::DEFAULT_OPEN)
                            .label::<String, _>("main".to_string())
                            .push(ui);

                        ui.table_next_column();

                        if let Some(node) = tree_node {
                            let command_dispatcher = world
                                .system_data::<State>()
                                .plugins()
                                .features()
                                .broker()
                                .command_dispatcher();

                            for (entity, adhoc, _) in
                                world.system_data::<State>().list_adhoc_operations()
                            {
                                ui.table_next_row();
                                ui.table_next_column();
                                if adhoc.tag != "operation" {
                                    ui.text(format!(
                                        "{} ({})",
                                        adhoc.name,
                                        adhoc.tag.trim_end_matches(".operation")
                                    ));
                                } else {
                                    ui.text(adhoc.name);
                                }

                                ui.table_next_column();
                                if ui.button(format!("Spawn {}", entity.id())) {
                                    match command_dispatcher
                                        .try_send((NodeCommand::Spawn(entity), None::<Yielding>))
                                    {
                                        Ok(_) => {}
                                        Err(err) => {
                                            event!(Level::ERROR, "error sending command {err}");
                                        }
                                    }
                                }
                            }
                            node.pop();
                        }

                        for guest in world.system_data::<Runner>().guests() {
                            ui.table_next_row();
                            ui.table_next_column();
                            let tree_node =
                                TreeNode::new(format!("Guest {}", guest.owner.id())).push(ui);

                            ui.table_next_column();
                            if let Some(node) = tree_node {
                                let command_dispatcher = guest
                                    .protocol()
                                    .as_ref()
                                    .system_data::<State>()
                                    .plugins()
                                    .features()
                                    .broker()
                                    .command_dispatcher();

                                for (entity, adhoc, _) in guest
                                    .protocol()
                                    .as_ref()
                                    .system_data::<State>()
                                    .list_adhoc_operations()
                                {
                                    ui.table_next_row();
                                    ui.table_next_column();
                                    if adhoc.tag != "operation" {
                                        ui.text(format!(
                                            "{} ({})",
                                            adhoc.name,
                                            adhoc.tag.trim_end_matches(".operation")
                                        ));
                                    } else {
                                        ui.text(adhoc.name);
                                    }

                                    ui.table_next_column();
                                    if ui.button(format!("Spawn {}", entity.id())) {
                                        match command_dispatcher.try_send((
                                            NodeCommand::Spawn(entity),
                                            None::<Yielding>,
                                        )) {
                                            Ok(_) => {}
                                            Err(err) => {
                                                event!(Level::ERROR, "error sending command {err}");
                                            }
                                        }
                                    }
                                }

                                node.pop();
                            }
                        }

                        token.end();
                    }
                }

                if imgui::CollapsingHeader::new("Plugins").build(ui) {
                    self.plugins(world, ui);
                }

                if imgui::CollapsingHeader::new("Node command handlers").build(ui) {
                    self.custom_node_handlers(world, ui);
                }
            });
    }
}

impl Extension for WorkspaceEditor {
    fn on_ui(&'_ mut self, world: &specs::World, ui: &'_ imgui::Ui<'_>) {
        self.handle_clipboard(ui);
        self.workspace_window(world, ui);

        {
            for guest in world.system_data::<Runner>().guests() {
                let mut guest_editor = guest.guest_editor();
                let title = format!(
                    "Guest {}",
                    self.appendix
                        .name(&guest.owner)
                        .unwrap_or(format!("{}", guest.owner.id()).as_str())
                );

                Window::new("Workspace editor")
                    .menu_bar(true)
                    .build(ui, || {
                        ui.menu_bar(|| {
                            ui.menu("Windows", || {
                                ui.menu("Guests", || {
                                    if imgui::MenuItem::new(format!("{}", title))
                                        .selected(guest_editor.events_window_opened())
                                        .build(ui)
                                    {
                                        if guest_editor.events_window_opened() {
                                            guest_editor.close_event_window();
                                        } else {
                                            guest_editor.open_event_window();
                                        }
                                    }
                                });
                            })
                        })
                    });

                guest_editor.events_window(
                    format!(
                        "Guest {}",
                        self.appendix
                            .name(&guest.owner)
                            .unwrap_or(format!("{}", guest.owner.id()).as_str())
                    ),
                    ui,
                );
                guest_editor.run_now(guest.protocol().as_ref());
                for n in guest.iter_nodes() {
                    if let Some(display) = n.display {
                        display(n, ui);
                    }
                }

                guest.run();
                guest.maintain();
            }
        }

        {
            for guest in (&mut world.write_component::<Guest>()).join() {
                for n in guest.iter_nodes_mut() {
                    let active = if let Some(edit) = n.edit.as_mut() {
                        edit(n, ui)
                    } else {
                        true
                    };

                    if !active {
                         if let Some(edit) = n.edit.take() {
                            n.suspended_edit = Some(edit);
                         }
                    }
                }
                guest.handle();
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
