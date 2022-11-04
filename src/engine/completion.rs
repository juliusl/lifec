use std::collections::BTreeMap;

use atlier::system::Value;
use reality::BlockProperties;
use specs::Entity;

/// Struct that contains the results of a single thunk completion,
/// 
#[derive(Clone, PartialEq)]
pub struct Completion {
    /// Event entity that initiated this completion,
    /// 
    /// If this is a completion from a spawned event, then its possible that the spawned event was cleaned up,
    /// 
    pub event: Entity,
    /// Thunk entity that owns this completion,
    /// 
    /// The thunk entity will have all of the relevant state components
    /// 
    pub thunk: Entity,
    /// Control value state, 
    /// 
    pub control_values: BTreeMap<String, Value>,
    /// Block object query that resulted in this completion,
    /// 
    /// When a plugin implements BlockObject, it can declare a query that represents the properties 
    /// it will look for during it's call. These are the properties that were used. 
    /// 
    pub query: BlockProperties,
    /// Block object return that was the result of this completion,
    /// 
    /// A BlockObject may optionally declare a set of block properties that might be committed to state.
    /// 
    pub returns: Option<BlockProperties>,
}