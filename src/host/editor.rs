use std::{collections::HashSet, sync::Arc};

use imgui::{ChildWindow, StyleVar, Window};
use specs::Write;

use crate::{
    editor::{Appendix, Node},
    prelude::*,
};

/// Extension trait for Host, that provides functions for opening a GUI editor,
///
pub trait Editor {
    /// Opens this host app with the runtime editor extension,
    ///
    fn open_runtime_editor<P>(self)
    where
        P: Project;

    /// Opens this host app with an extension,
    ///
    fn open<P, E>(self, width: f64, height: f64, extension: E)
    where
        P: Project,
        E: Extension + 'static;
}

impl Editor for Host {
    fn open_runtime_editor<P>(mut self)
    where
        P: Project,
    {
        // Build runtime appendix
        self.world_mut().exec(
            |(entities, events, thunks, mut appendix): (
                Entities,
                ReadStorage<Event>,
                ReadStorage<Thunk>,
                Write<Appendix>,
            )| {
                for (entity, event, thunk) in (&entities, events.maybe(), thunks.maybe()).join() {
                    match (event, thunk) {
                        (None, Some(thunk)) => {
                            appendix.insert_general(entity, thunk);
                        }
                        (Some(event), None) => {
                            appendix.insert_general(entity, event);
                        }
                        _ => {}
                    }
                }
            },
        );

        self.open::<P, _>(1920.0, 1080.0, RuntimeEditor::default())
    }

    fn open<P, E>(mut self, width: f64, height: f64, extension: E)
    where
        P: Project,
        E: Extension + 'static,
    {
        self.prepare::<P>();
        let builder = self.new_dispatcher_builder::<P>();

        let set = {
            let operations = self.world().system_data::<Operations>();
            HashSet::from_iter(operations.scan_root().iter().cloned())
        };

        if let Some(appendix) = self.world_mut().remove::<Appendix>() {
            self.world_mut().insert(Arc::new(appendix));
        }

        // Consume the compiled world
        let world = self.world.take();

        // Open the window
        atlier::prelude::open_window(
            HostEditor::name(),
            width,
            height,
            HostEditor::from(set),
            extension,
            world,
            Some(builder),
        );
    }
}

impl From<HashSet<(String, Event)>> for HostEditor {
    fn from(value: HashSet<(String, Event)>) -> Self {
        Self {
            workspace_operations: value,
            ..Default::default()
        }
    }
}

/// Tool for viewing and interacting with a host,
///
#[derive(Default)]
pub struct HostEditor {
    /// Current nodes,
    ///
    nodes: Vec<Node>,
    /// Available workspace operations to execute,
    ///
    workspace_operations: HashSet<(String, Event)>,
    /// Event tick rate,
    ///
    tick_rate: u64,
    /// Command to execute a serialized tick,
    ///
    tick: Option<()>,
    /// Command to pause the runtime,
    ///
    pause: Option<()>,
    /// Command to reset all events,
    ///
    reset: Option<()>,
}

impl<'a> System<'a> for HostEditor {
    type SystemData = Events<'a>;

    fn run(&mut self, mut events: Self::SystemData) {
        if let Some(_) = self.tick.take() {
            events.serialized_tick();
        }

        if let Some(_) = self.pause.take() {
            if events.can_continue() {
                events.pause();
            } else {
                events.resume();
            }
        }

        if let Some(_) = self.reset.take() {
            events.reset_all();
        }

        self.tick_rate = events.tick_rate();

        // Handle node commands
        // 
        for mut node in self.nodes.drain(..) {
            if let Some(command) = node.command.take() {
                match command {
                    crate::editor::NodeCommand::Activate(event) => {
                        if events.activate(event) {
                            event!(Level::DEBUG, "Activating event {}", event.id());
                        }
                    }
                    crate::editor::NodeCommand::Reset(event) => {
                        if events.reset(event) {
                            event!(Level::DEBUG, "Reseting event {}", event.id());
                        }
                    },
                }
            }
        }

        // Get latest node state,
        //
        for node in events.nodes() {
            self.nodes.push(node);
        }
    }
}

impl App for HostEditor {
    fn name() -> &'static str {
        "Lifec Editor"
    }

    fn edit_ui(&mut self, ui: &imgui::Ui) {
        Window::new("Events")
            .size([580.0, 700.0], imgui::Condition::Appearing)
            .build(ui, || {
                if ui.button("Tick") {
                    self.tick = Some(());
                }

                ui.same_line();
                if ui.button("Pause") {
                    self.pause = Some(());
                }

                ui.same_line();
                if ui.button("Reset All") {
                    self.reset = Some(());
                }
                ui.same_line();
                ui.text(format!("tick rate: {} hz", self.tick_rate));
                ui.separator();
                ui.new_line();

                ChildWindow::new(&format!("Event Section"))
                    .size([250.0, 0.0])
                    .build(ui, || {
                        let frame_padding = ui.push_style_var(StyleVar::FramePadding([8.0, 5.0]));
                        let window_padding =
                            ui.push_style_var(StyleVar::WindowPadding([16.0, 16.0]));
                        for node in self.nodes.iter_mut() {
                            node.edit_ui(ui);
                            ui.new_line();
                        }
                        frame_padding.end();
                        window_padding.end();
                    });
                ui.same_line();
            });
    }

    fn display_ui(&self, ui: &imgui::Ui) {
        Window::new("Events").build(ui, || {
            ChildWindow::new(&format!("Statuses")).build(ui, || {
                let frame_padding = ui.push_style_var(StyleVar::FramePadding([8.0, 5.0]));
                let window_padding = ui.push_style_var(StyleVar::WindowPadding([16.0, 16.0]));
                ui.text("Operations:");
                for (tag, operation) in self.workspace_operations.iter() {
                    ui.text(format!("tag: {tag}, name: {}", operation.symbol()));
                }
                frame_padding.end();
                window_padding.end();
            });
        });
    }
}
