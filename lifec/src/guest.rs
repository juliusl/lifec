use std::hash::Hash;

use reality::wire::{Protocol, WireObject};
use specs::{Component, Entity, HashMapStorage, World, WorldExt};
use tokio::sync::watch::Ref;

use crate::prelude::{Host, Project, State, ThunkContext, Workspace};

cfg_editor! {
    use crate::prelude::{HostEditor, Node};
    use tracing::{event, Level};
    use specs::RunNow;
}

mod remote_protocol;
pub use remote_protocol::RemoteProtocol;

/// Runs systems without a dispatcher,
///
pub type Run = fn(&Guest);

/// Guest host as a component,
/// 
/// This is useful in situations where a plugin wishes to maintain their own world/host, and mostly convienient for running
/// "stateless" functions from plugins via the remote_protocol feature.
/// 
/// Internally a "Protocol" is used to create a boundary between callers and internal world storage. This also enables the guest component to 
/// directly encode/decode wire objects from/to the World.
/// 
/// If the "editor" feature is enabled, the guest component also includes a feature to add custom "nodes" to storage. In the editor these nodes will be updated and allowed to create 
/// ui. Plugins are freely to modify guests they create/consume. The runtime does not enforce any other isolation beyond ensuring that the guest cannot mutate the main host. Guests will have
/// the same permissions as the current host on the environment.
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
    /// Guest nodes, requires the "editor" feature
    #[cfg(feature = "editor")]
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
            #[cfg(feature = "editor")]
            nodes: vec![],
        };

        guest
    }

    cfg_editor! {
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

    /// Returns a host editor for this guest,
    ///
    pub fn guest_editor(&self) -> HostEditor {
        let state = self.protocol();

        let features = state.as_ref().system_data::<crate::engine::PluginFeatures>();

        let mut host_editor = features.host_editor();

        if let Some(remote) = self.remote.as_ref() {
            if !host_editor.has_remote() {
                host_editor.set_remote(remote.clone());
            }
        }

        host_editor.run_now(self.protocol.borrow().as_ref());
        host_editor
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

    /// Returns a reference to protocol,
    ///
    pub fn protocol(&self) -> Ref<Protocol> {
        self.protocol.borrow()
    }

    /// Encode wire objects to protocol and update the watch channel,
    ///
    /// Returns true if objects were encoded
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
