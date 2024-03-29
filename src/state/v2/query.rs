use std::{sync::Arc, any::Any};

use atlier::system::{Attribute, Value};
use specs::{Component, DefaultVecStorage};
use tracing::{event, Level};

use crate::{AttributeIndex, catalog::Item};

/// A query is used to materialize types that implement `catalog::Item`
/// 
/// A query is built by specifying search parameters, and then evaluating those parameters against a
/// a source type that implements `AttributeIndex`. Types that implement the `AttributeIndex` trait can use .query() 
/// to create a new `Query` struct.
/// 
/// When a parameter is evaluated, this query visits a destination type that implements `catalog::Item` and passes
/// the found value.
/// 
#[derive(Component, Default, Clone, Hash, PartialEq, Eq)]
#[storage(DefaultVecStorage)]
pub struct Query<I> 
where
    I: AttributeIndex + Clone + Default + Sync + Send + Any
{
    /// This is a reference to the source index, that will be used when evaluating this query
    /// 
    pub src: Arc<I>,
    
    /// The entity_id is generally the id portion from a specs::Entity,
    /// 
    /// *Note* A specs entity, also include generation, but that isn't important from this context
    pub entity_id: u32,

    /// A search param is a transient attribute, that is initialized w/ an empty value 
    /// in the stable value position, and a the expected type in the transient value position.
    /// 
    /// The initial value for transient values is the default value of the literal type. 
    /// 
    pub search_params: Vec<Attribute>
}

impl<I> Query<I>
where
    I: AttributeIndex + Clone + Default + Sync + Send + Any
{
    /// Evaluates the current search parameters, by looping through each transient attribute
    /// and a value is found, visits the dest item
    /// 
    pub fn evaluate(&self, dest: &mut impl Item) {
        self.evaluate_with(self.src.clone(), dest)
    }

    /// Evaluates the current search parameters with a `src` that implements `AttributeIndex`
    /// 
    pub fn evaluate_with(&self, src: Arc<I>, dest: &mut impl Item) {
        for search in self.search_params.iter() {
            if let Some((name, _)) = search.transient() {
                if let Some(value) = src.find_value(name) {
                    dest.visit(name, value);
                }
            }
        }
    }

    /// Visits the dest item w/ the current cached values stored in the search parameters
    /// 
    pub fn cached(&self, dest: &mut impl Item) {
        for search in self.search_params.iter() {
            let cached = search.value();

            match cached {
                Value::Empty => event!(Level::WARN, "value is not cached, {}", search.name()),
                Value::Reference(_) => event!(Level::WARN, "cached reference is not implemented, {}", search.name()),
                _ => {
                    dest.visit(search.name(), cached);
                }
            }
        }
    }

    /// Returns a new query, evaluating search parameters w/ the current src and caching the result.
    /// 
    /// Overwrites any previous cached values
    /// 
    /// A value is cached by committing that value to the search parameter attribute
    /// 
    pub fn cache(&self) -> Self 
    where
        Self: Clone
    {
        let mut cached = self.clone();
        let src = cached.src.clone();
        cached.cache_with(&src);
        cached
    }

    /// Evaluates search parameters w/ a src index, caching the result.
    /// 
    /// Overwrites any previous cached values
    /// 
    /// A value is cached by committing that value to the search parameter attribute
    /// 
    pub fn cache_with(&mut self, src: &Arc<I>) {
        for search in self.search_params.iter_mut() {
            if let Some((name, value)) = search.clone().transient() {
                if let Some(caching) = src.find_value(name) {
                    // Cache the value by committing it to the stable portion
                    search.edit_as(caching.clone());
                    search.commit();

                    // Reset the search param's attribute type
                    search.edit_as(value.clone());
                }
            }
        }
    }

    /// Adds a search for a text attribute w/ name
    ///
    pub fn find_text(mut self, with_name: impl AsRef<str>) -> Self {
        self.add_attribute(with_name, Value::TextBuffer("".to_string()));
        self
    }

    ///  Adds a search for a symbol attribute w/ name
    ///
    pub fn find_symbol(mut self, with_name: impl AsRef<str>) -> Self {
        self.add_attribute(with_name, Value::Symbol("".to_string()));
        self
    }

    ///  Adds a search for a bool attribute w/ name
    ///
    pub fn find_bool(mut self, with_name: impl AsRef<str>) -> Self {
        self.add_attribute(with_name, Value::Bool(false));
        self
    }

    ///  Adds a search for a int attribute w/ name
    ///
    pub fn find_int(mut self, with_name: impl AsRef<str>) -> Self {
        self.add_attribute(with_name, Value::Int(0));
        self
    }

    ///  Adds a search for a int pair attribute w/ name
    ///
    pub fn find_int_pair(mut self, with_name: impl AsRef<str>) -> Self {
        self.add_attribute(with_name, Value::IntPair(0, 0));
        self
    }

    ///  Adds a search for a int range attribute w/ name
    ///
    pub fn find_int_range(mut self, with_name: impl AsRef<str>) -> Self {
        self.add_attribute(with_name, Value::IntPair(0, 0));
        self
    }

    ///  Adds a search for a float attribute w/ name
    ///
    pub fn find_float(mut self, with_name: impl AsRef<str>) -> Self {
        self.add_attribute(with_name, Value::IntPair(0, 0));
        self
    }

    ///  Adds a search for a float pair attribute w/ name
    ///
    pub fn find_float_pair(mut self, with_name: impl AsRef<str>) -> Self {
        self.add_attribute(with_name, Value::IntPair(0, 0));
        self
    }

    ///  Adds a search for a float range attribute w/ name
    ///
    pub fn find_float_range(mut self, with_name: impl AsRef<str>) -> Self {
        self.add_attribute(with_name, Value::IntPair(0, 0));
        self
    }

    ///  Adds a search for a binary attribute w/ name
    /// 
    pub fn find_binary(mut self, with_name: impl AsRef<str>) -> Self {
        self.add_attribute(with_name, Value::IntPair(0, 0));
        self
    }

    /// Add's a transient attribute search parameter
    /// 
    fn add_attribute(&mut self, name: impl AsRef<str>, initial_transient: Value) {
        let mut attribute = Attribute::new(self.entity_id, name, Value::Empty);
        attribute.edit_as(initial_transient);
        self.search_params.push(attribute);
    }
}