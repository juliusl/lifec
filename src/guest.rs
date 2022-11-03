use std::hash::Hash;

use specs::{Component, Entity, RunNow, VecStorage};

use crate::{prelude::{Host, HostEditor, PluginFeatures, Project, ThunkContext}, engine::State};

/// Guest host as a component,
///
#[derive(Component)]
#[storage(VecStorage)]
pub struct Guest {
    /// Owner of the guest host,
    pub owner: Entity,
    /// Host w/ protocol enabled,
    host: Host,
    /// Run function
    stateless: Run,
}

/// Runs systems without a dispatcher,
/// 
pub type Run =  fn(&mut Host);

impl Guest {
    /// Returns a new guest component,
    ///
    pub fn new<P>(owner: Entity, mut host: Host, stateless: Run) -> Self
    where
        P: Project,
    {
        host.enable_protocol();
        let guest = Self {
            owner,
            host,
            stateless,
        };
        guest
    }

    /// Gets a guest thunk context,
    ///
    pub fn guest_context(&mut self) -> ThunkContext {
        let features = self.host.world().system_data::<State>();

        features.initialize_context(self.owner, None)
    }

    /// Returns a host editor for this guest,
    ///
    pub fn guest_editor(&self) -> HostEditor {
        let features = self.host.world().system_data::<PluginFeatures>();

        let mut host_editor = features.host_editor();

        host_editor.run_now(self.host.world());

        host_editor
    }

    /// Returns a host,
    ///
    pub fn host(&self) -> &Host {
        &self.host
    }

    /// Returns a mutable host reference,
    ///
    pub fn host_mut(&mut self) -> &mut Host {
        &mut self.host
    }

    /// Stateless run
    /// 
    pub fn run(&mut self) {
        (self.stateless)(&mut self.host)
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

impl Eq for Guest {}
