use specs::Component;
use specs::storage::DenseVecStorage;

use crate::AttributeGraph;

/// BlockContext provides common methods for working with blocks
#[derive(Component, Default, Clone)]
#[storage(DenseVecStorage)]
pub struct BlockContext(AttributeGraph);

impl BlockContext {
    /// returns the block name of the current context
    pub fn block_name(&self) -> Option<String> {
        self.as_ref().find_text("block_name")
    }

    /// returns the block symbol of the current context
    pub fn block_symbol(&self) -> Option<String> {
        self.as_ref().find_text("block_symbol")
    }
}

impl From<AttributeGraph> for BlockContext {
    fn from(g: AttributeGraph) -> Self {
        Self(g)
    }
}

impl AsRef<AttributeGraph> for BlockContext {
    fn as_ref(&self) -> &AttributeGraph {
        &self.0
    }
}

impl AsMut<AttributeGraph> for BlockContext {
    fn as_mut(&mut self) -> &mut AttributeGraph {
        &mut self.0
    }
}
