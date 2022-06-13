use std::collections::{BTreeSet, BTreeMap};

use atlier::system::{App, Extension, Value};
use imgui::Window;
use knot::store::Store;
use serde::{Deserialize, Serialize};
use specs::storage::HashMapStorage;
use specs::{Component, Entities, Join, ReadStorage, RunNow, System, WorldExt, WriteStorage};

use crate::AttributeGraph;

use super::unique_title;

#[derive(Default)]
pub struct AttributeEditor {
    title: String,
    entities: BTreeMap<u32, AttributeComponent>,
}

#[derive(Clone, Default, Serialize, Deserialize, Component)]
#[storage(HashMapStorage)]
pub struct AttributeComponent {
    id: u32,
    references: BTreeSet<(String, u64)>,
    store: Store<Value>,
}

impl AttributeEditor {
    pub fn new() -> Self {
        let mut init = Self::default();
        init.title = unique_title(<AttributeEditor as App>::name());
        init
    }
}

impl Extension for AttributeEditor {
    fn configure_app_world(w: &mut specs::World) {
        w.register::<AttributeComponent>();
    }

    fn configure_app_systems(_: &mut specs::DispatcherBuilder) {}

    fn on_ui(&mut self, app_world: &specs::World, ui: &imgui::Ui) {
        self.run_now(app_world);
        self.show_editor(ui);
    }
}

impl App for AttributeEditor {
    fn name() -> &'static str {
        "Attribute Editor"
    }

    fn show_editor(&mut self, ui: &imgui::Ui) {
        let entities = self.entities.clone();

        entities.iter().for_each(|(e, ae)| {
            Window::new(format!("Entity: {}", e))
            .size([800.0, 600.0], imgui::Condition::Appearing)
            .build(ui, ||{
                if ui.button("Refresh") {
                    self.entities.clear();
                }

                ui.text(format!("Entity: {}", e));
                for r in ae.references.iter() {
                    ui.text(format!("{:?}", r));
                }
                ui.new_line();
            });
        });
    }
}

impl<'a> System<'a> for AttributeEditor {
    type SystemData = (Entities<'a>, ReadStorage<'a, AttributeGraph>,  WriteStorage<'a, AttributeComponent>);

    fn run(&mut self, (entities, attributes, mut attribute_components): Self::SystemData) {
        for e in entities.join() {
            match attributes.get(e) {
                Some(attributes) => {
                    if let Some(true) = attributes.is_enabled("enable attribute editor") {
                        if let None = self.entities.get(&e.id()) {
                            let entry = AttributeComponent {
                                store: {
                                    let mut store = Store::default();
                                    attributes
                                        .iter_attributes()
                                        .map(|a| a.value())
                                        .for_each(|v| {
                                            store = store.node(v.clone());
                                        });
                                    store
                                },
                                references: {
                                    let mut set = BTreeSet::<(String, u64)>::default();
                                    attributes
                                        .iter_attributes()
                                        .filter_map(|a| {
                                            if let Value::Reference(r) = a.value() {
                                                return Some((format!("ref {}", a), *r));
                                            }
    
                                            if let Value::Reference(r) = a.value().to_ref() {
                                                Some((format!("{}", a), r))
                                            } else {
                                                None
                                            }
                                        })
                                        .for_each(|entry| {
                                            set.insert(entry);
                                        });
                                    set
                                },
                                id: e.id(),
                            };
    
                            match attribute_components.insert(e, entry.clone()) {
                                Ok(_) => {
                                    self.entities.insert(e.id(), entry);
                                },
                                Err(err) => eprintln!("Error adding attribute component, {}", err),
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
