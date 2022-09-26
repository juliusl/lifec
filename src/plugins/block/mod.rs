use reality::{BlockProperties, BlockObject, BlockIndex};

mod address;
pub use address::BlockAddress;

mod project;
pub use project::Project;

use specs::storage::DenseVecStorage;
use specs::Component;
use std::collections::BTreeSet;

use crate::AttributeGraph;

/// BlockContext provides common methods for working with blocks
/// 
#[derive(Debug, Component, Default, Clone, Hash, Eq, PartialEq, PartialOrd)]
#[storage(DenseVecStorage)]
pub struct BlockContext {
    pub name: Option<String>,
    pub symbol: Option<String>, 
    pub index: Option<BlockIndex>,
    pub properties: Option<BlockProperties>,
    queries: BTreeSet<BlockProperties>,
}

impl BlockContext {
    /// Adds a query to the block context
    /// 
    pub fn add_query<T>(&mut self, object: &T) 
    where 
        T: BlockObject
    {
        self.queries.insert(object.query());
    }

    /// Resolves from a source, returns self if successful
    /// 
    pub fn resolve(&self, source: &BlockProperties) -> Option<Self> {
        let clone = self.clone();
        let mut resolved = vec![];
        for q in self.queries.iter() {
            if let Some(found) = q.query(source) {
                resolved.push(found);
            } else {
                return None;
            }
        }
        
        Some(clone)
    }

    pub fn to_blocks(&self) -> Vec<(String, AttributeGraph)> {
        todo!()
    }
}

