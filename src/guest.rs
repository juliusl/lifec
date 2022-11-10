use std::{hash::Hash, ops::Deref};

use reality::wire::{Protocol, WireObject};
use specs::{Component, Entity, HashMapStorage, RunNow, World, WorldExt};
use tokio::sync::watch::Ref;
use tracing::{event, Level};

use crate::{
    engine::{Performance, Runner},
    prelude::{
        Host, HostEditor, Journal, Node, NodeCommand, NodeStatus, PluginFeatures, Project, State,
        ThunkContext, Workspace,
    },
};

mod remote_protocol;
pub use remote_protocol::RemoteProtocol;

mod monitor;
pub use monitor::Monitor;

mod sender;
pub use sender::Sender;

/// Runs systems without a dispatcher,
///
pub type Run = fn(&Guest);

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
    /// Guest nodes,
    nodes: Vec<Node>,
}

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
            nodes: vec![],
        };

        guest
    }

    /// Adds a node to the guest,
    ///
    pub fn add_node(&mut self, node: Node) {
        self.nodes.push(node);
    }

    /// Returns an iterator over nodes,
    ///
    pub fn iter_nodes(&self) -> impl Iterator<Item = &Node> {
        self.nodes.iter()
    }

    /// Returns a mutable iterator over nodes,
    ///
    pub fn iter_nodes_mut(&mut self) -> impl Iterator<Item = &mut Node> {
        self.nodes.iter_mut()
    }

    /// Handle any commands from guest nodes and update protocol,
    /// 
    pub fn handle(&mut self) {
        let commands = self
            .nodes
            .iter_mut()
            .filter_map(|n| n.command.take())
            .collect::<Vec<_>>();
        self.protocol.send_if_modified(move |p| {
            let mut modified = false;
            for n in commands {
                match p
                    .as_ref()
                    .system_data::<State>()
                    .plugins()
                    .features()
                    .broker()
                    .try_send_node_command(n, None)
                {
                    Ok(_) => {
                        modified = true;
                    }
                    Err(err) => {
                        event!(Level::ERROR, "Error dispatching command, {err}");
                    }
                }
            }
            modified
        });
    }

    /// Enables the remote on this guest,
    ///
    /// When the remote is enabled, the host editor returned will have a remote protocol to read from,
    ///
    pub fn enable_remote(&mut self) {
        self.remote = Some(self.subscribe());

        self.protocol.send_modify(|p| {
            p.as_mut().insert(Some(self.subscribe()));
        })
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
            if !host_editor.has_remote() {
                host_editor.set_remote(remote.clone());
            }
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

    /// Encode a resource to the protocol,
    ///
    pub fn encode_resource<T>(&self, take_object: impl FnOnce(&Protocol) -> Option<T>) -> bool
    where
        T: WireObject + Clone + 'static,
    {
        self.protocol.send_if_modified(move |protocol| {
            if let Some(object) = { take_object(protocol) } {
                protocol.encoder::<T>(move |world, encoder| {
                    encoder.encode(&object, world);
                });

                true
            } else {
                false
            }
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

    /// Encode the command journal to protocol,
    ///
    pub fn encode_journal(&self) -> bool {
        self.encode_resource::<Journal>(|p| {
            let journal = p.as_ref().read_resource::<Journal>();

            Some(journal.deref().clone())
        })
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
        RemoteProtocol::new(self.protocol.subscribe())
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
