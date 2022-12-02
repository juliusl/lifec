use std::collections::{BTreeMap, BTreeSet};

use reality::BlockProperties;
use reality::{Attribute, Value};

/// V2 - Revising interface w/ attributes
///
/// Trait to support multiple attribute indexing implementations and stores
///
pub trait AttributeIndex {
    /// Returns the current entity in the context of the index
    ///
    fn entity_id(&self) -> u32;

    /// Returns the hash code of the current state
    ///
    fn hash_code(&self) -> u64;

    /// Finds a value from the index, from an attribute with_name
    ///
    /// This will always be the stable value from the attribute
    ///
    fn find_value(&self, with_name: impl AsRef<str>) -> Option<Value>;

    /// Returns all values found with_name
    ///
    fn find_values(&self, with_name: impl AsRef<str>) -> Vec<Value>;

    /// Adds an attribute to the index
    ///
    fn add_attribute(&mut self, attr: Attribute);

    /// Replaces an attribute to the index
    ///
    fn replace_attribute(&mut self, attr: Attribute);

    /// Returns a map of indexed values,
    ///
    fn values(&self) -> BTreeMap<String, Vec<Value>>;

    /// Returns a reference to state as a block properties struct,
    ///
    fn properties(&self) -> &BlockProperties;

    /// Returns a mutable reference to state as a block properties struct,
    ///
    fn properties_mut(&mut self) -> &mut BlockProperties;

    /// Returns a map of control values,
    ///
    /// Control values are defined at the block level and are used when the index for root of the index does not have a value,
    ///
    fn control_values(&self) -> &BTreeMap<String, Value>;

    /// Finds all text values with name,
    ///
    fn find_text_values(&self, with_name: impl AsRef<str>) -> Vec<String> {
        self.find_values(with_name)
            .iter()
            .filter_map(|v| match v {
                Value::TextBuffer(text) => Some(text.to_string()),
                _ => None,
            })
            .collect()
    }

    /// Finds all symbol values with name,
    ///
    fn find_symbol_values(&self, with_name: impl AsRef<str>) -> Vec<String> {
        self.find_values(with_name)
            .iter()
            .filter_map(|v| match v {
                Value::Symbol(symbol) => Some(symbol.to_string()),
                _ => None,
            })
            .collect()
    }

    /// Finds all symbol values with name,
    ///
    fn find_binary_values(&self, with_name: impl AsRef<str>) -> Vec<Vec<u8>> {
        self.find_values(with_name)
            .iter()
            .filter_map(|v| match v {
                Value::BinaryVector(bin) => Some(bin.to_vec()),
                _ => None,
            })
            .collect()
    }

    /// Finds a list of float ranges,
    ///
    fn find_float_range_values(&self, with_name: impl AsRef<str>) -> Vec<[f32; 3]> {
        self.find_values(with_name)
            .iter()
            .filter_map(|v| match v {
                Value::FloatRange(a, b, c) => Some([*a, *b, *c]),
                _ => None,
            })
            .collect()
    }

    /// Finds a list of float pairs,
    ///
    fn find_float_pair_values(&self, with_name: impl AsRef<str>) -> Vec<[f32; 2]> {
        self.find_values(with_name)
            .iter()
            .filter_map(|v| match v {
                Value::FloatPair(a, b) => Some([*a, *b]),
                _ => None,
            })
            .collect()
    }

    /// Finds a text value from an attribute
    ///
    fn find_text(&self, with_name: impl AsRef<str>) -> Option<String> {
        if let Some(Value::TextBuffer(a)) = self.find_value(with_name) {
            Some(a.to_string())
        } else {
            None
        }
    }

    /// Finds a symbol value from an attribute
    ///
    fn find_symbol(&self, with_name: impl AsRef<str>) -> Option<String> {
        if let Some(Value::Symbol(a)) = self.find_value(with_name) {
            Some(a.to_string())
        } else {
            None
        }
    }

    /// Finds a bool value from an attribute
    ///
    fn find_bool(&self, with_name: impl AsRef<str>) -> Option<bool> {
        if let Some(Value::Bool(a)) = self.find_value(with_name) {
            Some(a)
        } else {
            None
        }
    }

    /// Finds an int value from an attribute
    ///
    fn find_int(&self, with_name: impl AsRef<str>) -> Option<i32> {
        if let Some(Value::Int(a)) = self.find_value(with_name) {
            Some(a)
        } else {
            None
        }
    }

    /// Find an int pair value from an attribute
    ///
    fn find_int_pair(&self, with_name: impl AsRef<str>) -> Option<(i32, i32)> {
        if let Some(Value::IntPair(a, b)) = self.find_value(with_name) {
            Some((a, b))
        } else {
            None
        }
    }

    /// Find an int pair value from an attribute
    ///
    fn find_int_pair_values(&self, with_name: impl AsRef<str>) -> Vec<(i32, i32)> {
        self.find_values(with_name)
            .iter()
            .filter_map(Value::int_pair)
            .collect()
    }

    /// Finds an int range value from an attribute
    ///
    fn find_int_range(&self, with_name: impl AsRef<str>) -> Option<(i32, i32, i32)> {
        if let Some(Value::IntRange(a, b, c)) = self.find_value(with_name) {
            Some((a, b, c))
        } else {
            None
        }
    }

    /// Finds a float value of from an attribute
    ///
    fn find_float(&self, with_name: impl AsRef<str>) -> Option<f32> {
        if let Some(Value::Float(a)) = self.find_value(with_name) {
            Some(a)
        } else {
            None
        }
    }

    /// Finds a float pair value from an attribute
    ///
    fn find_float_pair(&self, with_name: impl AsRef<str>) -> Option<(f32, f32)> {
        if let Some(Value::FloatPair(a, b)) = self.find_value(with_name) {
            Some((a, b))
        } else {
            None
        }
    }

    /// Finds a float range value from an attribute
    ///
    fn find_float_range(&self, with_name: impl AsRef<str>) -> Option<(f32, f32, f32)> {
        if let Some(Value::FloatRange(a, b, c)) = self.find_value(with_name) {
            Some((a, b, c))
        } else {
            None
        }
    }

    /// Finds an attribute with binary value by name
    ///
    fn find_binary(&self, with_name: impl AsRef<str>) -> Option<Vec<u8>> {
        if let Some(Value::BinaryVector(content)) = self.find_value(with_name) {
            Some(content.to_vec())
        } else {
            None
        }
    }

    /// Returns self with an empty attribute w/ name.
    ///
    fn with_empty(&mut self, name: impl AsRef<str>) -> &mut Self {
        self.with(name, Value::Empty)
    }

    /// Returns self with a symbol attribute w/ name.
    ///
    fn with_symbol(&mut self, name: impl AsRef<str>, symbol: impl AsRef<str>) -> &mut Self {
        self.with(name, Value::Symbol(symbol.as_ref().to_string()))
    }

    /// Returns self with a binary attribute w/ name.
    ///
    fn with_binary(&mut self, name: impl AsRef<str>, binary: impl Into<Vec<u8>>) -> &mut Self {
        self.with(name, Value::BinaryVector(binary.into()))
    }

    /// Returns self with a text buffer attribute w/ name.
    ///
    fn with_text(&mut self, name: impl AsRef<str>, init_value: impl AsRef<str>) -> &mut Self {
        self.with(name, Value::TextBuffer(init_value.as_ref().to_string()))
    }

    /// Returns self with an int attribute w/ name.
    ///
    fn with_int(&mut self, name: impl AsRef<str>, init_value: i32) -> &mut Self {
        self.with(name, Value::Int(init_value))
    }

    /// Returns self with a float attribute w/ name.
    ///
    fn with_float(&mut self, name: impl AsRef<str>, init_value: f32) -> &mut Self {
        self.with(name, Value::Float(init_value))
    }

    /// Returns self with a bool attribute w/ name.
    ///
    fn with_bool(&mut self, name: impl AsRef<str>, init_value: bool) -> &mut Self {
        self.with(name, Value::Bool(init_value))
    }

    /// Returns self with a float pair attribute w/ name.
    ///
    fn with_float_pair(&mut self, name: impl AsRef<str>, init_value: &[f32; 2]) -> &mut Self {
        self.with(name, Value::FloatPair(init_value[0], init_value[1]))
    }

    /// Returns self with an int pair attribute w/ name.
    ///
    fn with_int_pair(&mut self, name: impl AsRef<str>, init_value: &[i32; 2]) -> &mut Self {
        self.with(name, Value::IntPair(init_value[0], init_value[1]))
    }

    /// Returns self with an int range attribute w/ name.
    ///
    fn with_int_range(&mut self, name: impl AsRef<str>, init_value: &[i32; 3]) -> &mut Self {
        self.with(
            name,
            Value::IntRange(init_value[0], init_value[1], init_value[2]),
        )
    }

    /// Returns self with a float range attribute w/ name.
    ///
    fn with_float_range(&mut self, name: impl AsRef<str>, init_value: &[f32; 3]) -> &mut Self {
        self.with(
            name,
            Value::FloatRange(init_value[0], init_value[1], init_value[2]),
        )
    }

    /// Adds a reference attribute w/ init_value and w/ name to index for entity.
    ///
    fn add_reference(&mut self, name: impl AsRef<str>, init_value: Value) {
        self.add_attribute(Attribute::new(
            self.entity_id(),
            name.as_ref().to_string(),
            init_value.to_ref(),
        ));
    }

    /// Adds a symbol attribute w/ symbol and w/ name to index for entity.
    ///
    fn add_symbol(&mut self, name: impl AsRef<str>, symbol: impl AsRef<str>) {
        self.add_attribute(Attribute::new(
            self.entity_id(),
            name.as_ref().to_string(),
            Value::Symbol(symbol.as_ref().to_string()),
        ));
    }

    /// Adds an empty attribute w/ name to index for entity.
    ///
    fn add_empty_attr(&mut self, name: impl AsRef<str>) {
        self.add_attribute(Attribute::new(
            self.entity_id(),
            name.as_ref().to_string(),
            Value::Empty,
        ));
    }

    /// Adds a binary vector attribute w/ name and w/ init_value for entity.
    ///
    fn add_binary_attr(&mut self, name: impl AsRef<str>, init_value: impl Into<Vec<u8>>) {
        self.add_attribute(Attribute::new(
            self.entity_id(),
            name.as_ref().to_string(),
            Value::BinaryVector(init_value.into()),
        ));
    }

    /// Adds a text buffer attribute w/ name and w/ init_value for entity.
    ///
    fn add_text_attr(&mut self, name: impl AsRef<str>, init_value: impl AsRef<str>) {
        self.add_attribute(Attribute::new(
            self.entity_id(),
            name.as_ref().to_string(),
            Value::TextBuffer(init_value.as_ref().to_string()),
        ));
    }

    /// Adds an int attribute w/ name and w/ init_value for entity.
    ///
    fn add_int_attr(&mut self, name: impl AsRef<str>, init_value: i32) {
        self.add_attribute(Attribute::new(
            self.entity_id(),
            name.as_ref().to_string(),
            Value::Int(init_value),
        ));
    }

    /// Adds an float attribute w/ name and w/ init_value for entity.
    ///
    fn add_float_attr(&mut self, name: impl AsRef<str>, init_value: f32) {
        self.add_attribute(Attribute::new(
            self.entity_id(),
            name.as_ref().to_string(),
            Value::Float(init_value),
        ));
    }

    /// Adds a bool attribute w/ name and w/ init_value for entity.
    ///
    fn add_bool_attr(&mut self, name: impl AsRef<str>, init_value: bool) {
        self.add_attribute(Attribute::new(
            self.entity_id(),
            name.as_ref().to_string(),
            Value::Bool(init_value),
        ));
    }

    /// Adds a float pair attribute w/ name and w/ init_value for entity.
    ///
    fn add_float_pair_attr(&mut self, name: impl AsRef<str>, init_value: &[f32; 2]) {
        self.add_attribute(Attribute::new(
            self.entity_id(),
            name.as_ref().to_string(),
            Value::FloatPair(init_value[0], init_value[1]),
        ));
    }

    /// Adds an int pair attribute w/ name and w/ init_value for entity.
    ///
    fn add_int_pair_attr(&mut self, name: impl AsRef<str>, init_value: &[i32; 2]) {
        self.add_attribute(Attribute::new(
            self.entity_id(),
            name.as_ref().to_string(),
            Value::IntPair(init_value[0], init_value[1]),
        ));
    }

    /// Adds an int range attribute w/ name and w/ init_value for entity.
    ///
    fn add_int_range_attr(&mut self, name: impl AsRef<str>, init_value: &[i32; 3]) {
        self.add_attribute(Attribute::new(
            self.entity_id(),
            name.as_ref().to_string(),
            Value::IntRange(init_value[0], init_value[1], init_value[2]),
        ));
    }

    /// Adds an float range attribute w/ name and w/ init_value for entity.
    ///
    fn add_float_range_attr(&mut self, name: impl AsRef<str>, init_value: &[f32; 3]) {
        self.add_attribute(Attribute::new(
            self.entity_id(),
            name.as_ref().to_string(),
            Value::FloatRange(init_value[0], init_value[1], init_value[2]),
        ));
    }

    /// Adds a complex attribute
    ///
    /// A complex attribute is a set of identifiers. In the context of a reality block,
    /// the identifiers are property map keys for a stable attribute that has a property map.
    ///
    fn add_complex(&mut self, name: impl AsRef<str>, init_value: impl Into<BTreeSet<String>>) {
        self.add_attribute(Attribute::new(
            self.entity_id(),
            name.as_ref().to_string(),
            Value::Complex(init_value.into()),
        ));
    }

    /// Replaces an float range attribute w/ name and w/ init_value for entity.
    ///
    fn replace_float_range_attr(
        &mut self,
        name: impl AsRef<str>,
        init_value: &[f32; 3],
    ) -> &mut Self {
        self.replace_attribute(Attribute::new(
            self.entity_id(),
            name.as_ref().to_string(),
            Value::FloatRange(init_value[0], init_value[1], init_value[2]),
        ));
        self
    }

    /// Replaces a complex attribute
    ///
    /// A complex attribute is a set of identifiers. In the context of a reality block,
    /// the identifiers are property map keys for a stable attribute that has a property map.
    ///
    fn replace_complex(
        &mut self,
        name: impl AsRef<str>,
        init_value: impl Into<BTreeSet<String>>,
    ) -> &mut Self {
        self.replace_attribute(Attribute::new(
            self.entity_id(),
            name.as_ref().to_string(),
            Value::Complex(init_value.into()),
        ));
        self
    }

    /// Replaces a reference attribute w/ init_value and w/ name to index for entity.
    ///
    fn replace_reference(&mut self, name: impl AsRef<str>, init_value: Value) -> &mut Self {
        self.replace_attribute(Attribute::new(
            self.entity_id(),
            name.as_ref().to_string(),
            init_value.to_ref(),
        ));
        self
    }

    /// Replaces a symbol attribute w/ symbol and w/ name to index for entity.
    ///
    fn replace_symbol(&mut self, name: impl AsRef<str>, symbol: impl AsRef<str>) -> &mut Self {
        self.replace_attribute(Attribute::new(
            self.entity_id(),
            name.as_ref().to_string(),
            Value::Symbol(symbol.as_ref().to_string()),
        ));
        self
    }

    /// Replaces an empty attribute w/ name to index for entity.
    ///
    fn replace_empty_attr(&mut self, name: impl AsRef<str>) -> &mut Self {
        self.replace_attribute(Attribute::new(
            self.entity_id(),
            name.as_ref().to_string(),
            Value::Empty,
        ));
        self
    }

    /// Replaces a binary vector attribute w/ name and w/ init_value for entity.
    ///
    fn replace_binary_attr(
        &mut self,
        name: impl AsRef<str>,
        init_value: impl Into<Vec<u8>>,
    ) -> &mut Self {
        self.replace_attribute(Attribute::new(
            self.entity_id(),
            name.as_ref().to_string(),
            Value::BinaryVector(init_value.into()),
        ));
        self
    }

    /// Replaces a text buffer attribute w/ name and w/ init_value for entity.
    ///
    fn replace_text_attr(
        &mut self,
        name: impl AsRef<str>,
        init_value: impl AsRef<str>,
    ) -> &mut Self {
        self.replace_attribute(Attribute::new(
            self.entity_id(),
            name.as_ref().to_string(),
            Value::TextBuffer(init_value.as_ref().to_string()),
        ));
        self
    }

    /// Replaces an int attribute w/ name and w/ init_value for entity.
    ///
    fn replace_int_attr(&mut self, name: impl AsRef<str>, init_value: i32) -> &mut Self {
        self.replace_attribute(Attribute::new(
            self.entity_id(),
            name.as_ref().to_string(),
            Value::Int(init_value),
        ));
        self
    }

    /// Replaces an float attribute w/ name and w/ init_value for entity.
    ///
    fn replace_float_attr(&mut self, name: impl AsRef<str>, init_value: f32) -> &mut Self {
        self.add_attribute(Attribute::new(
            self.entity_id(),
            name.as_ref().to_string(),
            Value::Float(init_value),
        ));
        self
    }

    /// Replaces a bool attribute w/ name and w/ init_value for entity.
    ///
    fn replace_bool_attr(&mut self, name: impl AsRef<str>, init_value: bool) -> &mut Self {
        self.add_attribute(Attribute::new(
            self.entity_id(),
            name.as_ref().to_string(),
            Value::Bool(init_value),
        ));
        self
    }

    /// Replaces a float pair attribute w/ name and w/ init_value for entity.
    ///
    fn replace_float_pair_attr(
        &mut self,
        name: impl AsRef<str>,
        init_value: &[f32; 2],
    ) -> &mut Self {
        self.replace_attribute(Attribute::new(
            self.entity_id(),
            name.as_ref().to_string(),
            Value::FloatPair(init_value[0], init_value[1]),
        ));
        self
    }

    /// Replaces an int pair attribute w/ name and w/ init_value for entity.
    ///
    fn replace_int_pair_attr(&mut self, name: impl AsRef<str>, init_value: &[i32; 2]) -> &mut Self {
        self.replace_attribute(Attribute::new(
            self.entity_id(),
            name.as_ref().to_string(),
            Value::IntPair(init_value[0], init_value[1]),
        ));
        self
    }

    /// Replaces an int range attribute w/ name and w/ init_value for entity.
    ///
    fn replace_int_range_attr(
        &mut self,
        name: impl AsRef<str>,
        init_value: &[i32; 3],
    ) -> &mut Self {
        self.replace_attribute(Attribute::new(
            self.entity_id(),
            name.as_ref().to_string(),
            Value::IntRange(init_value[0], init_value[1], init_value[2]),
        ));
        self
    }

    /// Updates the index
    ///
    fn update(&mut self, func: impl FnOnce(&mut Self)) -> &mut Self {
        (func)(self);
        self
    }

    /// Add's a value and returns self to make these api's chainable
    ///
    fn with(&mut self, name: impl AsRef<str>, value: Value) -> &mut Self {
        self.update(move |g| match value {
            Value::Empty => g.add_empty_attr(name),
            Value::Symbol(symbol) => g.add_symbol(name, symbol),
            Value::TextBuffer(text_buffer) => g.add_text_attr(name, text_buffer),
            Value::Float(init_value) => g.add_float_attr(name, init_value),
            Value::Int(init_value) => g.add_int_attr(name, init_value),
            Value::Bool(init_value) => g.add_bool_attr(name, init_value),
            Value::IntPair(e0, e1) => g.add_int_pair_attr(name, &[e0, e1]),
            Value::FloatPair(e0, e1) => g.add_float_pair_attr(name, &[e0, e1]),
            Value::FloatRange(value, min, max) => g.add_float_range_attr(name, &[value, min, max]),
            Value::IntRange(value, min, max) => g.add_int_range_attr(name, &[value, min, max]),
            Value::BinaryVector(init_value) => g.add_binary_attr(name, init_value),
            Value::Reference(_) => g.add_reference(name, value),
            Value::Complex(init_value) => g.add_complex(name, init_value),
        })
    }

    /// Creates a clone, and replaces a named value, returning self
    ///
    fn replace(&mut self, name: impl AsRef<str>, value: Value) -> &mut Self
    where
        Self: Clone,
    {
        match value {
            Value::Empty => self.replace_empty_attr(name),
            Value::Symbol(symbol) => self.replace_symbol(name, symbol),
            Value::TextBuffer(text_buffer) => self.replace_text_attr(name, text_buffer),
            Value::Float(init_value) => self.replace_float_attr(name, init_value),
            Value::Int(init_value) => self.replace_int_attr(name, init_value),
            Value::Bool(init_value) => self.replace_bool_attr(name, init_value),
            Value::IntPair(e0, e1) => self.replace_int_pair_attr(name, &[e0, e1]),
            Value::FloatPair(e0, e1) => self.replace_float_pair_attr(name, &[e0, e1]),
            Value::FloatRange(value, min, max) => {
                self.replace_float_range_attr(name, &[value, min, max])
            }
            Value::IntRange(value, min, max) => {
                self.replace_int_range_attr(name, &[value, min, max])
            }
            Value::BinaryVector(init_value) => self.replace_binary_attr(name, init_value),
            Value::Reference(_) => self.replace_reference(name, value),
            Value::Complex(init_value) => self.replace_complex(name, init_value),
        }
    }
}
