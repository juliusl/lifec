use std::{collections::BTreeMap, ops::{Deref, DerefMut}};

use atlier::system::{App, Extension};
use copypasta::{ClipboardContext, ClipboardProvider};
use imgui::{TableColumnFlags, TableColumnSetup, TableFlags, TreeNode, TreeNodeFlags, Ui, Window, ColorEdit};
use reality::{BlockIndex, Value};
use specs::{Join, RunNow, World, WorldExt};
pub use tokio::sync::broadcast::{channel, Receiver, Sender};
use tracing::{event, Level};

use crate::{
    engine::{EngineStatus, NodeCommandHandler, Runner, Yielding, WorkspaceCommand, NodeCommand},
    guest::Guest,
    prelude::{Runtime, State, WorkspaceConfig},
    state::AttributeGraph, debugger::Debugger, appendix::Appendix,
};

use super::NodeStatus;

/// Extension to display workspace editing tools,
///
///
#[derive(Default)]
pub struct WorkspaceEditor {
    /// Enables the imgui demo window
    enable_demo: bool,
    /// Appendix,
    appendix: Appendix,
    /// Clipboard context to enable copy/paste,
    clipboard: Option<ClipboardContext>,
    /// Local copy of workspace config,
    workspace_config: BTreeMap<String, BlockIndex>,
    /// Background clear color
    background: [f32; 4],
}

impl WorkspaceEditor {
    /// Adds workspace config from world,
    ///
    pub fn add_workspace_config(&mut self, world: &World) {
        let workspace_config = world.system_data::<WorkspaceConfig>();

        for config in workspace_config.scan_root() {
            let key = format!("{}-{:?}", config.root().name(), config.root().value());
            self.workspace_config.insert(key, config);
        }
    }
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

    /// List of workspace config,
    ///
    pub fn workspace_config(&mut self, world: &World, ui: &Ui) {
        let config = world.system_data::<WorkspaceConfig>();

        if let Some(token) = ui.begin_table("workspace_config_table", 2) {
            ui.table_setup_column("Name");
            ui.table_setup_column("Tag");
            ui.table_headers_row();

            for (idx, (_, c)) in self.workspace_config.iter_mut().enumerate() {
                match c.root().value() {
                    Value::Symbol(editing) => {
                        ui.table_next_row();
                        ui.table_next_column();

                        let tag = c.root().name();
                        let tree = TreeNode::new(format!("{editing}{idx}"))
                            .label::<String, _>(format!("{editing}"))
                            .push(ui);

                        ui.table_next_column();
                        ui.text(tag.trim_end_matches(".config").trim_end_matches("config"));

                        if let Some(tree) = tree {
                            ui.table_next_row();
                            ui.table_next_column();

                            let can_apply = config.can_apply(&c);

                            ui.disabled(!can_apply, || {
                                let mut graph = AttributeGraph::new(c.clone());
                                let code = graph.hash_code();
                                graph.edit_ui(ui);

                                if code != graph.hash_code() {
                                    config.find_apply(graph.index());

                                    *c = graph.index().to_owned();
                                }
                            });

                            tree.pop();
                        }
                    }
                    _ => {}
                }
            }

            token.end();
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
                    .flags(TreeNodeFlags::NO_TREE_PUSH_ON_OPEN | TreeNodeFlags::DEFAULT_OPEN )
                    .build(ui)
                {
                    if let Some(token) = ui.begin_table("hosts", 2) {
                        ui.table_setup_column("Name");
                        ui.table_setup_column("Controls");
                        ui.table_headers_row();

                        ui.table_next_row();
                        ui.table_next_column();
                        let tree_node = TreeNode::new("main_host")
                            .flags(TreeNodeFlags::DEFAULT_OPEN | TreeNodeFlags::FRAME_PADDING)
                            .label::<String, _>("main".to_string())
                            .push(ui);


                        if let Some(node) = tree_node {
                            let command_dispatcher = world
                                .system_data::<State>()
                                .plugins()
                                .features()
                                .broker()
                                .command_dispatcher();

                            let current_events = world.system_data::<State>().event_nodes();
                            let engines = world.system_data::<State>().engine_nodes();

                            for engine in engines {
                                ui.table_next_row();
                                ui.table_next_column();
                                let tree_node = TreeNode::new(format!(
                                    "{}",
                                    self.appendix
                                        .name(&engine.status.entity())
                                        .unwrap_or_default()
                                ))
                                .flags(TreeNodeFlags::FRAME_PADDING | TreeNodeFlags::NO_TREE_PUSH_ON_OPEN)
                                .push(ui);

                                ui.table_next_column();
                                if let NodeStatus::Engine(EngineStatus::Inactive(_)) = engine.status {
                                    if ui.button(format!("Start {}", engine.status.entity().id())) {
                                        if let Some(e) = engine.sequence.as_ref().and_then(|s| s.peek()) {
                                            match command_dispatcher.try_send((NodeCommand::Activate(e), None::<Yielding>)) {
                                                Ok(_) => {
                                                    event!(Level::DEBUG, "Activating engine, {}", engine.status.entity().id());
                                                },
                                                Err(err) => {
                                                    event!(Level::ERROR, "Could not send command to activate engine, {err}");
                                                },
                                            }
                                        }
                                    }
                                } else {
                                    ui.text("active");
                                }

                                if let Some(tree_node) = tree_node {
                                    if let Some(e) = engine.sequence.as_ref() {
                                        ui.table_next_row();
                                        for e in e.iter_entities() {
                                            ui.table_next_column();
                                            let name = self.appendix.name(&e).unwrap_or_default();
                                            ui.text(format!("{name}"));

                                            ui.table_next_column();
                                            let status = world.system_data::<State>().status(e);
                                            ui.text(format!("{}", status));
                                        }
                                    }
                                    tree_node.end();
                                }
                            }

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

                                let spawned = current_events
                                    .iter()
                                    .filter(|e| e.is_spawned())
                                    .filter_map(|e| e.connection_state)
                                    .filter(|c| c.source() == entity)
                                    .map(|c| format!("{}", c.incoming().id()))
                                    .collect::<Vec<_>>();

                                if !spawned.is_empty() {
                                    ui.same_line();
                                    ui.text(format!("Spawned: {}", spawned.join(",")));
                                }
                            }
                            node.pop();
                        }

                        for guest in world.system_data::<Runner>().guests() {
                            let title = format!(
                                "Guest {}",
                                self.appendix
                                    .name(&guest.owner)
                                    .unwrap_or(format!("{}", guest.owner.id()).as_str())
                            );
                            ui.table_next_row();
                            ui.table_next_column();
                            let tree_node = TreeNode::new(title).push(ui);

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

                if imgui::CollapsingHeader::new("Workspace Config")
                    .flags(TreeNodeFlags::NO_TREE_PUSH_ON_OPEN | TreeNodeFlags::DEFAULT_OPEN)
                    .build(ui)
                {
                    self.workspace_config(world, ui);
                }

                if imgui::CollapsingHeader::new("Plugins").build(ui) {
                    self.plugins(world, ui);
                }

                if imgui::CollapsingHeader::new("Node command handlers").build(ui) {
                    self.custom_node_handlers(world, ui);
                }

                // if imgui::CollapsingHeader::new("Debugger").build(ui) {
                //     if let Some(debugger) = world.read_resource::<Option<Debugger>>().deref() {
                //         debugger.display_ui(ui);
                //     }

                //     for guest in world.system_data::<Runner>().guests() {
                //         if let Some(debugger) = guest.protocol().as_ref().try_fetch::<Option<Debugger>>() {
                //             if let Some(debugger) = debugger.as_ref() {
                //                 debugger.display_ui(ui);
                //             }
                //         }
                //     }
                // }
            });
    }
}

impl Extension for WorkspaceEditor {
    fn configure_app_world(_world: &mut World) {
        _world.insert(wgpu::Color {
            r: 0.02122,
            g: 0.02519,
            b: 0.03434,
            a: 1.0,
        });
    }

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
            }
        }

        {
            for guest in (&mut world.write_component::<Guest>()).join() {
                for n in guest.iter_nodes_mut() {
                    let active = if let Some(edit) = n.edit.as_mut() {
                        edit(n, ui)
                    } else {
                        false
                    };

                    if !active {
                        if let Some(edit) = n.edit.take() {
                            n.suspended_edit = Some(edit);
                        } else {
                            Window::new("Workspace editor")
                                .menu_bar(true)
                                .build(ui, || {
                                    ui.menu_bar(|| {
                                        ui.menu("Windows", || {
                                            ui.menu("Suspended", || {
                                                let name = self
                                                    .appendix
                                                    .name(&n.status.entity())
                                                    .unwrap_or(
                                                        format!("{}", n.status.entity().id())
                                                            .as_str(),
                                                    )
                                                    .to_string();
                                                if imgui::MenuItem::new(format!("Guest {}", name))
                                                    .selected(n.suspended_edit.is_some())
                                                    .build(ui)
                                                {
                                                    if let Some(suspended) = n.suspended_edit.take()
                                                    {
                                                        n.edit = Some(suspended);
                                                    }
                                                }
                                            });
                                        })
                                    })
                                });
                        }
                    }
                }
                guest.handle();
            }
        }

        ui.main_menu_bar(|| {
            ui.menu("Theme", || {
                if ColorEdit::new("Background clear", &mut self.background).build(ui) {
                    let [r, g, b, a] = self.background;
                    let mut clear_color = world.write_resource::<wgpu::Color>();
                    let clear_color = clear_color.deref_mut();
                    *clear_color = wgpu::Color {
                        r: r.into(),
                        g: g.into(),
                        b: b.into(),
                        a: a.into(),
                    };
                }
            })
        })
    }
}

impl From<Appendix> for WorkspaceEditor {
    fn from(appendix: Appendix) -> Self {
        Self {
            enable_demo: false,
            appendix,
            clipboard: None,
            workspace_config: Default::default(),
            background: [0.02122, 0.02519, 0.03434, 1.0]
        }
    }
}
