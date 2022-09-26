use reality::{BlockProperties, BlockObject, BlockIndex, Block};

mod address;
pub use address::BlockAddress;

mod project;
pub use project::Project;

use specs::{storage::DenseVecStorage, Entity};
use specs::Component;
use std::collections::{BTreeSet, BTreeMap};

use crate::AttributeGraph;

/// BlockContext provides common methods for working with blocks,
/// 
/// Each block corresponds w/ a single symbol -- After that all names w/ the context are available,
/// 
#[derive(Debug, Component, Default, Clone, Hash, Eq, PartialEq, PartialOrd)]
#[storage(DenseVecStorage)]
pub struct BlockContext {
    /// Root index
    root: Option<AttributeGraph>,
    /// Set of queries to run against indexes
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
    pub fn resolve(&self, source: &BlockIndex) -> Option<Self> {
        let clone = self.clone();
        // let mut resolved = vec![];
        // for q in self.queries.iter() {
        //     if let Some(found) = q.query(source) {
        //         resolved.push(found);
        //     } else {
        //         return None;
        //     }
        // }
        
        Some(clone)
    }
}

