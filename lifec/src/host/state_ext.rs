use specs::{WorldExt, shred::Fetch};

use crate::{prelude::State, debugger::Debugger};

use super::Host;


/// Trait for accessing system data state
/// 
pub trait StateExt {
    /// Returns a reference to state system data, 
    /// 
    fn state(&self) -> State;

    /// Returns a debugger, if debugger is enabled,
    /// 
    fn debugger(&self) -> Fetch<Option<Debugger>>;
}

impl StateExt for Host {
    #[inline]
    fn state(&self) -> State {
        self.world().system_data::<State>()
    }

    #[inline]
    fn debugger(&self) -> Fetch<Option<Debugger>> {
        self.world().read_resource::<Option<Debugger>>()
    }
}