
use std::{collections::{HashMap, hash_map::DefaultHasher}, hash::{Hash, Hasher}};

use atlier::system::{Extension};
use specs::{WorldExt, Entity};
pub use tokio::sync::broadcast::{channel, Receiver, Sender};
use tracing::{event, Level};

use crate::{engine::Engines, prelude::WorkspaceConfig, state::AttributeGraph};

use super::Appendix;

/// Extension
/// 
#[derive(Default)]
pub struct WorkspaceEditor {
    enable_demo: bool, 
    appendix: Appendix,
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
                    },
                    crate::engine::EngineStatus::Active(e) => {
                        if let Some(name) = self.appendix.name(&e) {
                            ui.text(format!("engine: {name}"));
                            ui.text(format!("status: active"));
                            ui.text(format!("id: {}", e.id()));
                        }
                    },
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
                    value.edit(|value| {
                        AttributeGraph::edit_value(format!("{name} {id}"), value, ui);
                    }, 
                    |list| {
                        for (idx, value) in list.iter_mut().enumerate() {
                            AttributeGraph::edit_value(format!("{name} {id}.{idx}"), value, ui);
                        }
                    }, || {
                        None
                    });
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
    }
}

impl From<Appendix> for WorkspaceEditor {
    fn from(appendix: Appendix) -> Self {
        Self { enable_demo: false, appendix }
    }
}