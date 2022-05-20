use std::collections::{BTreeSet, HashMap};

use atlier::system::{Value, App, Extension};
use imgui::Window;
use serde::{Deserialize, Serialize};
use specs::storage::HashMapStorage;
use specs::{Component, Entities, Join, ReadStorage, System, RunNow};

use super::{SectionAttributes, unique_title};

#[derive(Component, Default)]
#[storage(HashMapStorage)]
pub struct AttributeEditor {
    title: String,
    entities: HashMap<u32, AttributeEntry>,
}

#[derive(Serialize, Deserialize)]
pub struct AttributeEntry {
    id: u32,
    references: BTreeSet<(String, u64)>,
}

impl AttributeEditor {
    pub fn new() -> Self {
        let mut init = Self::default();
        init.title = unique_title(<AttributeEditor as App>::name());
        init
    }
}

impl Extension for AttributeEditor {
    fn configure_app_world(_: &mut specs::World) {
    }

    fn configure_app_systems(_: &mut specs::DispatcherBuilder) {
    }

    fn extend_app_world(&mut self, app_world: &specs::World, ui: &imgui::Ui) {
        self.run_now(app_world);
        self.show_editor(ui);
    }
}

impl App for AttributeEditor {
    fn name() -> &'static str {
       "Attribute Editor"
    }

    fn show_editor(&mut self, ui: &imgui::Ui) {
        Window::new(&self.title).size([800.0, 600.0], imgui::Condition::Appearing).build(ui, ||{
            if ui.button("Refresh") {
                self.entities.clear();
            }
    
            self.entities.iter().for_each(|(e, ae)| {
                ui.text(format!("Entity: {}", e));
                for r in ae.references.iter() {
                    ui.text(format!("{:?}", r));
                }
                ui.new_line();
            })
        });
    }
}


impl<'a> System<'a> for AttributeEditor {
    type SystemData = (Entities<'a>, ReadStorage<'a, SectionAttributes>);

    fn run(&mut self, (entities, section_attributes): Self::SystemData) {
        for e in entities.join() {
            match section_attributes.get(e) {
                Some(attributes) => {
                    if let None = self.entities.get(&e.id()) {
                        let entry = AttributeEntry {
                            references: {
                                let mut set = BTreeSet::<(String, u64)>::default();
                                attributes
                                    .get_attrs()
                                    .iter()
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

                        self.entities.insert(e.id(), entry);
                    }
                }
                _ => {}
            }
        }
    }
}
