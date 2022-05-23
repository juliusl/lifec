use std::fmt::Display;

use atlier::system::{Attribute, Value};
use knot::store::Store;
use serde::{Deserialize, Serialize};
use specs::Component;
use specs::storage::DefaultVecStorage;

use crate::RuntimeState;

use super::EventComponent;

#[derive(Clone, Default, Serialize, Deserialize, Component)]
#[storage(DefaultVecStorage)]
pub struct EventGraph(pub knot::store::Store<EventComponent>);

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