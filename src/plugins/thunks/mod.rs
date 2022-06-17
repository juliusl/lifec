use crate::AttributeGraph;
use atlier::system::{Value, Attribute};
use specs::storage::DenseVecStorage;
use specs::Component;
use std::collections::BTreeMap;

mod println;
mod form;
pub use form::Form;
pub use println::Println;

mod write_files;
pub use write_files::WriteFiles;

pub mod demo {
    use super::write_files::demo;
    pub use demo::WriteFilesDemo;
}

use super::BlockContext;
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
    // Write a transient output value for this context
    pub fn write_output(&mut self, output_name: impl AsRef<str>, output: Value) {
        let mut block_context = BlockContext::from(self.as_ref().clone());

        block_context.update_block("publish", |u| {
            u.with(output_name, output);
        });

        self.as_mut().merge(block_context.as_ref());
    }

    pub fn read_outputs(&self) -> Option<BTreeMap<String, Value>> {
        let mut outputs = BTreeMap::default();

        let block_context = BlockContext::from(self.as_ref().clone()); 
        if let Some(publish) = block_context.get_block("publish") {
            for attr in publish.clone().iter_attributes() {
                if let Some((publish_name, value)) = attr.transient() {
                    outputs.insert(publish_name.to_string(), value.clone());
                }
            }

            Some(outputs)
        } else {
            None
        }
    }

    pub fn accept(&mut self, accept: impl Fn(&Attribute) -> bool) {
        let mut block_context = BlockContext::from(self.as_ref().clone()); 

        if let Some(accept_block) = block_context.get_block("accept") {
            for (name, value) in accept_block.iter_attributes().filter(|a| accept(a)).map(|a| (a.name(), a.value())) {
                block_context.update_block("thunk", |u| {
                    u.with(name, value.clone());
                });
            }
        }
    }
}
