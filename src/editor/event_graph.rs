use std::{fmt::Display, collections::BTreeSet};

use atlier::system::{Attribute, Value};
use knot::store::{Store, Visitor};
use serde::{Deserialize, Serialize};
use specs::Component;
use specs::storage::DefaultVecStorage;

use crate::{RuntimeState};

use super::EventComponent;

#[derive(Debug, Clone, Default, Serialize, Deserialize, Component)]
#[storage(DefaultVecStorage)]
pub struct EventGraph(pub knot::store::Store<EventComponent>);

impl EventGraph {
    pub fn add_event(&mut self, event: EventComponent) {
        let EventGraph(store) = self; 

        self.0 = store.node(event);
    }

    pub fn events(&self) -> Vec<&EventComponent> {
        let EventGraph(store) = self;

        store.nodes()
    }

    pub fn edit_as_table(&mut self, ui: &imgui:: Ui) {
        let EventGraph(store) = self;

        let mut next = Store::<EventComponent>::default();

        let mut set = BTreeSet::default();
        store.nodes().iter().cloned().for_each(|e| {
            set.insert(e.clone());
        });

        for mut e in set {
            let group = ui.begin_group();
            ui.input_text(format!("on {}", e.label), &mut e.on).build();
            ui.input_text(format!("call {}", e.label), &mut e.call).build();
            ui.input_text(format!("dispatch {}", e.label), &mut e.dispatch).build();
            ui.new_line();
            group.end();
            next = next.node(e);
        }

        *store = next;
    }
}

struct TableVisitor;

impl Visitor<EventComponent> for TableVisitor {
    fn visit(&self, _: &EventComponent, _: &EventComponent) -> bool {
        true
    }
}

impl Display for EventGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "event graph")
    }
}

impl RuntimeState for EventGraph {
    type Error = ();

    fn from_attributes(attributes: Vec<atlier::system::Attribute>) -> Self {
        let mut store = Store::<EventComponent>::default();
        for attr in attributes.iter() {
            if let Value::BinaryVector(val) = attr.value() {
                if let Some(n) = ron::de::from_bytes(val).ok() {
                    store = store.node(n);
                }
            }
        }
        Self(store)
    }

    fn into_attributes(&self) -> Vec<atlier::system::Attribute> {
        let EventGraph(store) = self;

        let mut attrs = vec![];
        for (id, e) in store.nodes().iter().enumerate() {
            let id = id as u32;
            match ron::ser::to_string(e) {
                Ok(s) => {
                    let attr = Attribute::new(id, &e.label, atlier::system::Value::BinaryVector(s.as_bytes().to_vec()));
                    attrs.push(attr);
                },
                Err(_) => {

                },
            }
        }

        attrs
    }

    fn process<S: AsRef<str> + ?Sized>(&self, _: &S) -> Result<Self, Self::Error> {
        Ok(self.clone())
    }
}