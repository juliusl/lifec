use specs::{storage::HashMapStorage, Component};
use atlier::system::{Attribute, Value};
use reality::BlockIndex;
use tracing::{event, Level};
use std::{
    hash::{Hash, Hasher},
    collections::{hash_map::DefaultHasher},
};

mod v2;
pub use v2::AttributeIndex;
pub use v2::Operation;

/// Wrapper struct over a block index,
/// 
#[derive(Debug, Default, Component, Clone, Hash, Eq, PartialEq, PartialOrd)]
#[storage(HashMapStorage)]
pub struct AttributeGraph {
    index: BlockIndex,
}

impl AttributeGraph {
    /// Creates an attribute graph over data found in a block,
    /// 
    pub fn new(index: BlockIndex) -> Self {
        Self { index }
    }
}

impl AttributeIndex for AttributeGraph {
    fn entity_id(&self) -> u32 {
        self.index.root().id()
    }

    fn hash_code(&self) -> u64 {
        self.hash_code()
    }

    fn find_value(&self, with_name: impl AsRef<str>) -> Option<Value> {
        match self.index.find_property(with_name.as_ref()) {
            Some(property) => match property {
                reality::BlockProperty::Single(value) => Some(value),
                reality::BlockProperty::List(values) => {
                    values.first().cloned()
                },
                reality::BlockProperty::Required => { 
                    event!(Level::ERROR, "Required property has not been set, {}", with_name.as_ref());
                    None
                },
                reality::BlockProperty::Optional => {
                    event!(Level::WARN, "Optional property has not been set, {}", with_name.as_ref());
                    None
                },
                reality::BlockProperty::Empty => {
                    None
                },
            },
            None => {
                event!(Level::TRACE, "Could not find any property {}", with_name.as_ref());
                None
            },
        }
    }

    fn find_values(&self, with_name: impl AsRef<str>) -> Vec<Value> {
        match self.index.find_property(with_name.as_ref()) {
            Some(property) => match property {
                reality::BlockProperty::Single(value) => vec![value],
                reality::BlockProperty::List(values) => {
                    values.clone()
                },
                reality::BlockProperty::Required => { 
                    event!(Level::ERROR, "Required property has not been set, {}", with_name.as_ref());
                    vec![]
                },
                reality::BlockProperty::Optional => {
                    event!(Level::WARN, "Optional property has not been set, {}", with_name.as_ref());
                    vec![]
                },
                reality::BlockProperty::Empty => {
                    vec![]
                },
            },
            None => {
                event!(Level::TRACE, "Could not find any property {}", with_name.as_ref());
                vec![]
            },
        }
    }

    fn add_attribute(&mut self, attr: Attribute) {
        let root = self.index.root().name().to_string();

        let properties = if self.entity_id() != attr.id() {
             self.index.child_properties_mut(attr.id).expect("Trying to add an attribute that is out of context of the current index")
        } else {
            self.index.properties_mut()
        };

        if attr.is_stable() {
            properties.add(attr.name, attr.value.clone());
        } else if let Some((name, value)) = attr.transient {
            let name = name.trim_start_matches(&root);
            properties.add(name, value.clone());
        }
    }
}

impl AttributeGraph {
    /// Returns the current hash_code of the graph
    pub fn hash_code(&self) -> u64 {
        let mut hasher = DefaultHasher::default();

        self.hash(&mut hasher);

        hasher.finish()
    }

    /// Returns some bool if there is a matching name attribute with bool value.
    pub fn is_enabled(&self, with_name: impl AsRef<str>) -> bool {
        self.find_bool(with_name).unwrap_or_default()
    }
}

