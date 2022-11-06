use std::{hash::Hash, ops::Deref};

use reality::wire::{Protocol, WireObject};
use specs::{Component, Entity, HashMapStorage, RunNow, World, WorldExt};
use tokio::sync::watch::Ref;

use crate::{
    debugger::Debugger,
    engine::{Performance, Runner},
    prelude::{
        Host, HostEditor, NodeCommand, NodeStatus, PluginFeatures, Project, State, ThunkContext,
        Workspace,
    },
};

/// Type alias for a remotely updated protocol,
///
pub type RemoteProtocol = tokio::sync::watch::Receiver<Protocol>;

/// Guest host as a component,
///
#[derive(Component)]
#[storage(HashMapStorage)]
pub struct Guest {
    /// Owner of the guest host,
    pub owner: Entity,
    /// Run function,
    stateless: Run,
    /// Workspace,
    workspace: Workspace,
    /// Host w/ protocol enabled,
    protocol: tokio::sync::watch::Sender<Protocol>,
    /// Remote protocol,
    remote: Option<RemoteProtocol>,
}

/// Runs systems without a dispatcher,
///
pub type Run = fn(&Guest);

impl Guest {
    /// Returns a new guest component,
    ///
    pub fn new<P>(owner: Entity, host: Host, stateless: Run) -> Self
    where
        P: Project,
    {
        let workspace = host.workspace().clone();
        let world: World = host.into();
        let protocol = Protocol::from(world);

        let (protocol, _) = tokio::sync::watch::channel(protocol);

        let guest = Self {
            owner,
            workspace,
            protocol,
            stateless,
            remote: None,
        };

        guest
    }

    /// Exports debug info,
    /// 
    pub fn export_debug_info(&self) {
        if let Some(debugger) = self
            .protocol()
            .as_ref()
            .fetch::<Option<Debugger>>()
            .deref()
            .clone()
        {
            // Encode completions from guest debugger,
            for c in debugger.completions() {
                
            }
        }
    }

    /// Enables the remote on this guest,
    ///
    /// When the remote is enabled, the host editor returned will have a remote protocol to read from,
    ///
    pub fn enable_remote(&mut self) {
        self.remote = Some(self.subscribe());
    }

    /// Returns true if remote protocol is enabled,
    ///
    pub fn is_remote(&self) -> bool {
        self.remote.is_some()
    }

    /// Stateless run
    ///
    pub fn run(&self) {
        (self.stateless)(self)
    }

    /// Returns the workspace hosting this guest,
    ///
    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    /// Gets a guest thunk context,
    ///
    pub fn guest_context(&self, initial: Option<&ThunkContext>) -> ThunkContext {
        let state = self.protocol();

        let mut context = state
            .as_ref()
            .system_data::<State>()
            .initialize_context(self.owner, initial);

        if let Some(remote) = self.remote.as_ref() {
            context.enable_remote(remote.clone());
        }

        context
    }

    /// Returns a host editor for this guest,
    ///
    pub fn guest_editor(&self) -> HostEditor {
        let state = self.protocol();

        let features = state.as_ref().system_data::<PluginFeatures>();

        let mut host_editor = features.host_editor();

        if let Some(remote) = self.remote.as_ref() {
            host_editor.set_remote(remote.clone());
        }

        host_editor.run_now(self.protocol.borrow().as_ref());
        host_editor
    }

    /// Returns a reference to protocol,
    ///
    pub fn protocol(&self) -> Ref<Protocol> {
        self.protocol.borrow()
    }

    /// Encode wire objects to protocol and update the watch channel,
    ///
    /// Returns objects that were encoded
    ///
    pub fn encode<T>(&self, take_objects: impl FnOnce(&Protocol) -> Vec<(Entity, T)>) -> bool
    where
        T: WireObject + Clone + 'static,
    {
        self.protocol.send_if_modified(move |protocol| {
            let objects = { take_objects(protocol) };

            let encoding = !objects.is_empty();

            protocol.encoder::<T>(move |world, encoder| {
                for (_, object) in objects {
                    encoder.encode(&object, world);
                }
            });

            encoding
        })
    }

    /// Updates the protocol,
    ///
    /// Returns true if there was a change,
    ///
    pub fn update_protocol(&self, update: impl FnOnce(&mut Protocol) -> bool) -> bool {
        self.protocol.send_if_modified(|protocol| update(protocol))
    }

    /// Encodes commands to protocol,
    ///
    /// returns true if any commands were encoded,
    ///
    pub fn encode_commands(&self) -> bool {
        self.encode::<NodeCommand>(|p| p.as_ref().system_data::<Runner>().take_commands())
    }

    /// Encodes performance to protocol,
    ///
    /// returns true if performances were encoded
    ///
    pub fn encode_performance(&self) -> bool {
        self.encode::<Performance>(|p| p.as_ref().system_data::<Runner>().take_performance())
    }

    /// Encodes node status to protocol,
    ///  
    /// returns true if status was encoded
    ///
    pub fn encode_status(&self) -> bool {
        self.encode::<NodeStatus>(|p| p.as_ref().system_data::<State>().take_statuses())
    }

    /// Maintain the protocol world,
    ///
    pub fn maintain(&self) {
        self.update_protocol(|protocol| {
            protocol.as_mut().maintain();
            true
        });
    }

    /// Returns a remote protocol,
    ///
    pub fn subscribe(&self) -> RemoteProtocol {
        self.protocol.subscribe()
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
