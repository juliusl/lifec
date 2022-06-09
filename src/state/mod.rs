use std::{collections::BTreeMap, fmt::Display};

use atlier::system::{Attribute, Value};
use logos::Logos;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use specs::{storage::HashMapStorage, Component, Entity};

use crate::{RuntimeDispatcher, RuntimeState};

mod system;
pub use system::AttributeStore;
pub use system::AttributeSystem;

/// Attribute graph is a component that indexes attributes for an entity
/// It is designed to be a general purpose enough to be the common element of runtime state storage
#[derive(Debug, Default, Component, Clone, Hash, Serialize, Deserialize, PartialEq, PartialOrd)]
#[storage(HashMapStorage)]
pub struct AttributeGraph {
    entity: u32,
    index: BTreeMap<String, Attribute>,
}

impl AttributeGraph {
    pub fn entity(&self) -> u32 {
        self.entity
    }

    /// Copies all the values from other graph
    pub fn copy(&mut self, other: &AttributeGraph) {
        other.iter_attributes().for_each(|a| {
            self.copy_attribute(a);
        })
    }

    /// Imports all the values from other graph
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

    /// Finds the value of an attribute by name that is owned by `self.entity`.
    pub fn find_attr_value(&self, with_name: impl AsRef<str>) -> Option<&Value> {
        self.find_attr(with_name).and_then(|a| Some(a.value()))
    }

    /// Finds the mut value of an attribute by name that is owned by `self.entity`.
    pub fn find_attr_value_mut(&mut self, with_name: impl AsRef<str>) -> Option<&mut Value> {
        self.find_attr_mut(with_name)
            .and_then(|a| Some(a.value_mut()))
    }

    /// Finds an attribute by name that is owned by `self.entity`
    pub fn find_attr(&self, with_name: impl AsRef<str>) -> Option<&Attribute> {
        self.iter_attributes()
            .filter(|attr| attr.id() == self.entity)
            .find(|attr| attr.name() == with_name.as_ref())
            .and_then(|a| Some(a))
    }

    /// Finds a mut attribute by name that is owned by `self.entity`
    pub fn find_attr_mut(&mut self, with_name: impl AsRef<str>) -> Option<&mut Attribute> {
        let current_id = self.entity;
        self.iter_mut_attributes()
            .filter(|attr| attr.id() == current_id)
            .find(|attr| attr.name() == with_name.as_ref())
            .and_then(|a| Some(a))
    }

    /// Finds a graph that has been imported by this graph
    pub fn find_imported_graph(&self, id: u32) -> Option<Self> {
        let mut imported = Self::from(id);

        self.iter_attributes()
            .filter(|attr| attr.id() == id)
            .for_each(|a| {
                imported.copy_attribute(a);
            });

        if imported.is_index_empty() {
            None
        } else {
            Some(imported)
        }
    }

    /// Returns self with an empty attribute w/ name.
    pub fn with_empty(&mut self, name: impl AsRef<str>) -> &mut Self {
        self.with(name, Value::Empty)
    }

    /// Returns self with a symbol attribute w/ name.
    pub fn with_symbol(&mut self, name: impl AsRef<str>, symbol: impl AsRef<str>) -> &mut Self {
        self.with(name, Value::Symbol(symbol.as_ref().to_string()))
    }

    /// Returns self with a text buffer attribute w/ name.
    pub fn with_text(&mut self, name: impl AsRef<str>, init_value: impl AsRef<str>) -> &mut Self {
        self.with(name, Value::TextBuffer(init_value.as_ref().to_string()))
    }

    /// Returns self with an int attribute w/ name.
    pub fn with_int(&mut self, name: impl AsRef<str>, init_value: i32) -> &mut Self {
        self.with(name, Value::Int(init_value))
    }

    /// Returns self with a float attribute w/ name.
    pub fn with_float(&mut self, name: impl AsRef<str>, init_value: f32) -> &mut Self {
        self.with(name, Value::Float(init_value))
    }

    /// Returns self with a bool attribute w/ name.
    pub fn with_bool(&mut self, name: impl AsRef<str>, init_value: bool) -> &mut Self {
        self.with(name, Value::Bool(init_value))
    }

    /// Returns self with a float pair attribute w/ name.
    pub fn with_float_pair(&mut self, name: impl AsRef<str>, init_value: &[f32; 2]) -> &mut Self {
        self.with(name, Value::FloatPair(init_value[0], init_value[1]))
    }

    /// Returns self with an int pair attribute w/ name.
    pub fn with_int_pair(&mut self, name: impl AsRef<str>, init_value: &[i32; 2]) -> &mut Self {
        self.with(name, Value::IntPair(init_value[0], init_value[1]))
    }

    /// Returns self with an int range attribute w/ name.
    pub fn with_int_range(&mut self, name: impl AsRef<str>, init_value: &[i32; 3]) -> &mut Self {
        self.with(
            name,
            Value::IntRange(init_value[0], init_value[1], init_value[2]),
        )
    }

    /// Returns self with a float range attribute w/ name.
    pub fn with_float_range(&mut self, name: impl AsRef<str>, init_value: &[f32; 3]) -> &mut Self {
        self.with(
            name,
            Value::FloatRange(init_value[0], init_value[1], init_value[2]),
        )
    }

    /// Add's a value and returns self to make these api's chainable
    pub fn with(&mut self, name: impl AsRef<str>, value: Value) -> &mut Self {
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

    fn update(&mut self, func: impl FnOnce(&mut Self)) -> &mut Self {
        (func)(self);
        self
    }
}

#[test]
fn test_attribute_graph() {
    let mut test_graph = AttributeGraph::default();

    test_graph
        .with("test_value", Value::Int(10))
        .with("test_float", Value::Float(10.0));

    assert!(test_graph.contains_attribute("test_value"));
    assert_eq!(
        test_graph.find_attr_value("test_value"),
        Some(&Value::Int(10))
    );
    assert!(test_graph.contains_attribute("test_float"));
    assert_eq!(
        test_graph.find_attr_value("test_float"),
        Some(&Value::Float(10.0))
    );
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
        Self::from(entity.id())
    }
}

impl From<u32> for AttributeGraph {
    fn from(entity_id: u32) -> Self {
        AttributeGraph {
            entity: entity_id,
            index: BTreeMap::default(),
        }
    }
}

impl RuntimeState for AttributeGraph {
    type Dispatcher = Self;

    /// Try to serialize self to string in .ron format.
    fn save(&self) -> Option<String> {
        ron::ser::to_string_pretty(self, PrettyConfig::new()).ok()
    }

    /// Try to load self from .ron formatted string.
    fn load(&self, init: impl AsRef<str>) -> Self {
        if let Some(state) = ron::from_str(init.as_ref()).ok() {
            state
        } else {
            Self::default()
        }
    }

    /// Returns dispatcher for runtime state
    fn dispatcher(&self) -> &Self::Dispatcher {
        self
    }

    /// Returns mutable dispatcher for runtime state
    fn dispatcher_mut(&mut self) -> &mut Self::Dispatcher {
        self
    }
}

impl RuntimeDispatcher for AttributeGraph {
    type Error = AttributeGraphErrors;

    /// dispatch_mut is a function that should take a string message that can mutate state
    /// and returns a result
    fn dispatch_mut(&mut self, msg: impl AsRef<str>) -> Result<(), Self::Error> {
        let mut event_lexer = AttributeGraphEvents::lexer(msg.as_ref());

        match event_lexer.next() {
            Some(event) => match event {
                AttributeGraphEvents::Add => self.on_add(event_lexer.remainder()),
                AttributeGraphEvents::FindRemove => self.on_find_remove(event_lexer.remainder()),
                AttributeGraphEvents::Import => self.on_import(event_lexer.remainder()),
                AttributeGraphEvents::Copy => self.on_copy(event_lexer.remainder()),
                AttributeGraphEvents::Define => self.on_define(event_lexer.remainder()),
                AttributeGraphEvents::Commit => self.on_commit(event_lexer.remainder()),
                AttributeGraphEvents::Edit => self.on_edit(event_lexer.remainder()),
                AttributeGraphEvents::Error => Err(AttributeGraphErrors::UnknownEvent),
            },
            None => Err(AttributeGraphErrors::EmptyMessage),
        }
    }
}

/// These are handlers for dispatched messages
impl AttributeGraph {
    fn on_edit(&mut self, msg: impl AsRef<str>) -> Result<(), AttributeGraphErrors> {
        let mut element_lexer = AttributeGraphElements::lexer(msg.as_ref());
        match (element_lexer.next(), element_lexer.next(), element_lexer.next()) {
            (Some(AttributeGraphElements::Symbol(name)), Some(AttributeGraphElements::Symbol(new_name)), Some(value)) => match value {
                AttributeGraphElements::Text(value)
                | AttributeGraphElements::Int(value)
                | AttributeGraphElements::Bool(value)
                | AttributeGraphElements::IntPair(value)
                | AttributeGraphElements::IntRange(value)
                | AttributeGraphElements::Float(value)
                | AttributeGraphElements::FloatPair(value)
                | AttributeGraphElements::FloatRange(value) => {
                    if let Some(attr) = self.find_attr_mut(name) {
                        attr.edit((new_name, value));
                    }
                    Ok(())
                }
                AttributeGraphElements::Empty => {
                    if let Some(attr) = self.find_attr_mut(name) {
                        attr.edit((new_name, Value::Empty));
                    }
                    Ok(())
                }
                AttributeGraphElements::Entity(_) => todo!("value type unknown"),
                AttributeGraphElements::Symbol(_) => todo!("unrecognized element"),
                AttributeGraphElements::Error => todo!("error parsing next value"),
            },
            _ => Err(AttributeGraphErrors::NotEnoughArguments),
        }
    }

    fn on_commit(&mut self, msg: impl AsRef<str>) -> Result<(), AttributeGraphErrors> {
        let mut element_lexer = AttributeGraphElements::lexer(msg.as_ref());
        match (element_lexer.next(), element_lexer.next()) {
            (Some(AttributeGraphElements::Symbol(name)), Some(AttributeGraphElements::Symbol(symbol))) =>{
                let symbol_attr_name = format!("{}::{}", name, symbol);
                if let Some(transient) = self.clone().find_attr(symbol_attr_name).and_then(|a| a.transient()) {
                    if let Some(to_edit) = self.find_attr_mut(name) {
                        to_edit.edit(transient.clone());
                        to_edit.commit();
                    }
                }

                Ok(())
            },
            _ => Err(AttributeGraphErrors::NotEnoughArguments),
        }
    }

    fn on_define(&mut self, msg: impl AsRef<str>) -> Result<(), AttributeGraphErrors> {
        let mut element_lexer = AttributeGraphElements::lexer(msg.as_ref());
        match (element_lexer.next(), element_lexer.next()) {
            (Some(AttributeGraphElements::Symbol(name)), Some(AttributeGraphElements::Symbol(symbol))) =>{
                self.add_symbol(format!("{}::{}", name, symbol), format!("{}::", symbol));
                Ok(())
            },
            _ => Err(AttributeGraphErrors::NotEnoughArguments),
        }
    }

    fn on_add(&mut self, msg: impl AsRef<str>) -> Result<(), AttributeGraphErrors> {
        let mut element_lexer = AttributeGraphElements::lexer(msg.as_ref());
        match (element_lexer.next(), element_lexer.next()) {
            (Some(AttributeGraphElements::Symbol(name)), Some(value)) => match value {
                AttributeGraphElements::Text(value)
                | AttributeGraphElements::Int(value)
                | AttributeGraphElements::Bool(value)
                | AttributeGraphElements::IntPair(value)
                | AttributeGraphElements::IntRange(value)
                | AttributeGraphElements::Float(value)
                | AttributeGraphElements::FloatPair(value)
                | AttributeGraphElements::FloatRange(value) => {
                    self.with(name, value);
                    Ok(())
                }
                AttributeGraphElements::Empty => {
                    self.with_empty(name);
                    Ok(())
                }
                AttributeGraphElements::Entity(_) => todo!("value type unknown"),
                AttributeGraphElements::Symbol(_) => todo!("unrecognized element"),
                AttributeGraphElements::Error => todo!("error parsing next value"),
            },
            _ => Err(AttributeGraphErrors::NotEnoughArguments),
        }
    }

    fn on_find_remove(&mut self, msg: impl AsRef<str>) -> Result<(), AttributeGraphErrors> {
        let mut element_lexer = AttributeGraphElements::lexer(msg.as_ref());
        match element_lexer.next() {
            Some(AttributeGraphElements::Symbol(attr_name)) => {
                if let Some(removed) = self.find_remove(&attr_name) {
                    eprintln!("Removed {}", removed);
                } else {
                    eprintln!("Attribute not found {}", &attr_name);
                }
                Ok(())
            }
            _ => Err(AttributeGraphErrors::NotEnoughArguments),
        }
    }

    fn on_import(&mut self, msg: impl AsRef<str>) -> Result<(), AttributeGraphErrors> {
        let mut element_lexer = AttributeGraphElements::lexer(msg.as_ref());
        match (
            element_lexer.next(),
            element_lexer.next(),
            element_lexer.next(),
        ) {
            (
                Some(AttributeGraphElements::Entity(entity)),
                Some(AttributeGraphElements::Symbol(name)),
                Some(value),
            ) => match value {
                AttributeGraphElements::Text(value)
                | AttributeGraphElements::Int(value)
                | AttributeGraphElements::Bool(value)
                | AttributeGraphElements::IntPair(value)
                | AttributeGraphElements::IntRange(value)
                | AttributeGraphElements::Float(value)
                | AttributeGraphElements::FloatPair(value)
                | AttributeGraphElements::FloatRange(value) => {
                    self.import_attribute(&Attribute::new(entity, name, value));
                    Ok(())
                }
                AttributeGraphElements::Empty => {
                    Err(AttributeGraphErrors::CannotImportEmptyAttribute)
                }
                AttributeGraphElements::Entity(_)
                | AttributeGraphElements::Symbol(_)
                | AttributeGraphElements::Error => {
                    Err(AttributeGraphErrors::IncorrectMessageFormat)
                }
            },
            _ => Err(AttributeGraphErrors::NotEnoughArguments),
        }
    }

    fn on_copy(&mut self, msg: impl AsRef<str>) -> Result<(), AttributeGraphErrors> {
        let mut element_lexer = AttributeGraphElements::lexer(msg.as_ref());
        match (element_lexer.next(), element_lexer.next()) {
            (
                Some(AttributeGraphElements::Entity(entity)),
                Some(AttributeGraphElements::Symbol(name)),
            ) => {
                if let Some(imported) = self.find_imported_graph(entity) {
                    if let Some(to_copy) = imported.find_attr(name) {
                        self.copy_attribute(to_copy);
                    }
                }
                Ok(())
            }
            (Some(AttributeGraphElements::Entity(entity)), None) => {
                if let Some(imported) = self.find_imported_graph(entity) {
                    self.copy(&imported);
                }
                Ok(())
            }
            _ => Err(AttributeGraphErrors::NotEnoughArguments),
        }
    }
}

#[test]
fn test_attribute_graph_dispatcher() {
    let mut graph = AttributeGraph::from(0);

    let test_messages = r#"
    add test_attr             .TEXT testing text attr
    add test_attr_empty       .EMPTY
    add test_attr_bool        .BOOL true
    add test_attr_int         .INT 510982
    add test_attr_int_pair    .INT_PAIR 5000, 1200
    add test_attr_int_range   .INT_RANGE 500, 0, 1000
    add test_attr_float       .FLOAT 510982.12
    add test_attr_float_pair  .FLOAT_PAIR 5000.0, 1200.12
    add test_attr_float_range .FLOAT_RANGE 500.0, 0.0, 1000.0
    define test_attr node
    "#;

    for message in test_messages.trim().split("\n") {
        assert!(graph.dispatch_mut(message).is_ok());
    }

    assert!(graph.contains_attribute("test_attr"));
    assert!(graph.contains_attribute("test_attr_int"));
    assert!(graph.contains_attribute("test_attr_int_pair"));
    assert!(graph.contains_attribute("test_attr_int_range"));
    assert!(graph.contains_attribute("test_attr_float"));
    assert!(graph.contains_attribute("test_attr_float_pair"));
    assert!(graph.contains_attribute("test_attr_float_range"));
    assert!(graph.contains_attribute("test_attr_empty"));
    assert!(graph.contains_attribute("test_attr_bool"));
    assert!(graph.contains_attribute("test_attr::node"));

    // test graph state
    assert_eq!(
        Some(&Value::TextBuffer("testing text attr".to_string())),
        graph.find_attr_value("test_attr")
    );

    // test edit/commit symbols 
    let test_messages = r#"
    edit test_attr::node test_attr .TEXT testing commit attr
    commit test_attr node
    "#;
    
    for message in test_messages.trim().split("\n") {
        assert!(graph.dispatch_mut(message).is_ok());
    }

    assert!(graph.contains_attribute("test_attr"));
    assert_eq!(
        Some(&Value::TextBuffer("testing commit attr".to_string())),
        graph.find_attr_value("test_attr")
    );

    assert_eq!(
        Some(&Value::Bool(true)),
        graph.find_attr_value("test_attr_bool")
    );
    
    assert_eq!(
        Some(&Value::Empty),
        graph.find_attr_value("test_attr_empty")
    );

    assert_eq!(
        Some(&Value::Int(510982)),
        graph.find_attr_value("test_attr_int")
    );
    
    assert_eq!(
        Some(&Value::IntPair(5000, 1200)),
        graph.find_attr_value("test_attr_int_pair")
    );
    
    assert_eq!(
        Some(&Value::IntRange(500, 0, 1000)),
        graph.find_attr_value("test_attr_int_range")
    );

    assert_eq!(
        Some(&Value::Float(510982.12)),
        graph.find_attr_value("test_attr_float")
    );
    
    assert_eq!(
        Some(&Value::FloatPair(5000.0, 1200.12)),
        graph.find_attr_value("test_attr_float_pair")
    );
    
    assert_eq!(
        Some(&Value::FloatRange(500.0, 0.0, 1000.0)),
        graph.find_attr_value("test_attr_float_range")
    );

    assert_eq!(
        Some(&Value::Symbol("node::".to_string())),
        graph.find_attr_value("test_attr::node")
    );

    // Test find_remove
    assert!(graph.dispatch_mut("find_remove test_attr").is_ok());
    assert!(!graph.contains_attribute("test_attr"));

    // Test import/copy save/load
    assert!(graph
        .dispatch_mut("import 10 test_attr .TEXT this value is imported")
        .is_ok());

    // Find and validate the graph after it has been imported
    if let Some(imported) = graph.find_imported_graph(10) {
        assert!(imported.contains_attribute("test_attr"));
        assert_eq!(
            Some(&Value::TextBuffer("this value is imported".to_string())),
            imported.find_attr_value("test_attr")
        );
        println!(
            "# Imported\n {}",
            imported
                .save()
                .expect("should be able to save imported attribute graph")
        );

        assert!(graph.dispatch_mut("copy 10 test_attr").is_ok());
        assert!(graph.contains_attribute("test_attr"));
        assert_eq!(
            Some(&Value::TextBuffer("this value is imported".to_string())),
            graph.find_attr_value("test_attr")
        );
    } else {
        assert!(false, "could not find imported graph");
    }

    println!(
        "# Result\n {}",
        graph
            .save()
            .expect("should be able to save an attribute graph")
    );
}

pub enum AttributeGraphErrors {
    UnknownEvent,
    NotEnoughArguments,
    IncorrectMessageFormat,
    CannotImportEmptyAttribute,
    EmptyMessage,
}

#[derive(Logos, Debug, Hash, Clone, PartialEq, PartialOrd)]
pub enum AttributeGraphEvents {
    /// Usage: add {`attribute-name`} {`value-type`} {`remaining as value`}
    /// Example: add test_attr .TEXT remaining text is text
    /// Adds a new attribute to the graph. Types omitted from this event are symbol, reference, and binary-vector
    #[token("add")]
    Add,
    /// Usage: find_remove {`attribute-name`}
    /// Example: find_remove test_attr
    /// Finds and removes an attribute from the graph by attribute-name
    #[token("find_remove")]
    FindRemove,
    /// Usage: import {`external entity id`} {`attribute-name`} {`value-type token`} {`remaining is parsed corresponding to value-type token`}
    /// Example: import 10 test_attr .TEXT remaining text is text
    /// Imports an attribute to the graph, Types omitted from this event are symbol, reference, and binary-vector
    #[token("import")]
    Import,
    /// Usage: copy {`external entity id`} {`attribute-name`}
    /// Examples: copy 10 test_attr
    ///           copy 10
    /// Copies imported attribute/s to the graph. Types omitted from this event are symbol, reference, and binary-vector
    /// Implementation requires that attributes are imported to the graph before copy message is handled
    #[token("copy")]
    Copy,
    /// Usage: define {`attribute-name`} {`symbol-name`}
    /// Examples: define test_attr node
    /// Defines and add's a symbol for a specified attribute name
    /// If the attribute doesn't already exist, it is not added.
    /// The format of the name for the symbol attribute is {`attribute-name`}::{`symbol-name`}
    /// The value of the symbol will be {`symbol-name`}::
    #[token("define")]
    Define,
    /// Usage: commit {`attribute-name`} {`symbol-name`}
    /// Examples: commit test_attr node
    /// If a symbol has been defined for attribute, and the symbol attribute has a transient value,
    /// commit will override the value with the transient value.
    /// For example if some symbol called node is defined for test_attr. Then an attribute will exist with the name test_attr:node.
    /// If some system edits the value of test_attr::node, then a transient value will exist for test_attr::node.
    /// Dispatching commit will take that value and write to test_attr.
    #[token("commit")]
    Commit,
    /// Usage: edit {`attribute-name`} {`new-attribute-name`} {`new-value-type`} {`remaining as value`}
    /// Examples: edit test_attr test_attr2 .TEXT editing the value of test_attr
    /// Set's the transient value for an attribute. Types omitted from this event are symbol, reference, and binary-vector.
    #[token("edit")]
    Edit,
    // Logos requires one token variant to handle errors,
    // it can be named anything you wish.
    #[error]
    // We can also use this variant to define whitespace,
    // or any other matches we wish to skip.
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

/// Elements contained within an attribute graph
#[derive(Logos, Debug, Hash, Clone, PartialEq, PartialOrd)]
pub enum AttributeGraphElements {
    /// text element parses all remaining text after .TEXT as a string
    #[token(".TEXT", graph_lexer::from_text)]
    Text(Value),
    /// bool element parses remaining as bool
    #[token(".BOOL", graph_lexer::from_bool)]
    Bool(Value),
    /// int element parses remaining as i32
    #[token(".INT", graph_lexer::from_int)]
    Int(Value),
    /// int pair element parses remaining as 2 comma-delimmited i32's
    #[token(".INT_PAIR", graph_lexer::from_int_pair)]
    IntPair(Value),
    /// int range element parses remaining as 3 comma-delimitted i32's
    #[token(".INT_RANGE", graph_lexer::from_int_range)]
    IntRange(Value),
    /// float element parses remaining as f32
    #[token(".FLOAT", graph_lexer::from_float)]
    Float(Value),
    /// float pair element parses reamining as 2 comma delimitted f32's
    #[token(".FLOAT_PAIR", graph_lexer::from_float_pair)]
    FloatPair(Value),
    /// float range element parses remaining as 3 comma delimitted f32's
    #[token(".FLOAT_RANGE", graph_lexer::from_float_range)]
    FloatRange(Value),
    /// empty element parses
    #[token(".EMPTY")]
    Empty,
    /// entity ids should be parsed before symbols
    #[regex("[0-9]+", graph_lexer::from_entity)]
    Entity(u32),
    /// symbols must start with a character, and is composed of lowercase characters, digits, underscores, and colons
    #[regex("[a-z]+[a-z_:0-9]*", graph_lexer::from_symbol)]
    Symbol(String),
    // Logos requires one token variant to handle errors,
    // it can be named anything you wish.
    #[error]
    // We can also use this variant to define whitespace,
    // or any other matches we wish to skip.
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

mod graph_lexer {
    use atlier::system::Value;
    use logos::Lexer;

    use super::{AttributeGraphElements, AttributeGraphEvents};

    pub fn from_entity(lexer: &mut Lexer<AttributeGraphElements>) -> Option<u32> {
        lexer.slice().parse().ok()
    }

    pub fn from_symbol(lexer: &mut Lexer<AttributeGraphElements>) -> Option<String> {
        Some(lexer.slice().to_string())
    }

    pub fn from_text(lexer: &mut Lexer<AttributeGraphElements>) -> Option<Value> {
        let remaining = lexer.remainder().trim().to_string();

        Some(Value::TextBuffer(remaining))
    }

    pub fn from_bool(lexer: &mut Lexer<AttributeGraphElements>) -> Option<Value> {
        if let Some(value) = lexer.remainder().trim().parse().ok() {
            Some(Value::Bool(value))
        } else {
            None
        }
    }

    pub fn from_int(lexer: &mut Lexer<AttributeGraphElements>) -> Option<Value> {
        if let Some(value) = lexer.remainder().trim().parse::<i32>().ok() {
            Some(Value::Int(value))
        } else {
            None
        }
    }

    pub fn from_int_pair(lexer: &mut Lexer<AttributeGraphElements>) -> Option<Value> {
        let pair: Vec<i32> = lexer
            .remainder()
            .trim()
            .split(",")
            .filter_map(|i| i.trim().parse().ok())
            .collect();

        match (pair.get(0), pair.get(1)) {
            (Some(f0), Some(f1)) => Some(Value::IntPair(*f0, *f1)),
            _ => None,
        }
    }

    pub fn from_int_range(lexer: &mut Lexer<AttributeGraphElements>) -> Option<Value> {
        let range: Vec<i32> = lexer
            .remainder()
            .trim()
            .split(",")
            .filter_map(|i| i.trim().parse().ok())
            .collect();

        match (range.get(0), range.get(1), range.get(2)) {
            (Some(f0), Some(f1), Some(f2)) => Some(Value::IntRange(*f0, *f1, *f2)),
            _ => None,
        }
    }

    pub fn from_float(lexer: &mut Lexer<AttributeGraphElements>) -> Option<Value> {
        if let Some(value) = lexer.remainder().trim().parse::<f32>().ok() {
            Some(Value::Float(value))
        } else {
            None
        }
    }

    pub fn from_float_pair(lexer: &mut Lexer<AttributeGraphElements>) -> Option<Value> {
        let pair: Vec<f32> = lexer
            .remainder()
            .trim()
            .split(",")
            .filter_map(|i| i.trim().parse().ok())
            .collect();

        match (pair.get(0), pair.get(1)) {
            (Some(f0), Some(f1)) => Some(Value::FloatPair(*f0, *f1)),
            _ => None,
        }
    }

    pub fn from_float_range(lexer: &mut Lexer<AttributeGraphElements>) -> Option<Value> {
        let range: Vec<f32> = lexer
            .remainder()
            .trim()
            .split(",")
            .filter_map(|i| i.trim().parse().ok())
            .collect();

        match (range.get(0), range.get(1), range.get(2)) {
            (Some(f0), Some(f1), Some(f2)) => Some(Value::FloatRange(*f0, *f1, *f2)),
            _ => None,
        }
    }
}
