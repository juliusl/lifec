use std::{future::Future, net::SocketAddr, path::PathBuf, sync::Arc};

use crate::prelude::*;
use reality::Block;
use specs::{Component, DenseVecStorage, Entity};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    net::{TcpListener, UdpSocket},
    runtime::Handle,
    select,
    sync::{
        self,
        mpsc::Sender,
        oneshot::{self, channel},
    },
    task::JoinHandle,
};
use tracing::event;
use tracing::Level;

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
#[derive(Debug, Component, Default, Clone)]
#[storage(DenseVecStorage)]
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

    /// Entity that owns this context
    entity: Option<Entity>,
    /// Tokio runtime handle, to spawn additional tasks
    handle: Option<Handle>,
    /// Client for sending secure http requests
    client: Option<SecureClient>,
    /// Workspace for this context, if enabled the work_dir from the workspace can be used
    workspace: Option<Workspace>,
    /// Local tcp socket address,
    local_tcp_addr: Option<SocketAddr>,
    /// Local udp socket address,
    local_udp_addr: Option<SocketAddr>,

    /// # I/O Utilities,
    /// Project-type implements handling the listener side,     

    /// Sender for status updates for the thunk
    status_updates: Option<Sender<StatusUpdate>>,
    /// Dispatcher for runmd
    dispatcher: Option<Sender<RunmdFile>>,
    /// Dispatcher for operations
    operation_dispatcher: Option<Sender<Operation>>,
    /// Dispatcher for start commands
    start_command_dispatcher: Option<Sender<Start>>,
    /// Channel to send bytes to a listening char_device
    char_device: Option<Sender<(u32, u8)>>,

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
}

/// This block has all the async related features
impl ThunkContext {
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

    /// Returns a new context with state,
    ///
    pub fn with_block(&self, block: &Block) -> Self {
        let mut context = self.clone();
        context.block = block.clone();
        context
    }

    /// Returns a new context with the current state committed to the
    /// previous field,
    ///
    pub fn commit(&self) -> Self {
        let mut clone = self.clone();
        clone.previous_graph = Some(clone.graph.clone());
        clone
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

    /// Returns a context w/ a dispatcher
    ///
    /// Plugins using this context will be able to dispatch attribute graphs to the underlying
    /// runtime.
    ///
    pub fn enable_dispatcher(&mut self, dispatcher: Sender<RunmdFile>) -> &mut ThunkContext {
        self.dispatcher = Some(dispatcher);
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

    /// Returns a context w/ the start command dispatcher enabled
    ///
    pub fn enable_start_command_dispatcher(
        &mut self,
        start_commands: Sender<Start>,
    ) -> &mut ThunkContext {
        self.start_command_dispatcher = Some(start_commands);
        self
    }

    /// Enables output to a char_device, a plugin can use to output bytes to.
    ///
    /// The implementation of the char_device, can choose how to handle this output,
    /// but in general this is mainly useful for tty type of situations. For example,
    /// the Process and Remote plugins write to this device.
    ///
    pub fn enable_output(&mut self, tx: Sender<(u32, u8)>) {
        self.char_device = Some(tx);
    }

    /// Enable a workspace for this context,
    ///
    pub fn enable_workspace(&mut self, workspace: Workspace) {
        self.workspace = Some(workspace);
    }

    /// Enables a tcp listener for this context to listen to. accepts the first listener, creates a connection
    /// and then exits after the connection is dropped.
    ///
    /// Returns a buffered reader over each line sent over the stream.
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
            event!(
                Level::INFO,
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
                    event!(Level::WARN, "{local_addr} is being cancelled");
                    None
                }
            }
        } else {
            event!(Level::ERROR, "No local address assigned to context");
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
                        event!(Level::DEBUG, "created socket at {address}");

                        // Add the socket address as a transient value
                        // self.define("socket", "address")
                        //     .edit_as(Value::TextBuffer(address));
                        self.udp_socket = Some(Arc::new(socket));
                    }

                    self.udp_socket.clone()
                }
                Err(err) => {
                    event!(Level::ERROR, "could not enable socket {err}");
                    None
                }
            }
        } else {
            event!(Level::ERROR, "No local address assigned to context");
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
                        event!(
                            Level::ERROR,
                            "Could not get local socket address for udp socket, {udp} {err}"
                        );
                    }
                },
                Err(err) => {
                    event!(
                        Level::ERROR,
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
                        event!(
                            Level::ERROR,
                            "Could not get local address for tcp listener, {tcp} {err}"
                        );
                    }
                },
                Err(err) => {
                    event!(
                        Level::ERROR,
                        "Could not assign address for tcp listener, {tcp} {err}"
                    );
                }
            }
        }
    }

    /// Sends a character to a the char_device if it exists
    ///
    /// Caveat: If `enable_output`/`enable_async` haven't been called this is a no-op
    ///
    pub async fn send_char(&self, c: u8) {
        if let Some(entity) = self.entity {
            if let Some(char_device) = &self.char_device {
                match char_device.send((entity.id(), c)).await {
                    Ok(_) => event!(Level::TRACE, "sent char for {:?}", entity),
                    Err(err) => event!(Level::ERROR, "error sending char, {err}, {:?}", entity),
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

    /// Dipatches .runmd to a listener capable of interpreting and creating blocks at runtime
    ///
    /// Note: Since thunk context is clonable, it's easy to inject into other libraries such as logos and poem.
    /// For example, if running a runtime within a plugin that hosts a web api, you can use this method within
    /// request handlers to dispatch blocks to the hosting runtime.
    ///
    pub async fn dispatch(&self, symbol: impl AsRef<str>, source: impl AsRef<str>) {
        if let Some(dispatcher) = &self.dispatcher {
            dispatcher
                .send(RunmdFile::new_src(symbol, source.as_ref().to_string()))
                .await
                .ok();
        }
    }

    /// Dispatches a start command,
    ///
    pub async fn dispatch_start_command(&self, start_command: Start) {
        if let Some(dispatcher) = &self.start_command_dispatcher {
            dispatcher.send(start_command).await.ok();
        }
    }

    /// Returns the underlying dispatch transmitter
    ///
    pub fn dispatcher(&self) -> Option<sync::mpsc::Sender<RunmdFile>> {
        self.dispatcher.clone()
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

    /// Sends an update for the status and progress
    ///
    pub async fn update_progress(&self, status: impl AsRef<str>, progress: f32) {
        // if let ThunkContext {
        //     status_updates: Some(status_updates),
        //     entity: Some(entity),
        //     ..
        // } = self
        // {
        //     match status_updates
        //         .send((*entity, progress, status.as_ref().to_string()))
        //         .await
        //     {
        //         Ok(_) => {}
        //         Err(_) => {}
        //     }
        // }
        event!(Level::TRACE, "progress {}, {}", progress, status.as_ref());
    }

    /// Updates status of thunk execution
    ///
    pub async fn update_status_only(&self, status: impl AsRef<str>) {
        // TODO self.update_progress(&status, 0.0).await;
        event!(Level::TRACE, "{}", status.as_ref())
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
    pub fn to_block_address(&self) -> Option<BlockAddress> {
        if let Some(_socket_addr) = self.socket_address() {
            // let address = BlockAddress::new(self);
            // Some(address.with_socket_addr(socket_addr))
            todo!()
        } else {
            None
        }
    }

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
        event!(Level::DEBUG, "Connecting to stream");

        stream.readable().await.ok();

        event!(Level::DEBUG, "Reading from stream");
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

/// Cryptography related functions, Private Key side
///
impl ThunkContext {
    /// Create a signature using the assigned signature key,
    ///
    pub fn sign(&self) {
        /*
            1) Get assigned identity of the current block,
            - The prefix should be {block.name()}.{block.symbol()}.
            - or, {block.symbol()}.control.
            2) Lookup the private-key with the assigned identity

            After a listener starts and is bound to an address,
            I need to be able to assign the address to a name.

            receive.runner.{host}/{path}

            receive.runner.obddemo.azurecr.io/demo/library/redis/6.2.1

            1) look up host
            2) look up path
            3) look up block name/symbol

            4) What private key to use?
            {host}
                -> {block}
                    -> {key}

        Name File:
        {
            "data": {
                // Settings from Project struct
                "host": "",
                "container": "",
                "path": "",

                // Full entity name
                "name": "",
                "symbol": "",
                "local_addr": "",

                "signatures": {
                    "host": "", // Find this in .world/{host}/
                    "container": "" // Find this in .world/{host}/{container}/
                    "path": "", // Find this in .world/{host}/{container}/{path}/

                    "name": "",
                    "symbol": "",
                    "local_addr": "",
                },
            },
            "signature": ""
        }

        */
    }

    /// Decrypt some bytes using the assigned decryption key,
    ///
    pub fn decrypt(&self) {}
}

/// Cryptography related functions, Public Key side
///
impl ThunkContext {
    /// Verify the signature of some bytes using the assigned verifying key,
    ///
    pub fn verify(&self) {}

    /// Encrypt some bytes using the assigned encryption key
    ///
    pub fn encrypt(&self) {}
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

#[test]
fn test_security() {
    use rsa::pkcs8::EncodePublicKey;
    use rsa::pss::BlindedSigningKey;
    use rsa::{pss::VerifyingKey, RsaPrivateKey};
    use sha2::Sha256;
    // use std::str::from_utf8;
    // use rsa::pkcs8::DecodePrivateKey;
    // use rsa::pkcs1::EncodeRsaPublicKey;
    // use pkcs8::EncodePrivateKey;

    use signature::{RandomizedSigner, Signature, Verifier};
    let mut rng = rand::thread_rng();

    let bits = 2048;
    let private_key = RsaPrivateKey::new(&mut rng, bits).expect("failed to generate a key");

    // private_key.to_pkcs8_encrypted_der(&mut rng, "test1234");

    let signing_key = BlindedSigningKey::<Sha256>::new(private_key);
    // signing_key.to_pkcs8_encrypted_der(rng, "test").unwrap().write_der_file(path);
    let verifying_key: VerifyingKey<_> = (&signing_key).into();

    // Sign
    let data = b"hello world";
    let signature = signing_key.sign_with_rng(&mut rng, data);
    assert_ne!(signature.as_bytes(), data);

    // Verify
    verifying_key
        .verify(data, &signature)
        .expect("failed to verify");

    match verifying_key.to_public_key_der() {
        Ok(_document) => {}
        Err(_) => todo!(),
    }

    // let mut stdin = std::io::stdin().lock();
    // let mut pass = vec![];
    // rpassword::prompt_password_from_bufread(&mut stdin, &mut pass, "Password for key:").expect("should have a value");

    // RsaPrivateKey::from_pkcs8_encrypted_pem(
    //     "",
    //     from_utf8(pass.as_slice()).expect("should be a string"),
    // ).ok();
}
