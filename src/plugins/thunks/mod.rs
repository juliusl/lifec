use std::collections::BTreeMap;

use atlier::system::{Attribute, Value};

use crate::{AttributeGraph, RuntimeState};

/// This trait is to help organize different thunk contexts
pub trait Thunk {
    /// Returns the symbol name for this thunk, to reference call by name
    fn symbol() -> &'static str;

    /// Function that can be indexed with a call table
    fn call(values: &mut AttributeGraph) {
        let mut context = ThunkContext::from(values.clone());
        let context = &mut context;
        Self::call_with_context(context);

        *values = values.merge_with(context.as_ref());
    }

    fn call_with_context(context: &mut ThunkContext);
}

pub struct ThunkContext(AttributeGraph);

impl From<AttributeGraph> for ThunkContext {
    fn from(g: AttributeGraph) -> Self {
        Self(g)
    }
}

impl AsRef<AttributeGraph> for ThunkContext {
    fn as_ref(&self) -> &AttributeGraph {
        &self.0
    }
}

impl AsMut<AttributeGraph> for ThunkContext {
    fn as_mut(&mut self) -> &mut AttributeGraph {
        &mut self.0
    }
}

impl ThunkContext {
    pub fn outputs(&self) -> BTreeMap<String, Value> {
        let mut outputs = BTreeMap::default();
        self.as_ref()
            .find_symbol_values("output")
            .iter()
            .for_each(|(k, o)| {
                outputs.insert(k.to_string(), o.clone());
            });

        outputs
    }

    pub fn write_output(&mut self, output_name: impl AsRef<str>, output: Value) {
        let symbol = format!("{}::output", output_name.as_ref());
        self.as_mut()
            .with(&symbol, Value::Symbol("output::".to_string()));
        self.as_mut()
            .find_attr_mut(&symbol)
            .expect("just added")
            .edit((symbol, output));
    }

    pub fn set_return<T>(&mut self, returns: Value)
    where
        T: Thunk,
    {
        let symbol = format!("{}::returns", T::symbol());
        self.as_mut()
            .with(&symbol, Value::Symbol("returns::".to_string()));
        self.as_mut()
            .find_attr_mut(&symbol)
            .expect("just added")
            .edit((symbol, returns));
    }

    pub fn returns(&self) -> Vec<&(String, Value)> {
        self.as_ref()
            .find_symbols("returns")
            .iter()
            .filter_map(|a| a.transient())
            .collect()
    }

    pub fn returns_for<T>(&self) -> Option<&Value>
    where
        T: Thunk,
    {
        let symbol = format!("{}::returns", T::symbol());
        self.as_ref()
            .find_attr(symbol)
            .and_then(|a| if a.is_stable() { Some(a.value()) } else { None })
    }
}
