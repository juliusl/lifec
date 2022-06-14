use crate::AttributeGraph;
use crate::RuntimeDispatcher;
use atlier::system::Value;
use specs::storage::DenseVecStorage;
use specs::Component;
use std::collections::BTreeMap;

mod println;
pub use println::Println;

mod write_files;
pub use write_files::WriteFiles;

pub mod demo {
    use super::write_files::demo;
    pub use demo::WriteFilesDemo;
}

use super::Plugin;

/// ThunkContext provides common methods for updating the underlying state graph,
/// in the context of a thunk.
#[derive(Component, Default, Clone)]
#[storage(DenseVecStorage)]
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
        if let Some(_) = self
            .as_mut()
            .batch_mut(format!(
                r#"
                define {0} output
                edit {0}::output .EMPTY
                "#,
                output_name.as_ref()
            ))
            .ok()
        {
            self.as_mut()
                .find_update_attr(format!("{}::output", output_name.as_ref()), |a| a.edit_as(output));
        }
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
    pub fn set_return<T>(&mut self, name: &str, returns: Value)
    where
        T: Plugin<ThunkContext>,
    {
        if let Some(_) = self
            .as_mut()
            .batch_mut(format!(
                r#"
                define {0} returns
                edit {0}::returns {1} .EMPTY
                "#,
                T::symbol(),
                name
            ))
            .ok()
        {
            self.as_mut()
                .find_update_attr(format!("{}::returns", T::symbol()), |a| a.edit_as(returns));
        }
    }

    // Returns the transient return value for thunk type of T
    pub fn return_for<T>(&self) -> Option<&Value>
    where
        T: Plugin<ThunkContext>,
    {
        let symbol = format!("{}::returns", T::symbol());
        self.as_ref()
            .find_attr(symbol)
            .and_then(|a| if a.is_stable() { Some(a.value()) } else { None })
    }
}
