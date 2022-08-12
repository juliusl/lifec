use std::{any::Any, sync::Arc};

use atlier::system::{Attribute, Value};
use specs::{Component, DefaultVecStorage};
use tracing::{event, Level};

use crate::{
    catalog::Item,
    plugins::{Thunk, ThunkContext},
    AttributeIndex,
};

use super::Operation;

/// A query is used to materialize types that implement `catalog::Item`
///
/// A query is built by specifying search parameters, and then evaluating those parameters against a
/// a source type that implements `AttributeIndex`. Types that implement the `AttributeIndex` trait can use .query()
/// to create a new `Query` struct.
///
/// When a parameter is evaluated, this query visits a destination type that implements `catalog::Item` and passes
/// the found value.
///
#[derive(Debug, Component, Default, Clone, Hash, PartialEq, Eq)]
#[storage(DefaultVecStorage)]
pub struct Query<I>
where
    I: AttributeIndex + Clone + Default + Sync + Send + Any,
{
    /// This is the root source index for this query
    ///
    /// **Note** Alternative sources can be used after this query is constructed
    ///
    pub src: Arc<I>,

    /// The entity_id is generally the id portion from a specs::Entity,
    ///
    pub entity_id: u32,

    /// A search param is a transient attribute, that is initialized w/ an empty value
    /// in the stable value position, and a the expected type in the transient value position.
    ///
    /// The initial value for transient values is the default value of the literal type.
    ///
    pub search_params: Vec<Attribute>,
}

impl<I> Query<I>
where
    I: AttributeIndex + Clone + Default + Sync + Send + Any,
{
    /// Returns a thunk that can be lazily evaluated w/ a src index, using
    /// the current search parameters, and provided item that implements Into<ThunkContext>
    ///
    /// Optionally, if a plugin thunk is provided, it will be evaluated w/ the thunk context, and
    /// if the thunk spawns a task, that task will be returned in the Operation object. The thunk will
    /// get a chance to modify the context as well before returning the task.
    /// 
    /// # Arguments
    /// 
    /// * `item` - This item will be visited when an attribute index is passed to the thunk
    /// After, the item will be converted into a thunk context.
    /// 
    /// * `plugin_thunk` - If set, this thunk will be called after evaluating an index, and after the 
    /// item has been visited and converted to a thunk context. 
    /// 
    /// **Important** The plugin itself has a chance to update the context further before returning a task.
    ///
    /// # Very Important Design Caveat if implementing Host/Plugin
    /// 
    /// A) If this is being used w/ the `Host` trait and a setup operation
    /// changes a stable attribute **AND**,
    /// B) the host event source was configured to setup the event from the project,
    /// **THEN** the host event source will not see those changes. 
    /// 
    /// This choice is made to preserve the definition of a stable attribute, and what that means to
    /// the runtime. In this case if the desire is to propagate these type of changes, then
    /// the plugin should design around a transient attribute.
    /// 
    /// We make a best effort to enforce this in this method by ensuring a block_name
    /// is set before the new context is returned. However, the alternate src could have a different
    /// block_name set, which means when the event is executed, the original block_name would have changed.
    /// That being said, logically that would also mean that the alternate src is representing a different 
    /// block, which would make that behavior valid. If this type of interaction is going to occur,
    /// it's important that the item has a block name set before .into() is called. If the block_name
    /// is already set, then we will skip this policy. 
    /// 
    /// 
    pub fn thunk<Alt>(
        &'_ self,
        item: impl Item + Into<ThunkContext> + Clone,
        plugin_thunk: Option<Thunk>,
    ) -> impl Fn(Arc<Alt>) -> Operation
    where
        Alt: AttributeIndex,
    {
        let c = self.clone();
        move |src| {
            let mut item = item.clone();
            c.evaluate_with(&src, &mut item);
            let mut context = item.into();

            // The block_name is used by the event runtime to configure an event 
            // If the follow up to the completion of the operation we're returning here
            // fires off an event w/ this context. It's important that we try to ensure
            // the block_name is set, so that a config can be found and executed
            // However, if the below plugin_thunk
            if context.block.block_name.is_empty() {
                if let Some(block_name) = src.find_text("block_name") {
                    event!(Level::TRACE, "context did not have a block name, setting block name found in src");
                    context.block.block_name = block_name;
                } else {
                    event!(Level::TRACE, "could not find a block name to use");
                }
            }


            let mut task = None;
            if let Some(Thunk(name, func)) = plugin_thunk.as_ref() {
                event!(Level::DEBUG, "Calling thunk {name}, from query thunk");
                task = (func)(&mut context);
            }

            Operation { context, task }
        }
    }

    /// Evaluates the current search parameters, by looping through each transient attribute
    /// and a value is found, visits the dest item
    ///
    pub fn evaluate(&self, dest: &mut impl Item) {
        self.evaluate_with(&self.src, dest)
    }

    /// Evaluates the current search parameters with a `src` that implements `AttributeIndex`
    ///
    pub fn evaluate_with<Alt>(&self, src: &Arc<Alt>, dest: &mut impl Item)
    where
        Alt: AttributeIndex,
    {
        for search in self.search_params.iter() {
            if let Some((name, _)) = search.transient() {
                // TODO: Enforce type validation ?  
                if let Some(value) = src.find_value(name) {
                    dest.visit(name, &value);
                }
            }
        }
    }

    /// Adds a search parameter for a text attribute w/ name
    ///
    pub fn find_text(mut self, with_name: impl AsRef<str>) -> Self {
        self.add_attribute(with_name, Value::TextBuffer("".to_string()));
        self
    }

    ///  Adds a search parameter for a symbol attribute w/ name
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

    ///  Adds a search parameter for a int attribute w/ name
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

    ///  Adds a search parameter for a float attribute w/ name
    ///
    pub fn find_float(mut self, with_name: impl AsRef<str>) -> Self {
        self.add_attribute(with_name, Value::IntPair(0, 0));
        self
    }

    ///  Adds a search parameter for a float pair attribute w/ name
    ///
    pub fn find_float_pair(mut self, with_name: impl AsRef<str>) -> Self {
        self.add_attribute(with_name, Value::IntPair(0, 0));
        self
    }

    ///  Adds a search parameter for a float range attribute w/ name
    ///
    pub fn find_float_range(mut self, with_name: impl AsRef<str>) -> Self {
        self.add_attribute(with_name, Value::IntPair(0, 0));
        self
    }

    ///  Adds a search parameter for a binary attribute w/ name
    ///
    pub fn find_binary(mut self, with_name: impl AsRef<str>) -> Self {
        self.add_attribute(with_name, Value::IntPair(0, 0));
        self
    }

    /// Adds a transient attribute search parameter
    ///
    fn add_attribute(&mut self, name: impl AsRef<str>, initial_transient: Value) {
        let mut attribute = Attribute::new(
            self.entity_id,
            name,
            // This value is used as the value cache
            // To access, cached() must be called explicitly
            Value::Empty,
        );
        attribute.edit_as(initial_transient);
        self.search_params.push(attribute);
    }
}

/// Cache methods  
///
impl<I> Query<I>
where
    I: AttributeIndex + Clone + Default + Sync + Send + Any,
{
    /// Visits the dest item w/ the current cached values stored in the search parameters
    ///
    pub fn cached(&self, dest: &mut impl Item) {
        for search in self.search_params.iter() {
            let cached = search.value();

            match cached {
                Value::Empty => event!(Level::WARN, "value is not cached, {}", search.name()),
                Value::Reference(_) => event!(
                    Level::WARN,
                    "cached reference is not implemented, {}",
                    search.name()
                ),
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
        Self: Clone,
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
}
