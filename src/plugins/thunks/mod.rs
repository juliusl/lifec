use crate::AttributeGraph;
use atlier::system::Attribute;
use specs::storage::DenseVecStorage;
use specs::Component;

mod println;
pub use println::Println;

mod write_files;
pub use write_files::WriteFiles;

pub mod demo {
    use super::write_files::demo;
    pub use demo::WriteFilesDemo;
}

use super::{BlockContext, Plugin};

/// Thunk is a function that can be passed around for the system to call later
#[derive(Component, Clone)]
#[storage(DenseVecStorage)]
pub struct Thunk(&'static str, fn(&mut ThunkContext));

impl Thunk {
    pub fn from_plugin<P>() -> Self
    where
        P: Plugin<ThunkContext>,
    {
        Self(P::symbol(), P::call_with_context)
    }

    pub fn symbol(&self) -> impl AsRef<str> {
        self.0
    }

    pub fn call(&self, context: &mut ThunkContext) {
        (self.1)(context);

        let ThunkContext(block) = context; 

        block.update_block("thunk", |t| {
            t.add_text_attr("thunk_symbol", self.symbol().as_ref().to_string());
        });
    }
}

/// ThunkContext provides common methods for updating the underlying state graph,
/// in the context of a thunk.
#[derive(Component, Default, Clone)]
#[storage(DenseVecStorage)]
pub struct ThunkContext(pub BlockContext);

impl From<AttributeGraph> for ThunkContext {
    fn from(g: AttributeGraph) -> Self {
        Self(BlockContext::from(g))
    }
}

impl AsRef<AttributeGraph> for ThunkContext {
    fn as_ref(&self) -> &AttributeGraph {
        self.0.as_ref()
    }
}

impl AsMut<AttributeGraph> for ThunkContext {
    fn as_mut(&mut self) -> &mut AttributeGraph {
        self.0.as_mut()
    }
}

impl ThunkContext {
    pub fn symbol(&self) -> Option<String> {
        let ThunkContext(block) = self; 
        if let Some(thunk) = block.get_block("thunk") {
            thunk.find_text("thunk_symbol")
        } else {
            None
        }
    }

    /// Updates error block
    pub fn error(&mut self, record: impl FnOnce(&mut AttributeGraph)) {
        self.0.update_block("error", record);
    }

    /// Update publish block
    pub fn publish(&mut self, update: impl FnOnce(&mut AttributeGraph)) {
        self.0.update_block("publish", update);
    }

    /// Receives values from the accept block, and updates the destination block with the new values
    pub fn accept(&mut self, dest_block: impl AsRef<str>, accept: impl Fn(&Attribute) -> bool) {
        if let Some(accept_block) = self.0.get_block("accept") {
            for (name, value) in accept_block
                .iter_attributes()
                .filter(|a| accept(a))
                .map(|a| (a.name(), a.value()))
            {
                self.0.update_block(dest_block.as_ref(), |u| {
                    u.with(name, value.clone());
                });
            }
        }
    }
}
