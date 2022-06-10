use std::collections::BTreeMap;
use atlier::system::Value;
use crate::{AttributeGraph};

mod println;
pub use println::Println;

mod write_files;
pub use write_files::WriteFiles;

use super::Plugin;

/// ThunkContext provides common methods for updating the underlying state graph
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
    // Gets the current outputs for this context
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

    // Write a transient output value for this context
    pub fn write_output(&mut self, output_name: impl AsRef<str>, output: Value) {
        let symbol = format!("{}::output", output_name.as_ref());
        self.as_mut()
            .with(&symbol, Value::Symbol("output::".to_string()));
        self.as_mut()
            .find_attr_mut(&symbol)
            .expect("just added")
            .edit((symbol, output));
    }

    // Returns all transient return values from this context
    pub fn returns(&self) -> Vec<&(String, Value)> {
        self.as_ref()
            .find_symbols("returns")
            .iter()
            .filter_map(|a| a.transient())
            .collect()
    }
    
    // Set a transient return value for this context
    pub fn set_return<T>(&mut self, returns: Value)
    where
        T:  Plugin<ThunkContext>,
    {
        let symbol = format!("{}::returns", T::symbol());
        self.as_mut()
            .with(&symbol, Value::Symbol("returns::".to_string()));
        self.as_mut()
            .find_attr_mut(&symbol)
            .expect("just added")
            .edit((symbol, returns));
    }

    // Returns the transient return value for thunk type of T
    pub fn return_for<T>(&self) -> Option<&Value>
    where
        T:  Plugin<ThunkContext>,
    {
        let symbol = format!("{}::returns", T::symbol());
        self.as_ref()
            .find_attr(symbol)
            .and_then(|a| if a.is_stable() { Some(a.value()) } else { None })
    }
}
