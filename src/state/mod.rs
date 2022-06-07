use std::{collections::BTreeMap, fmt::Display};

use atlier::system::{Attribute, Value};
use serde::{Serialize, Deserialize};
use specs::{Entity, Component, storage::HashMapStorage};

use crate::RuntimeState;

/// Attribute graph indexes attributes for an entity and provides methods for editing attributes
#[derive(Debug, Default, Component, Clone, Hash, Serialize, Deserialize, PartialEq, PartialOrd)]
#[storage(HashMapStorage)]
pub struct AttributeGraph {
    entity: u32,
    index: BTreeMap<String, Attribute>,
}

impl AttributeGraph {
    /// Copies all the values from another graph
    pub fn copy(&mut self, other: &AttributeGraph) {
        other.iter_attributes().for_each(|a| {
            self.copy_attribute(a);
        })
    }

    /// Imports all the values from another graph
    pub fn import(&mut self, other: &AttributeGraph) {
        other.iter_attributes().for_each(|a| {
            self.import_attribute(a);
        })
    }

    /// Returns true if the graph has an attribute w/ name
    pub fn contains_attribute(&self, with_name: impl AsRef<str>) -> bool {
        self.find_attr(with_name).is_some()
    }

    /// Returns some bool if there is a matching name attribute with bool value.
    pub fn is_enabled(&self, with_name: impl AsRef<str>) -> Option<bool> {
        if let Some(Value::Bool(val)) = self.find_attr_value(with_name) {
            Some(*val)
        } else {
            None 
        }
    }

    /// Updates the parent entity id of the graph.
    pub fn set_parent_entity(&mut self, parent: Entity) {
        self.set_parent_entity_id(parent.id());
    }

    /// Sets the current parent entity id.
    /// The parent entity id is used when adding attributes to the graph.
    pub fn set_parent_entity_id(&mut self, entity_id: u32) {
        // Update only attributes that the current parent owns
        // attributes that have a different id are only in the collection as references 
        let current = self.clone();
        let current_id = current.entity;

        current
            .iter_attributes()
            .filter(|a| a.id() == current_id)
            .for_each(|a| {
                if let Some(mut a) = self.remove(&a) {
                    a.set_id(entity_id);
                    self.add_attribute(a);
                }
            });

        // Finally update the id
        self.entity = entity_id;
    }

    /// Import an attribute that can have a different entity id.
    /// If the external_attribute has the same id as parent entity, this will instead be a no-op.
    /// This behavior is to enforce that attributes should be added with the below api's.
    pub fn import_attribute(&mut self, external_attribute: &Attribute) {
        if external_attribute.id() == self.entity {
            eprintln!("Warning: No-Op, Trying to import an attribute that is not external to this graph, add this attribute by value instead");
            return;
        }
        self.add_attribute(external_attribute.clone());
    }

    /// Copies an attribute and add's it as being owned by the parent entity.
    pub fn copy_attribute(&mut self, external_attribute: &Attribute) {
        let mut copy = external_attribute.clone();
        copy.set_id(self.entity);

        self.add_attribute(copy);
    }

    /// Finds and removes an attribute w/ name.
    pub fn find_remove(&mut self, with_name: impl AsRef<str>) -> Option<Attribute> {
        let finding = self.clone();
        let finding = finding.find_attr(with_name);
        if let Some(attr) = finding {
            self.remove(attr)
        } else {
            None
        }
    }

    /// Removes an attribute from the index, returns the removed attribute.
    pub fn remove(&mut self, attr: &Attribute) -> Option<Attribute> {
        self.index.remove(&attr.to_string())
    }

    /// Clears the attribute index.
    pub fn clear_index(&mut self) {
        self.index.clear();
    }

    /// Returns true if the index is empty.
    pub fn is_index_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// Returns a mut iterator over indexed attributes.
    pub fn iter_mut_attributes(&mut self) -> impl Iterator<Item = &mut Attribute> {
        self.index.iter_mut().map(|(_, a)| a)
    }

    /// Returns an iterator over indexed attributes.
    pub fn iter_attributes(&self) -> impl Iterator<Item = &Attribute> {
        self.index.values().into_iter()
    }

    /// Maybe returns the value of an attribute with_name.
    pub fn find_attr_value(&self, with_name: impl AsRef<str>) -> Option<&Value> {
        self.find_attr(with_name)
            .and_then(|a| Some(a.value()))
    }

    /// Maybe returns a mutable value off an attribute with_name.
    pub fn find_attr_value_mut(&mut self, with_name: impl AsRef<str>) -> Option<&mut Value> {
        self.find_attr_mut(with_name)
            .and_then(|a| Some(a.get_value_mut()))
    }

    /// Maybe returns an attribute with_name.
    pub fn find_attr(&self, with_name: impl AsRef<str>) -> Option<&Attribute> {
        self.iter_attributes()
            .filter(|attr| attr.id() == self.entity)
            .find(|attr| attr.name() == with_name.as_ref())
            .and_then(|a| Some(a))
    }

    /// Maybe returns a mutable attribute with_name.
    pub fn find_attr_mut(&mut self, with_name: impl AsRef<str>) -> Option<&mut Attribute> {
        let current_id = self.entity;
        self.iter_mut_attributes()
            .filter(|attr| attr.id() == current_id)
            .find(|attr| attr.name() == with_name.as_ref())
            .and_then(|a| Some(a))
    }
    
    /// Returns self with an empty attribute w/ name.
    pub fn with_empty(&mut self, name: impl AsRef<str>) -> Self {
        self.with(name, Value::Empty)
    }

    /// Returns self with a symbol attribute w/ name.
    pub fn with_symbol(&mut self, name: impl AsRef<str>, symbol: impl AsRef<str>) -> Self {
        self.with(name, Value::Symbol(symbol.as_ref().to_string()))
    }

    /// Returns self with a text buffer attribute w/ name.
    pub fn with_text(&mut self, name: impl AsRef<str>, init_value: impl AsRef<str>) -> Self {
        self.with(name, Value::TextBuffer(init_value.as_ref().to_string()))
    }

    /// Returns self with an int attribute w/ name.
    pub fn with_int(&mut self, name: impl AsRef<str>, init_value: i32) -> Self {
        self.with(name, Value::Int(init_value))
    }

    /// Returns self with a float attribute w/ name.
    pub fn with_float(&mut self, name: impl AsRef<str>, init_value: f32) -> Self {
        self.with(name, Value::Float(init_value))
    }

    /// Returns self with a bool attribute w/ name.
    pub fn with_bool(&mut self, name: impl AsRef<str>, init_value: bool) -> Self {
        self.with(name, Value::Bool(init_value))
    }

    /// Returns self with a float pair attribute w/ name.
    pub fn with_float_pair(&mut self, name: impl AsRef<str>, init_value: &[f32; 2]) -> Self {
        self.with(name, Value::FloatPair(init_value[0], init_value[1]))
    }

    /// Returns self with an int pair attribute w/ name.
    pub fn with_int_pair(&mut self, name: impl AsRef<str>, init_value: &[i32; 2]) -> Self {
        self.with(name, Value::IntPair(init_value[0], init_value[1]))
    }

    /// Returns self with an int range attribute w/ name.
    pub fn with_int_range(&mut self, name: impl AsRef<str>, init_value: &[i32; 3]) -> Self {
        self.with(
            name,
            Value::IntRange(init_value[0], init_value[1], init_value[2]),
        )
    }

    /// Returns self with a float range attribute w/ name.
    pub fn with_float_range(&mut self, name: impl AsRef<str>, init_value: &[f32; 3]) -> Self {
        self.with(
            name,
            Value::FloatRange(init_value[0], init_value[1], init_value[2]),
        )
    }

    /// Add's the value as an attribute to the graph and returns a copy of self
    pub fn with(&mut self, name: impl AsRef<str>, value: Value) -> Self {
        self.update(move |g| match value {
            Value::Empty => {
                g.add_empty_attr(name);
            }
            Value::Symbol(symbol) => {
                g.add_symbol(name, symbol);
            }
            Value::TextBuffer(text_buffer) => {
                g.add_text_attr(name, text_buffer);
            }
            Value::Float(init_value) => {
                g.add_float_attr(name, init_value);
            }
            Value::Int(init_value) => {
                g.add_int_attr(name, init_value);
            }
            Value::Bool(init_value) => {
                g.add_bool_attr(name, init_value);
            }
            Value::IntPair(e0, e1) => {
                g.add_int_pair_attr(name, &[e0, e1]);
            }
            Value::FloatPair(e0, e1) => {
                g.add_float_pair_attr(name, &[e0, e1]);
            }
            Value::FloatRange(value, min, max) => {
                g.add_float_range_attr(name, &[value, min, max]);
            }
            Value::IntRange(value, min, max) => {
                g.add_int_range_attr(name, &[value, min, max]);
            }
            Value::BinaryVector(init_value) => {
                g.add_binary_attr(name, init_value);
            }
            Value::Reference(init_value) => {
                g.add_reference(name, init_value);
            }
        })
    }

    /// Adds a reference attribute w/ init_value and w/ name to index for entity.
    pub fn add_reference(&mut self, name: impl AsRef<str>, init_value: impl Into<u64>) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::Reference(init_value.into()),
        ));
    }
    
    /// Adds a symbol attribute w/ symbol and w/ name to index for entity.
    pub fn add_symbol(&mut self, name: impl AsRef<str>, symbol: impl AsRef<str>) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::Symbol(symbol.as_ref().to_string()),
        ));
    }

    /// Adds an empty attribute w/ name to index for entity.
    pub fn add_empty_attr(&mut self, name: impl AsRef<str>) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::Empty,
        ));
    }

    /// Adds a binary vector attribute w/ name and w/ init_value for entity.
    pub fn add_binary_attr(&mut self, name: impl AsRef<str>, init_value: impl Into<Vec<u8>>) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::BinaryVector(init_value.into()),
        ));
    }

    /// Adds a text buffer attribute w/ name and w/ init_value for entity.
    pub fn add_text_attr(&mut self, name: impl AsRef<str>, init_value: impl AsRef<str>) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::TextBuffer(init_value.as_ref().to_string()),
        ));
    }

    /// Adds an int attribute w/ name and w/ init_value for entity.
    pub fn add_int_attr(&mut self, name: impl AsRef<str>, init_value: i32) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::Int(init_value),
        ));
    }

    /// Adds an float attribute w/ name and w/ init_value for entity.
    pub fn add_float_attr(&mut self, name: impl AsRef<str>, init_value: f32) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::Float(init_value),
        ));
    }

    /// Adds a bool attribute w/ name and w/ init_value for entity.
    pub fn add_bool_attr(&mut self, name: impl AsRef<str>, init_value: bool) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::Bool(init_value),
        ));
    }

    /// Adds a float pair attribute w/ name and w/ init_value for entity.
    pub fn add_float_pair_attr(&mut self, name: impl AsRef<str>, init_value: &[f32; 2]) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::FloatPair(init_value[0], init_value[1]),
        ));
    }

    /// Adds an int pair attribute w/ name and w/ init_value for entity.
    pub fn add_int_pair_attr(&mut self, name: impl AsRef<str>, init_value: &[i32; 2]) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::IntPair(init_value[0], init_value[1]),
        ));
    }

    /// Adds an int range attribute w/ name and w/ init_value for entity.
    pub fn add_int_range_attr(&mut self, name: impl AsRef<str>, init_value: &[i32; 3]) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::IntRange(init_value[0], init_value[1], init_value[2]),
        ));
    }

    /// Adds an float range attribute w/ name and w/ init_value for entity.
    pub fn add_float_range_attr(&mut self, name: impl AsRef<str>, init_value: &[f32; 3]) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::FloatRange(init_value[0], init_value[1], init_value[2]),
        ));
    }

    fn add_attribute(&mut self, attr: Attribute) {
        self.index.insert(attr.to_string(), attr);
    }

    fn update(&mut self, func: impl FnOnce(&mut Self)) -> Self {
        let next = self;

        (func)(next);

        next.to_owned()
    }
}

#[test]
fn test_attribute_graph() {
    let mut test_graph = AttributeGraph::default();

    test_graph.with("test_value", Value::Int(10));

    assert!(test_graph.contains_attribute("test_value"));
    assert_eq!(test_graph.find_attr_value("test_value"), Some(&Value::Int(10)));
}


impl Display for AttributeGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}

impl AsRef<AttributeGraph> for AttributeGraph {
    fn as_ref(&self) -> &AttributeGraph {
        self
    }
}

impl AsMut<AttributeGraph> for AttributeGraph {
    fn as_mut(&mut self) -> &mut AttributeGraph {
        self
    }
}

impl From<Entity> for AttributeGraph {
    fn from(entity: Entity) -> Self {
        AttributeGraph {
            entity: entity.id(),
            index: BTreeMap::default(),
        }
    }
}

impl RuntimeState for AttributeGraph {
    type Error = ();
    type State = Self;

    fn dispatch(&self, _: impl AsRef<str>) -> Result<Self, Self::Error> {
        todo!("dispatcher not implemented")
    }

    fn state(&self) -> &Self::State {
        self
    }

    fn state_mut(&mut self) -> &mut Self::State {
        self
    }
}