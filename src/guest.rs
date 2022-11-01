use std::hash::Hash;

use specs::{Component, VecStorage, Entity};

use crate::prelude::{Host, Plugins, ThunkContext};

/// Guest host as a component,
/// 
#[derive(Component)]
#[storage(VecStorage)]
pub struct Guest { 
    /// Owner of the guest host,
    pub owner: Entity, 
    /// Host w/ protocol enabled,
    host: Host,
}

impl Guest {
    /// Returns a new guest component,
    /// 
    pub fn new(owner: Entity, host: Host) -> Self {
        let mut guest = Self { owner, host };
        guest.host.enable_protocol();
        guest
    }

    /// Gets a guest thunk context,
    /// 
    pub fn guest_context(&mut self) -> ThunkContext {
        let features = self.host.world().system_data::<Plugins>();

        features.initialize_context(self.owner, None)
    }
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