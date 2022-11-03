use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    time::Duration,
};

use atlier::system::Extension;
use specs::{Entity, Join, RunNow, WorldExt};
pub use tokio::sync::broadcast::{channel, Receiver, Sender};
use tokio::time::Instant;
use tracing::{event, Level};

use crate::{
    engine::{Engines, Runner},
    prelude::WorkspaceConfig,
    state::AttributeGraph,
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
    /// Last run
    last_run: Option<Instant>,
}

impl WorkspaceEditor {
    /// Returns true if can run,
    /// 
    pub fn can_run(&self) -> bool {
        if let Some(last) = self.last_run.as_ref() {
            last.elapsed() <= Duration::from_millis(16)
        } else {
            true
        }
    }
}

impl Extension for WorkspaceEditor {
    fn on_ui(&'_ mut self, world: &specs::World, ui: &'_ imgui::Ui<'_>) {
        ui.checkbox("enable imgui demo window", &mut self.enable_demo);
        if self.enable_demo {
            ui.show_demo_window(&mut self.enable_demo);
        }

        {
            let engines = world.system_data::<Engines>();

            for e in engines.scan_engines() {
                match e {
                    crate::engine::EngineStatus::Inactive(e) => {
                        if let Some(name) = self.appendix.name(&e) {
                            ui.text(format!("engine: {name}"));
                            ui.text(format!("status: inactive"));
                            ui.text(format!("id: {}", e.id()));
                        }
                    }
                    crate::engine::EngineStatus::Active(e) => {
                        if let Some(name) = self.appendix.name(&e) {
                            ui.text(format!("engine: {name}"));
                            ui.text(format!("status: active"));
                            ui.text(format!("id: {}", e.id()));
                        }
                    }
                }
                ui.new_line();
                ui.separator();
            }
        }

        {
            let config = world.system_data::<WorkspaceConfig>();
            let mut hasher = DefaultHasher::new();
            let mut configs = config.scan_root();
            configs.hash(&mut hasher);
            let previous = hasher.finish();

            for config in configs.iter_mut() {
                let id = config.root().name().to_string();
                for (name, value) in config.properties_mut().iter_properties_mut() {
                    value.edit(
                        |value| {
                            AttributeGraph::edit_value(format!("{name} {id}"), value, ui);
                        },
                        |list| {
                            for (idx, value) in list.iter_mut().enumerate() {
                                AttributeGraph::edit_value(format!("{name} {id}.{idx}"), value, ui);
                            }
                        },
                        || None,
                    );
                }
            }

            let mut hasher = DefaultHasher::new();
            configs.hash(&mut hasher);
            let current = hasher.finish();

            if current != previous {
                event!(Level::INFO, "Changed");
            }
        }

        let map = world.read_resource::<HashMap<String, Entity>>();
        for (expression, entity) in map.iter() {
            ui.text(format!("{} - {}", entity.id(), expression));
        }

        // for (_, state) in self.appendix.state.iter_mut() {
        //     state.graph.edit_ui(ui);
        //     state.graph.display_ui(ui);
        //     ui.new_line();
        //     ui.separator();
        // }

        {
            let runner = world.system_data::<Runner>();
            for guest in runner.guests() {
                let mut guest_editor = guest.guest_editor();

                guest_editor.events_window(format!("Guest {} - Events", guest.owner.id()), ui);

                if self.can_run() {
                    guest_editor.run_now(guest.host().world());
                }
            }
        }
    }

    fn on_run(&'_ mut self, world: &specs::World) {
        if !self.can_run() {
            return;
        }

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

            self.last_run = Some(tokio::time::Instant::now());
        }
    }
}

impl From<Appendix> for WorkspaceEditor {
    fn from(appendix: Appendix) -> Self {
        Self {
            enable_demo: false,
            appendix,
            last_run: None,
        }
    }
}
