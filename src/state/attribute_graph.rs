use atlier::system::{Attribute, Value};
use reality::BlockProperties;
use specs::{storage::HashMapStorage, Component, Entity};
use std::{
    collections::{hash_map::DefaultHasher, BTreeMap},
    hash::{Hash, Hasher},
};
use tracing::{event, Level};

use crate::{BlockIndex, BlockProperty, AttributeIndex};

/// Wrapper struct over a block index,
/// 
/// Implements AttributeIndex
///
#[derive(Debug, Default, Component, Clone, Hash, Eq, PartialEq, PartialOrd)]
#[storage(HashMapStorage)]
pub struct AttributeGraph {
    /// Block index 
    /// 
    index: BlockIndex,
    /// Scopes the graph to a child entity
    ///
    child: Option<Entity>,
}

impl AttributeGraph {
    /// Creates an attribute graph over data found in a block,
    ///
    pub fn new(index: BlockIndex) -> Self {
        Self { index, child: None }
    }

    /// Returns the current hash_code of the graph
    /// 
    pub fn hash_code(&self) -> u64 {
        let mut hasher = DefaultHasher::default();

        self.hash(&mut hasher);

        hasher.finish()
    }

    /// Returns some bool if there is a matching name attribute with bool value
    /// 
    pub fn is_enabled(&self, with_name: impl AsRef<str>) -> bool {
        self.find_bool(with_name).unwrap_or_default()
    }

    /// Returns a new graph scoped at the child entity,
    /// 
    /// If the child is not a part of this graph, nothing is returned
    /// 
    pub fn scope(&self, child: Entity) -> Option<AttributeGraph> {
        if let Some(_) = self.index.child_properties(child.id()) {
            let mut clone = self.clone();
            clone.child = Some(child);
            Some(clone)
        } else {
            None
        }
    }

    /// Returns an unscoped graph,
    /// 
    pub fn unscope(&self) -> AttributeGraph {
        let mut clone = self.clone();
        clone.child = None;
        clone
    }

    /// Resolves the properties to use within the current scope,
    /// 
    fn resolve_properties(&self) -> &BlockProperties {
        if let Some(child) = self
            .child
            .and_then(|child| self.index.child_properties(child.id()))
        {
            child
        } else {
            self.index.properties()
        }
    }
}

impl AttributeIndex for AttributeGraph {
    fn entity_id(&self) -> u32 {
        if let Some(child) = self.child {
            child.id()
        } else {
            self.index.root().id()
        }
    }

    fn values(&self) -> BTreeMap<String, Vec<Value>> {
        let mut values = BTreeMap::default();
        for (name, property) in self.resolve_properties().iter_properties() {
            let mut property_values = vec![];

            match property {
                BlockProperty::Single(val) => {
                    property_values.push(val.clone());
                },
                BlockProperty::List(vals) => {
                    let mut vals = vals.iter().cloned().collect();
                    let vals = &mut vals;
                    property_values.append(vals);
                },
                _ => {
                    continue;
                }
            }

            values.insert(name.to_string(), property_values);
        }

        values
    }

    fn hash_code(&self) -> u64 {
        self.hash_code()
    }

    fn find_value(&self, with_name: impl AsRef<str>) -> Option<Value> {
        let search = |property: Option<BlockProperty>| match property {
            Some(property) => match property {
                BlockProperty::Single(value) => Some(value),
                BlockProperty::List(values) => values.first().cloned(),
                BlockProperty::Required => {
                    event!(
                        Level::ERROR,
                        "Required property has not been set, {}",
                        with_name.as_ref()
                    );
                    None
                }
                BlockProperty::Optional => {
                    event!(
                        Level::WARN,
                        "Optional property has not been set, {}",
                        with_name.as_ref()
                    );
                    None
                }
                BlockProperty::Empty => None,
            },
            None => {
                event!(
                    Level::TRACE,
                    "Could not find any property {}",
                    with_name.as_ref()
                );
                None
            }
        };

        let properties = self.resolve_properties();
        match search(properties.property(with_name.as_ref()).cloned()) {
            Some(val) => Some(val),
            None =>  {
                event!(Level::TRACE, "Searching for `{}` from control values", with_name.as_ref());
                self.index.control_values().get(with_name.as_ref()).cloned()
            },
        }
    }

    fn find_values(&self, with_name: impl AsRef<str>) -> Vec<Value> {
        let search = |property: Option<BlockProperty>| match property {
            Some(property) => match property {
                BlockProperty::Single(value) => vec![value],
                BlockProperty::List(values) => values.clone(),
                BlockProperty::Required => {
                    event!(
                        Level::ERROR,
                        "Required property has not been set, {}",
                        with_name.as_ref()
                    );
                    vec![]
                }
                BlockProperty::Optional => {
                    event!(
                        Level::WARN,
                        "Optional property has not been set, {}",
                        with_name.as_ref()
                    );
                    vec![]
                }
                BlockProperty::Empty => {
                    vec![]
                }
            },
            None => {
                event!(
                    Level::TRACE,
                    "Could not find any property {}",
                    with_name.as_ref()
                );
                vec![]
            }
        };

        let properties = self.resolve_properties();
        let mut output = search(properties.property(with_name.as_ref()).cloned());

        if output.is_empty() {
            event!(Level::TRACE, "Searching for `{}` from control values", with_name.as_ref());
            if let Some(val) = self.index.control_values().get(with_name.as_ref()) {
                output.push(val.clone());
            }
        }
        output
    }

    fn add_attribute(&mut self, attr: Attribute) {
        let root = self.index.root().name().to_string();

        let properties = if self.index.root().id() != attr.id() {
            self.index
                .child_properties_mut(attr.id)
                .expect("Trying to add an attribute that is out of context of the current index")
        } else {
            self.index.properties_mut()
        };

        if attr.is_stable() {
            // If added through this with/add functions, then the attribute should 
            // always be stable
            properties.add(attr.name, attr.value.clone());
        } else if let Some((name, value)) = attr.transient {
            let name = name.trim_start_matches(&root);
            properties.add(name, value.clone());
        }
    }

    fn replace_attribute(&mut self, attr: Attribute) {
        let root = self.index.root().name().to_string();

        let properties = if self.index.root().id() != attr.id() {
            self.index
                .child_properties_mut(attr.id)
                .expect("Trying to add an attribute that is out of context of the current index")
        } else {
            self.index.properties_mut()
        };

        if attr.is_stable() {
            // If added through this with/add functions, then the attribute should 
            // always be stable
            properties.set(attr.name, BlockProperty::Single(attr.value.clone()));
        } else if let Some((name, value)) = attr.transient {
            let name = name.trim_start_matches(&root);
            properties.set(name, BlockProperty::Single(value.clone()));
        }
    }
}
