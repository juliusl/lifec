use std::collections::BTreeMap;

use atlier::system::{Attribute, Value};
use serde::{Serialize, Deserialize};
use specs::{Entity, Component, storage::HashMapStorage};

#[derive(Debug, Default, Component, Clone, Hash, Serialize, Deserialize, PartialEq, PartialOrd)]
#[storage(HashMapStorage)]
pub struct AttributeGraph {
    entity: u32,
    index: BTreeMap<String, Attribute>,
}

impl From<Entity> for AttributeGraph {
    fn from(entity: Entity) -> Self {
        AttributeGraph {
            entity: entity.id(),
            index: BTreeMap::default(),
        }
    }
}

impl AttributeGraph {
    /// updates the parent entity id of the graph 
    pub fn set_parent_entity(&mut self, parent: Entity) {
        self.set_parent_entity_id(parent.id());
    }

    /// sets the current parent entity id
    /// the parent entity id is used when adding attributes to the graph
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

    /// import an attribute that can have a different entity id
    /// if the external_attribute has the same id as parent entity, this will instead be a no-op
    /// This is to enforce that attributes should be added with the below api's
    pub fn import_attribute(&mut self, external_attribute: &Attribute) {
        if external_attribute.id() == self.entity {
            eprintln!("Warning: Trying to import an attribute that is not external to this graph, add this attribute by value");
            return;
        }
        self.add_attribute(external_attribute.clone());
    }

    /// copies an attribute and add's it as being owned by the parent entity
    pub fn copy_attribute(&mut self, external_attribute: &Attribute) {
        let mut copy = external_attribute.clone();
        copy.set_id(self.entity);

        self.add_attribute(copy);
    }

    /// removes an attribute from the index, returns the removed attribute
    pub fn remove(&mut self, attr: &Attribute) -> Option<Attribute> {
        self.index.remove(&attr.to_string())
    }

    /// clears the attribute index
    pub fn clear_index(&mut self) {
        self.index.clear();
    }

    /// returns true if the index is empty
    pub fn is_index_empty(&self) -> bool {
        self.index.is_empty()
    }

    pub fn iter_mut_attributes(&mut self) -> impl Iterator<Item = &mut Attribute> {
        self.index.iter_mut().map(|(_, a)| a)
    }

    pub fn iter_attributes(&self) -> impl Iterator<Item = &Attribute> {
        self.index.values().into_iter()
    }

    pub fn get_attr_value(&self, with_name: impl AsRef<str>) -> Option<&Value> {
        self.get_attr(with_name).and_then(|a| Some(a.value()))
    }

    pub fn get_attr_value_mut(&mut self, with_name: impl AsRef<str>) -> Option<&mut Value> {
        self.get_attr_mut(with_name)
            .and_then(|a| Some(a.get_value_mut()))
    }

    pub fn get_attr(&self, with_name: impl AsRef<str>) -> Option<&Attribute> {
        self.iter_attributes()
            .filter(|attr| attr.id() == self.entity)
            .find(|attr| attr.name() == with_name.as_ref())
            .and_then(|a| Some(a))
    }

    pub fn get_attr_mut(&mut self, with_name: impl AsRef<str>) -> Option<&mut Attribute> {
        let current_id = self.entity;
        self.iter_mut_attributes()
            .filter(|attr| attr.id() == current_id)
            .find(|attr| attr.name() == with_name.as_ref())
            .and_then(|a| Some(a))
    }
    
    pub fn with_empty(&mut self, name: impl AsRef<str>) -> Self {
        self.with(name, Value::Empty)
    }

    pub fn with_symbol(&mut self, name: impl AsRef<str>, symbol: impl AsRef<str>) -> Self {
        self.with(name, Value::Symbol(symbol.as_ref().to_string()))
    }

    pub fn with_text(&mut self, name: impl AsRef<str>, init_value: impl AsRef<str>) -> Self {
        self.with(name, Value::TextBuffer(init_value.as_ref().to_string()))
    }

    pub fn with_int(&mut self, name: impl AsRef<str>, init_value: i32) -> Self {
        self.with(name, Value::Int(init_value))
    }

    pub fn with_float(&mut self, name: impl AsRef<str>, init_value: f32) -> Self {
        self.with(name, Value::Float(init_value))
    }

    pub fn with_bool(&mut self, name: impl AsRef<str>, init_value: bool) -> Self {
        self.with(name, Value::Bool(init_value))
    }

    pub fn with_float_pair(&mut self, name: impl AsRef<str>, init_value: &[f32; 2]) -> Self {
        self.with(name, Value::FloatPair(init_value[0], init_value[1]))
    }

    pub fn with_int_pair(&mut self, name: impl AsRef<str>, init_value: &[i32; 2]) -> Self {
        self.with(name, Value::IntPair(init_value[0], init_value[1]))
    }

    pub fn with_int_range(&mut self, name: impl AsRef<str>, init_value: &[i32; 3]) -> Self {
        self.with(
            name,
            Value::IntRange(init_value[0], init_value[1], init_value[2]),
        )
    }

    pub fn with_float_range(&mut self, name: impl AsRef<str>, init_value: &[f32; 3]) -> Self {
        self.with(
            name,
            Value::FloatRange(init_value[0], init_value[1], init_value[2]),
        )
    }

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

    pub fn add_reference(&mut self, name: impl AsRef<str>, init_value: impl Into<u64>) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::Reference(init_value.into()),
        ));
    }

    pub fn add_symbol(&mut self, name: impl AsRef<str>, symbol: impl AsRef<str>) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::Symbol(symbol.as_ref().to_string()),
        ));
    }

    pub fn add_empty_attr(&mut self, name: impl AsRef<str>) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::Empty,
        ));
    }

    pub fn add_binary_attr(&mut self, name: impl AsRef<str>, init_value: impl Into<Vec<u8>>) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::BinaryVector(init_value.into()),
        ));
    }

    pub fn add_text_attr(&mut self, name: impl AsRef<str>, init_value: impl AsRef<str>) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::TextBuffer(init_value.as_ref().to_string()),
        ));
    }

    pub fn add_int_attr(&mut self, name: impl AsRef<str>, init_value: i32) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::Int(init_value),
        ));
    }

    pub fn add_float_attr(&mut self, name: impl AsRef<str>, init_value: f32) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::Float(init_value),
        ));
    }

    pub fn add_bool_attr(&mut self, name: impl AsRef<str>, init_value: bool) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::Bool(init_value),
        ));
    }

    pub fn add_float_pair_attr(&mut self, name: impl AsRef<str>, init_value: &[f32; 2]) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::FloatPair(init_value[0], init_value[1]),
        ));
    }

    pub fn add_int_pair_attr(&mut self, name: impl AsRef<str>, init_value: &[i32; 2]) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::IntPair(init_value[0], init_value[1]),
        ));
    }

    pub fn add_int_range_attr(&mut self, name: impl AsRef<str>, init_value: &[i32; 3]) {
        self.add_attribute(Attribute::new(
            self.entity,
            name.as_ref().to_string(),
            Value::IntRange(init_value[0], init_value[1], init_value[2]),
        ));
    }

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