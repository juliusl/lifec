use std::sync::Arc;
use std::hash::Hash;

use specs::{Component, VecStorage, Entity};

use crate::prelude::Host;

/// Guest host as a component,
/// 
#[derive(Component, Clone)]
#[storage(VecStorage)]
pub struct Guest { 
    /// Owner of the guest host,
    pub owner: Entity, 
    /// Reference to the guest host,
    pub guest_host: Arc<Host>,
}

impl PartialEq for Guest {
    fn eq(&self, other: &Self) -> bool {
        self.owner == other.owner
    }
}

impl Hash for Guest {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.owner.hash(state);
    }
}

impl Eq for Guest {
    
}