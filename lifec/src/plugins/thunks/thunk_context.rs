use crate::appendix::Appendix;
use crate::engine::{Completion, NodeCommand, Yielding};
use crate::error::Error;
use crate::guest::{Guest, RemoteProtocol};
use crate::prelude::{attributes::Fmt, *};
use hyper::{Body, Response};
use reality::Block;
use specs::Entity;
use std::fmt::Debug;
use std::{future::Future, net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::{TcpListener, UdpSocket},
    runtime::Handle,
    select,
    sync::{
        mpsc::Sender,
        oneshot::{self, channel},
    },
    task::JoinHandle,
};
use tracing::{error, debug, warn, info};

pub mod thunk_context_ext;

use super::{CancelSource, CancelToken, ErrorContext, SecureClient, StatusUpdate};

/// Thunk context is the major component for thunks,
///
/// Contains utilities for working with async code, such as network/io
/// while storing and updating data via an underlying block context.
///
/// Optionally, if a plugin sets the project field before it returns, and another plugin is scheduled to run
/// after, the event runtime will pass the project as a binary attribute called 'previous' to the next plugin.
/// Binary data passed in this manner can then be unpacked to the current graph using .apply("previous").
///
/// All fields of this context are intentionally safe to clone, share, and stored. Once an event has used a context,
/// that context will keep it's async deps for subsequent calls. This ensures that as long as a plugin only makes changes
/// to the context once, on subsequent calls of the plugin, the context will remain the same.
///
#[derive(Default)]
pub struct ThunkContext {
    /// # State Properties

    /// Compiled block that sourced the thunk,
    block: Block,
    /// Error graph recorded if the previous plugin call encountered an error
    error_graph: Option<AttributeGraph>,
    /// Previous graph used w/ this context, set by calling .commit()
    previous_graph: Option<AttributeGraph>,
    /// Current state of this context,
    graph: AttributeGraph,
    /// Error struct,
    error: Option<Error>,

    /// Entity that owns this context
    entity: Option<Entity>,
    /// Tokio runtime handle, to spawn additional tasks
    handle: Option<Handle>,
    /// Client for sending secure http requests
    client: Option<SecureClient>,
    /// Workspace for this context, if enabled the work_dir from the workspace can be used
    workspace: Option<Workspace>,
    /// Reference to an appendix,
    appendix: Option<Arc<Appendix>>,
    /// Local tcp socket address,
    local_tcp_addr: Option<SocketAddr>,
    /// Local udp socket address,
    local_udp_addr: Option<SocketAddr>,

    /// # I/O Utilities,
    /// Project-type implements handling the listener side,     

    /// Sender for status updates for the thunk
    status_updates: Option<Sender<StatusUpdate>>,
    /// Dispatcher for operations
    operation_dispatcher: Option<Sender<Operation>>,
    /// Dispatcher for sending a guest,
    guest_dispatcher: Option<Sender<Guest>>,
    /// Dispatcher for sending a node command,
    node_dispatcher: Option<Sender<(NodeCommand, Option<Yielding>)>>,
    /// Dispatcher for sending a node command,
    completion_dispatcher: Option<Sender<Completion>>,
    /// Watches for changes to the world's HostEditor,
    #[cfg(feature = "editor")]
    host_editor_watcher: Option<tokio::sync::watch::Receiver<HostEditor>>,
    /// Remote protocol
    remote: Option<RemoteProtocol>,

    /// UDP socket,
    ///
    /// Notes: Since UDP is connectionless, it can be shared, cloned, and stored in the
    /// context,
    ///
    /// In comparison, `enable_listener()` would start,
    ///     1) wait for a connection to be made,
    ///     2) wait for the connection to close,
    ///     3) and cannot be stored in the context,
    ///
    udp_socket: Option<Arc<UdpSocket>>,
    /// Caches a single http response, Cannot be cloned,
    ///
    response_cache: Option<Response<Body>>,
    /// Caches a single http body, Cannot be cloned,
    ///
    body_cache: Option<Body>,
}

impl Clone for ThunkContext {
    fn clone(&self) -> Self {
        Self {
            block: self.block.clone(),
            error_graph: self.error_graph.clone(),
            previous_graph: self.previous_graph.clone(),
            graph: self.graph.clone(),
            error: self.error.clone(),
            entity: self.entity.clone(),
            handle: self.handle.clone(),
            client: self.client.clone(),
            workspace: self.workspace.clone(),
            appendix: self.appendix.clone(),
            local_tcp_addr: self.local_tcp_addr.clone(),
            local_udp_addr: self.local_udp_addr.clone(),
            status_updates: self.status_updates.clone(),
            operation_dispatcher: self.operation_dispatcher.clone(),
            udp_socket: self.udp_socket.clone(),
            #[cfg(feature = "editor")]
            host_editor_watcher: self.host_editor_watcher.clone(),
            guest_dispatcher: self.guest_dispatcher.clone(),
            node_dispatcher: self.node_dispatcher.clone(),
            completion_dispatcher: self.completion_dispatcher.clone(),
            remote: self.remote.clone(),
            response_cache: None,
            body_cache: None,
        }
    }
}

impl Debug for ThunkContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThunkContext")
            .field("block", &self.block)
            .field("error_graph", &self.error_graph)
            .field("previous_graph", &self.previous_graph)
            .field("graph", &self.graph)
            .field("error", &self.error)
            .field("entity", &self.entity)
            .field("handle", &self.handle)
            .field("client", &self.client)
            .field("workspace", &self.workspace)
            .field("local_tcp_addr", &self.local_tcp_addr)
            .field("local_udp_addr", &self.local_udp_addr)
            .field("status_updates", &self.status_updates)
            .field("operation_dispatcher", &self.operation_dispatcher)
            .field("guest_dispatcher", &self.guest_dispatcher)
            .field("udp_socket", &self.udp_socket)
            .field("response_cache", &self.response_cache)
            .field("body_cache", &self.body_cache)
            .finish()
    }
}

/// This block has all the async related features
impl ThunkContext {
    /// Modify graph state,
    ///
    pub fn modify_graph(&mut self) -> &mut AttributeGraph {
        &mut self.graph
    }

    /// Caches a http body in the context,
    ///
    pub fn cache_body(&mut self, body: Body) {
        self.body_cache = Some(body);
    }

    /// Caches a response in this context,
    ///
    /// The motivation behind this is that http responses are common enough that most applications will use them, however reading the body
    /// from the response and saving it directly to graph as binary can get very expensive. Since the response has the body as a stream, this is cheaper.
    ///
    pub fn cache_response(&mut self, resp: Response<Body>) {
        self.response_cache = Some(resp);
    }

    /// Returns a reference to the cached response,
    ///
    pub fn cached_response(&self) -> Option<&Response<Body>> {
        self.response_cache.as_ref()
    }

    /// Returns a reference to the cached body,
    ///
    pub fn cached_body(&self) -> Option<&Body> {
        self.body_cache.as_ref()
    }

    /// Takes the response from the response cache,
    ///
    pub fn take_response(&mut self) -> Option<Response<Body>> {
        self.response_cache.take()
    }

    /// Takes the body from the body cache,
    ///
    pub fn take_body(&mut self) -> Option<Body> {
        self.body_cache.take()
    }

    /// Returns a clone of the block that originated this context,
    ///
    /// Typically using this directly is advanced, it's more likely that state/state_mut are used to get state data,
    ///
    pub fn block(&self) -> Block {
        self.block.clone()
    }

    /// Returns the current state of this thunk context,
    ///
    pub fn state(&self) -> &(impl AttributeIndex + Clone) {
        &self.graph
    }

    /// Returns a mutable reference to the underlying state,
    ///
    pub fn state_mut(&mut self) -> &mut (impl AttributeIndex + Clone) {
        &mut self.graph
    }

    /// Returns an immutable reference to the previous state,
    ///
    pub fn previous(&self) -> Option<&impl AttributeIndex> {
        self.previous_graph.as_ref()
    }

    /// Returns an attribute index that checks both current and previous states for values,
    ///
    pub fn search(&self) -> &impl AttributeIndex {
        self
    }

    /// Returns the workspace in use,
    ///
    pub fn workspace(&self) -> Option<&Workspace> {
        self.workspace.as_ref()
    }

    /// Returns the work directory to use,
    ///
    pub fn work_dir(&self) -> Option<PathBuf> {
        match &self.workspace {
            Some(workspace) => Some(workspace.work_dir().clone()),
            None => self
                .search()
                .find_symbol("work_dir")
                .and_then(|d| Some(PathBuf::from(d))),
        }
    }

    /// Returns the current tag to use,
    ///
    pub fn tag(&self) -> Option<String> {
        self.workspace.as_ref().and_then(|w| w.tag().cloned())
    }

    /// Finds and spawns an operation by name and tag from the current workspace,
    ///
    /// Blocks until a result is received, and returns the thunk context if successful
    ///
    pub async fn run(&self, operation_name: impl AsRef<str>) -> Option<ThunkContext> {
        let tag = self.tag().unwrap_or(String::from("operation"));

        if let Some(operation) = self
            .appendix
            .as_ref()
            .and_then(|a| a.find_operation(&tag, operation_name.as_ref()))
            .and_then(|op| self.dispatch_node_command(NodeCommand::Spawn(op)))
        {
            match operation.await {
                Ok(tc) => Some(tc),
                Err(err) => {
                    error!(
                        "Encountered error while yielding for spawned operation, {err}"
                    );
                    None
                }
            }
        } else {
            warn!(
                "Did not find operation, {} {}",
                operation_name.as_ref(),
                tag
            );
            None
        }
    }

    /// Copies previous state to the current state,
    ///
    /// This is so that previous values can move forward to the next plugin, when .commit() gets called.
    ///
    pub fn copy_previous(&mut self) {
        if let Some(previous) = self.previous() {
            for (name, value) in previous.values() {
                for value in value {
                    self.state_mut().with(&name, value);
                }
            }
        }
    }

    /// Returns a new context with state,
    ///
    pub fn with_state(&self, state: impl Into<AttributeGraph>) -> Self {
        let mut context = self.clone();
        context.graph = state.into();
        context
    }

    /// Sets the current graph,
    ///
    pub fn set_state(&mut self, state: impl Into<AttributeGraph>) {
        self.graph = state.into();
    }

    /// Returns a new context with state,
    ///
    pub fn with_block(&self, block: &Block) -> Self {
        let mut context = self.clone();
        context.block = block.clone();
        context
    }

    /// Sets the current block,
    ///
    pub fn set_block(&mut self, block: &Block) {
        self.block = block.clone();
    }

    /// Returns a new context with the current state committed to the
    /// previous field,
    ///
    pub fn commit(&self) -> Self {
        if self.response_cache.is_some() {
            warn!(
                "Committing context without consuming response_cache"
            );
        }

        if self.body_cache.is_some() {
            warn!(
                "Committing context without consuming body_cache"
            );
        }

        let mut clone = self.clone();
        clone.previous_graph = Some(clone.graph.clone());
        clone
    }

    /// Takes any un-clonable fields from the context, and add's it to a committed version of the context,
    ///
    pub fn consume(&mut self) -> Self {
        if self.response_cache.is_some() || self.body_cache.is_some() {
            let response = self.take_response();
            let body = self.take_body();
            let mut comitted = self.commit();
            if let Some(response) = response {
                comitted.cache_response(response);
            }
            if let Some(body) = body {
                comitted.cache_body(body);
            }
            comitted
        } else {
            self.commit()
        }
    }

    /// Returns true if the property is some boolean
    ///
    pub fn is_enabled(&self, property: impl AsRef<str>) -> bool {
        self.graph.find_bool(property).unwrap_or_default()
    }

    /// Returns true if the source has been cancelled.
    /// Note: In most cases you could just use tokio::select! macro with the source,
    /// but there are control flows where getting a boolean is more ergonomic.
    /// (Example: Timer uses this, while Process uses select!)
    pub fn is_cancelled(cancel_source: &mut oneshot::Receiver<()>) -> bool {
        match cancel_source.try_recv() {
            Ok(_) | Err(tokio::sync::oneshot::error::TryRecvError::Closed) => true,
            _ => false,
        }
    }

    /// Returns a context w/ async features enabled
    ///
    /// **Caveat** As long as the thunk context was used w/ a plugin and event runtime,
    /// these dependencies will be injected at runtime. This is so the context can be configured
    /// w/o needing to initialize additional async dependencies.
    ///
    /// **Caveat** This method will set the parent entity for the underlying attribute graph,
    /// and also update all attributes currently in context
    ///
    pub fn enable_async(&self, entity: Entity, handle: Handle) -> ThunkContext {
        let mut async_enabled = self.clone();
        async_enabled.entity = Some(entity);
        async_enabled.handle = Some(handle);
        async_enabled
    }

    /// Sets the entity,
    ///
    pub fn set_entity(&mut self, entity: Entity) {
        self.entity = Some(entity);
    }

    /// Returns the entity,
    ///
    pub fn entity(&self) -> Option<Entity> {
        self.entity.clone()
    }

    /// Enables a guest on this context's entity, returns true if the guest was dispatched,
    ///
    pub fn enable_guest(&self, guest: Guest) -> bool {
        if let (Some(_), Some(guest_dispatcher)) = (self.entity, self.guest_dispatcher.as_ref()) {
            match guest_dispatcher.try_send(guest) {
                Ok(_) => true,
                Err(err) => {
                    error!("Error sending a guest {err}");
                    false
                }
            }
        } else {
            false
        }
    }

    /// Eanbles the appendix for this context,
    ///
    pub fn enable_appendix(&mut self, appendix: Arc<Appendix>) {
        self.appendix = Some(appendix);
    }

    /// Enables the remote protocol on this thunk,
    ///
    pub fn enable_remote(&mut self, remote: RemoteProtocol) {
        self.remote = Some(remote);
    }

    /// Returns a remote protocol,
    ///
    pub fn remote(&self) -> Option<RemoteProtocol> {
        self.remote.clone()
    }

    /// Returns a context w/ an https client
    ///
    /// The event runtime creates a client on setup, and passes a clone to each thunk context
    /// when enabling this dependency. HTTPS clients are intended to be cheap to clone, and the
    /// underlying connection pool will be reused.
    ///
    pub fn enable_https_client(&mut self, client: SecureClient) -> &mut ThunkContext {
        self.client = Some(client);
        self
    }

    /// Returns a context w/ an operation dispatcher
    ///
    /// Plugins using this context will be able to dispatch operations for the underlying system to
    /// handle.
    ///
    pub fn enable_operation_dispatcher(
        &mut self,
        dispatcher: Sender<Operation>,
    ) -> &mut ThunkContext {
        self.operation_dispatcher = Some(dispatcher);
        self
    }

    /// Returns a context w/ the status update channel enabled
    ///
    pub fn enable_status_updates(
        &mut self,
        status_updates: Sender<StatusUpdate>,
    ) -> &mut ThunkContext {
        self.status_updates = Some(status_updates);
        self
    }

    /// Returns a context w/ the guest sender enabled
    ///
    pub fn enable_guest_dispatcher(&mut self, guest: Sender<Guest>) -> &mut ThunkContext {
        self.guest_dispatcher = Some(guest);
        self
    }

    /// Returns a context w/ the node dispatcher enabled
    ///
    pub fn enable_node_dispatcher(
        &mut self,
        nodes: Sender<(NodeCommand, Option<Yielding>)>,
    ) -> &mut ThunkContext {
        self.node_dispatcher = Some(nodes);
        self
    }

    pub fn enable_completion_dispatcher(
        &mut self,
        completions: Sender<Completion>,
    ) -> &mut ThunkContext {
        self.completion_dispatcher = Some(completions);
        self
    }

    /// Enable a workspace for this context,
    ///
    pub fn enable_workspace(&mut self, workspace: Workspace) {
        self.workspace = Some(workspace);
    }

    cfg_editor! {
    /// Enables watching the runtime's host editor,
    ///
    pub fn enable_host_editor_watcher(
        &mut self,
        watcher: tokio::sync::watch::Receiver<HostEditor>,
    ) {
        self.host_editor_watcher = Some(watcher);
    }

    /// Returns the current state of the host editor,
    ///
    pub fn host_editor(&self) -> Option<HostEditor> {
        if let Some(host_editor) = self.host_editor_watcher.as_ref() {
            Some(host_editor.borrow().clone())
        } else {
            None
        }
    }

    /// Returns the next host editor, if None, either there was an error or this feature is not enabled,
    /// or the channel has closed.
    ///
    pub async fn next_host_editor(&mut self) -> Option<HostEditor> {
        if let Some(watcher) = self.host_editor_watcher.as_mut() {
            match watcher.changed().await {
                Ok(_) => Some(watcher.borrow().clone()),
                Err(err) => {
                    error!("Error watching host editor for changes {err}");
                    None
                }
            }
        } else {
            None
        }
    }

    /// Returns a new host watcher receiver,
    ///
    pub fn host_watcher(&self) -> Option<tokio::sync::watch::Receiver<HostEditor>> {
        if let Some(recv) = self.host_editor_watcher.as_ref() {
            Some(recv.clone())
        } else {
            None
        }
    }
    }

    /// Enables a tcp listener for this context to listen to. accepts the first listener, creates a connection
    /// and then exits after the connection is dropped.
    ///
    pub async fn enable_listener(
        &self,
        cancel_source: &mut oneshot::Receiver<()>,
    ) -> Option<(tokio::net::TcpStream, SocketAddr)> {
        if let Some(address) = self.local_tcp_addr {
            let listener = TcpListener::bind(address)
                .await
                .expect("needs to be able to bind to an address");

            let local_addr = listener.local_addr().expect("was just created").to_string();
            info!(
                "Entity {} Listening on {local_addr}",
                self.entity.expect("should have an entity").id()
            );

            // let name = self.block.name();
            // let symbol = self.block.symbol();
            // let hash_code = self.hash_code();
            // TODO: Assign test.publish.

            select! {
                Ok((stream, address)) = listener.accept() => {
                    Some((stream, address))
                },
                _ = cancel_source => {
                    warn!("{local_addr} is being cancelled");
                    None
                }
            }
        } else {
            error!("No local address assigned to context");
            None
        }
    }

    /// Creates a UDP socket for this context, and saves the address to the underlying graph
    ///
    pub async fn enable_socket(&mut self) -> Option<Arc<UdpSocket>> {
        if let Some(address) = self.local_udp_addr {
            match UdpSocket::bind(address).await {
                Ok(socket) => {
                    if let Some(address) =
                        socket.local_addr().ok().and_then(|a| Some(a.to_string()))
                    {
                        debug!("created socket at {address}");

                        // Add the socket address as a transient value
                        // self.define("socket", "address")
                        //     .edit_as(Value::TextBuffer(address));
                        self.udp_socket = Some(Arc::new(socket));
                    }

                    self.udp_socket.clone()
                }
                Err(err) => {
                    error!("could not enable socket {err}");
                    None
                }
            }
        } else {
            error!("No local address assigned to context");
            None
        }
    }

    /// Assigns addresses to the context by trying to bind to the address,
    ///
    pub async fn assign_addresses(&mut self) {
        if let Some(udp) = self.search().find_symbol("udp") {
            match UdpSocket::bind(&udp).await {
                Ok(socket) => match socket.local_addr() {
                    Ok(addr) => {
                        self.local_udp_addr = Some(addr);
                    }
                    Err(err) => {
                        error!(
                            "Could not get local socket address for udp socket, {udp} {err}"
                        );
                    }
                },
                Err(err) => {
                    error!(
                        "Could not assign address for udp socket, {udp} {err}"
                    );
                }
            }
        }

        if let Some(tcp) = self.search().find_symbol("tcp") {
            match TcpListener::bind(&tcp).await {
                Ok(listener) => match listener.local_addr() {
                    Ok(addr) => {
                        self.local_tcp_addr = Some(addr);
                    }
                    Err(err) => {
                        error!(
                            "Could not get local address for tcp listener, {tcp} {err}"
                        );
                    }
                },
                Err(err) => {
                    error!(
                        "Could not assign address for tcp listener, {tcp} {err}"
                    );
                }
            }
        }
    }

    /// Returns a secure http client. By default this context will only
    /// support http using a secure client,
    ///
    /// If a plugin wishes to make insecure requests,
    /// they must generate an insecure http client at runtime.
    ///
    pub fn client(&self) -> Option<SecureClient> {
        self.client.clone()
    }

    /// Returns a handle to the tokio runtime for spawning additional tasks. Uncommon to use in most cases,
    /// as .task() is a more ergonomic api to use.
    ///
    /// Caveat: enable_async() must be called, for this to be enabled. Will automatically be enabled by the
    /// event runtime once the plugin using this context starts.
    ///
    pub fn handle(&self) -> Option<Handle> {
        self.handle.as_ref().and_then(|h| Some(h.clone()))
    }

    /// Returns a node dispatcher,
    ///
    pub fn node_dispatcher(&self) -> Option<Sender<(NodeCommand, Option<Yielding>)>> {
        self.node_dispatcher.clone()
    }

    /// Dispatches a completion,
    ///
    pub fn dispatch_completion(&self, completion: Completion) {
        if let Some(disp) = self.completion_dispatcher.as_ref() {
            match disp.try_send(completion) {
                Ok(_) => {}
                Err(err) => {
                    error!("Could not dispatch {err}");
                }
            }
        }
    }

    /// Sends a node command, if the command was a spawn command assumes yielding and returns a receiver to get the result on completion,
    ///
    pub fn dispatch_node_command(
        &self,
        command: NodeCommand,
    ) -> Option<tokio::sync::oneshot::Receiver<ThunkContext>> {
        if let Some(node_dispatcher) = self.node_dispatcher.as_ref() {
            match command {
                NodeCommand::Spawn(_) => {
                    let (yielding, receiver) = Yielding::new(self.clone());
                    match node_dispatcher.try_send((command, Some(yielding))) {
                        Ok(_) => Some(receiver),
                        Err(err) => {
                            error!("Could not send command, {err}");
                            None
                        }
                    }
                }
                _ => match node_dispatcher.try_send((command, None)) {
                    Ok(_) => None,
                    Err(err) => {
                        error!("Could not send command, {err}");
                        None
                    }
                },
            }
        } else {
            None
        }
    }

    /// Dispatches a custom node command,
    ///
    pub fn dispatch_custom_node_command(
        &self,
        command: impl AsRef<str>,
    ) -> Option<tokio::sync::oneshot::Receiver<ThunkContext>> {
        if let Some(node_dispatcher) = self.node_dispatcher.as_ref() {
            // Create a yield so that a response can be received,
            let (yielding, receiver) = Yielding::new(self.clone());

            // Send the custom node command,
            match node_dispatcher.try_send((
                NodeCommand::custom(
                    command.as_ref(),
                    self.entity().expect("should have an entity"),
                ),
                Some(yielding),
            )) {
                // Return a receiver which is a thunk context,
                Ok(_) => Some(receiver),
                Err(err) => {
                    error!("Could not send command, {err}");
                    None
                }
            }
        } else {
            None
        }
    }

    /// Spawns and executes a task that will be managed by the event runtime
    ///
    /// Caveat: async must be enabled for this api to work, otherwise it will result in a
    /// no-op
    ///
    pub fn task<F>(
        &self,
        task: impl FnOnce(CancelSource) -> F,
    ) -> Option<(JoinHandle<ThunkContext>, CancelToken)>
    where
        F: Future<Output = Option<ThunkContext>> + Send + 'static,
    {
        if let Self {
            handle: Some(handle),
            ..
        } = self
        {
            let default_return = self.clone();
            let (tx, cancel) = channel::<()>();

            let task = (task)(cancel);
            Some((
                handle.spawn(async {
                    match task.await {
                        Some(next) => next,
                        None => default_return,
                    }
                }),
                tx,
            ))
        } else {
            None
        }
    }

    /// Returns a task if a task should be awaited,
    ///
    /// The task function can return a Result to allow for better error handling,
    ///
    pub fn task_with_result<F>(
        &self,
        task: impl FnOnce(CancelSource) -> F,
    ) -> Option<(JoinHandle<ThunkContext>, CancelToken)>
    where
        F: Future<Output = Result<ThunkContext, Error>> + Send + 'static,
    {
        if let Self {
            handle: Some(handle),
            ..
        } = self
        {
            let mut default_return = self.clone();
            let (tx, cancel) = channel::<()>();

            let task = (task)(cancel);
            Some((
                handle.spawn(async {
                    match task.await {
                        Ok(next) => next,
                        Err(err) => {
                            default_return.error = Some(err);
                            default_return
                        }
                    }
                }),
                tx,
            ))
        } else {
            None
        }
    }

    /// Sends an update for the status and progress
    ///
    pub async fn progress(&self, status: impl AsRef<str>, progress: f32) {
        if let ThunkContext {
            status_updates: Some(status_updates),
            entity: Some(entity),
            ..
        } = self
        {
            match status_updates
                .send((*entity, progress, status.as_ref().to_string()))
                .await
            {
                Ok(_) => {}
                Err(_) => {}
            }
        }
        // event!(Level::TRACE, "progress {}, {}", progress, status.as_ref());
    }

    /// Updates status of thunk execution
    ///
    pub async fn status(&self, status: impl AsRef<str>) {
        self.progress(&status, -1.0).await;
    }

    /// Returns the error context this context has an error block
    ///
    /// Notes: When a plugin completes, the event_runtime will call this method
    /// to determine how to handle the error.
    ///
    /// If the graph contains a bool `stop_on_error`, the event runtime will
    /// not execute any of the next events in the sequence
    ///  
    pub fn get_errors(&self) -> Option<ErrorContext> {
        self.error_graph.as_ref().and_then(|e| {
            if self.is_enabled("stop_on_error") {
                Some(ErrorContext::new(e.clone(), self.entity.clone()))
            } else {
                Some(ErrorContext::new(e.clone(), None))
            }
        })
    }

    /// Returns the current error state,
    /// 
    pub fn err(&self) -> Option<&Error> {
        self.error.as_ref()
    }
}

/// Functions consuming special attributes,
///
impl ThunkContext {
    /// Formats a message w/ symbols from state,
    ///
    pub fn format(&self, message: impl AsRef<str>) -> String {
        Fmt::apply(self, message.as_ref())
    }
}

/// Methods for working with the scoket
///  
impl ThunkContext {
    /// Returns the underlying udp socket, if a socket has been enabled on this context
    ///
    pub fn socket(&self) -> Option<Arc<UdpSocket>> {
        self.udp_socket.clone()
    }

    /// If the socket is enabled for this context, returns the SocketAddr for the socket
    ///
    pub fn socket_address(&self) -> Option<SocketAddr> {
        self.socket().and_then(|s| s.local_addr().ok())
    }

    /// If the socket is enabled for this context, returns the block address
    ///
    // pub fn to_block_address(&self) -> Option<BlockAddress> {
    //     if let Some(_socket_addr) = self.socket_address() {
    //         // let address = BlockAddress::new(self);
    //         // Some(address.with_socket_addr(socket_addr))
    //         todo!()
    //     } else {
    //         None
    //     }
    // }

    /// Read lines from a stream and returns the result,
    ///
    pub async fn readln_stream(&self) -> String {
        use std::fmt::Write;

        let address = self
            .search()
            .find_symbol("address")
            .expect("should have an address");

        let stream = tokio::net::TcpStream::connect(address)
            .await
            .expect("Should be able to connect");
        debug!("Connecting to stream");

        stream.readable().await.ok();

        debug!("Reading from stream");
        let mut lines = BufReader::new(stream).lines();

        let mut received = String::new();

        while let Ok(Some(line)) = lines.next_line().await {
            writeln!(received, "{line}").ok();
        }
        received
    }

    /// Returns the local tcp addr,
    ///
    pub fn local_tcp_addr(&self) -> Option<SocketAddr> {
        self.local_tcp_addr
    }

    /// Returns the local udp addr,
    ///
    pub fn local_udp_addr(&self) -> Option<SocketAddr> {
        self.local_udp_addr
    }
}

/// Some utility methods
///
impl ThunkContext {
    /// Updates error block
    pub fn error(&mut self, record: impl Fn(&mut AttributeGraph)) {
        let mut error_graph = self.graph.clone();
        record(&mut error_graph);
        self.error_graph = Some(error_graph);
    }
}
