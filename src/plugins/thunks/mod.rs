use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::AttributeGraph;
use crate::Operation;
use crate::RuntimeDispatcher;
use crate::state::AttributeIndex;
use atlier::system::Value;
use hyper::client::HttpConnector;
use imgui::Ui;
use specs::Component;
use specs::{storage::DenseVecStorage, Entity};

mod open_file;
pub use open_file::OpenFile;

mod open_dir;
pub use open_dir::OpenDir;

mod write_file;
use tokio::io;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::net::TcpListener;
use tokio::net::UdpSocket;
use tokio::select;
use tokio::sync;
use tokio::sync::oneshot;
use tracing::Level;
use tracing::event;
pub use write_file::WriteFile;

mod timer;
pub use timer::Timer;

mod println;
pub use println::Println;

mod error;
pub use error::ErrorContext;

mod dispatch;
pub use dispatch::Dispatch;

use super::block::BlockAddress;
use super::{BlockContext, Plugin, Project};
use tokio::{runtime::Handle, sync::mpsc::Sender, sync::oneshot::channel, task::JoinHandle};

/// Thunk is a function that can be passed around for the system to call later
#[derive(Component, Clone)]
#[storage(DenseVecStorage)]
pub struct Thunk(
    // thunk label
    pub &'static str,
    // thunk fn
    pub fn(&mut ThunkContext) -> Option<(JoinHandle<ThunkContext>, CancelToken)>,
);

/// Config for a thunk context
#[derive(Component, Clone)]
#[storage(DenseVecStorage)]
pub struct Config(
    /// config label
    pub &'static str,
    /// config fn
    pub fn(&mut ThunkContext),
);

impl AsRef<Config> for Config {
    fn as_ref(&self) -> &Config {
        self
    }
}

impl Thunk {
    /// Generates a thunk from a plugin impl
    pub fn from_plugin<P>() -> Self
    where
        P: Plugin<ThunkContext>,
    {
        Self(P::symbol(), P::call_with_context)
    }

    /// deprecated?
    pub fn show(&self, context: &mut ThunkContext, ui: &Ui) {
        ui.set_next_item_width(130.0);
        if ui.button(context.label(self.0)) {
            let Thunk(.., thunk) = self;
            thunk(context);
        }
    }
}

/// StatusUpdate for stuff like progress bars
pub type StatusUpdate = (
    // entity with an update
    Entity, 
    // progress
    f32, 
    // status message 
    String
);

/// Cancel token stored by the event runtime
pub type CancelToken = tokio::sync::oneshot::Sender<()>;

/// Cancel source stored by the thunk
pub type CancelSource = tokio::sync::oneshot::Receiver<()>;

/// Secure client for making http requests
pub type SecureClient = hyper::Client<hyper_tls::HttpsConnector<HttpConnector>>;

#[derive(Component)]
#[storage(DenseVecStorage)]
pub struct CancelThunk(
    // Oneshot channel that cancels the thunk
    pub CancelToken
);

impl From<CancelToken> for CancelThunk {
    fn from(token: CancelToken) -> Self {
        Self(token)
    }
}

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
#[derive(Component, Default, Clone)]
#[storage(DenseVecStorage)]
pub struct ThunkContext {
    /// Underlying block context for this thunk
    pub block: BlockContext,
    /// Current project
    pub project: Option<Project>,
    /// Async fields
    /// Entity that is identifying the thunk
    pub entity: Option<Entity>,
    /// Tokio runtime handle, to spawn additional tasks 
    pub handle: Option<Handle>,
    /// Sender for status updates for the thunk
    status_updates: Option<Sender<StatusUpdate>>,
    /// Client for sending secure http requests
    client: Option<SecureClient>,
    /// Dispatcher for attribute graphs
    dispatcher: Option<Sender<AttributeGraph>>,
    /// Dispatcher for attribute graphs
    operation_dispatcher: Option<Sender<Operation>>,
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
    udp_socket: Option<Arc<UdpSocket>>,
}

impl AttributeIndex for ThunkContext {
    fn entity_id(&self) -> u32 {
        self.entity.and_then(|e| Some(e.id())).unwrap_or_default()
    }

    fn add_attribute(&mut self, attr: atlier::system::Attribute) {
        self.as_mut().add_attribute(attr)
    }

    fn define(&mut self, name: impl AsRef<str>, symbol: impl AsRef<str>) -> &mut atlier::system::Attribute {
        self.as_mut().define(name, symbol)
    }

    fn find_value(&self, with_name: impl AsRef<str>) -> Option<Value> {
        self.as_ref().find_attr_value(with_name).and_then(|v| Some(v.clone()))
    }

    fn find_transient(&self, with_name: impl AsRef<str>, with_symbol: impl AsRef<str>) -> Option<&atlier::system::Attribute> {
        self.as_ref().find_attr(format!("{}::{}", with_name.as_ref(), with_symbol.as_ref()))
    }
}

/// This block has all the async related features
impl ThunkContext {
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
    pub fn enable_async(
        &self,
        entity: Entity,
        handle: Handle,
    ) -> ThunkContext {
        let mut async_enabled = self.clone();
        async_enabled.entity = Some(entity);
        async_enabled.handle = Some(handle);
        async_enabled.as_mut().set_parent_entity(entity);
        async_enabled
    }

    /// Returns a context w/ an https client
    /// 
    /// The event runtime creates a client on setup, and passes a clone to each thunk context
    /// when enabling this dependency. HTTPS clients are intended to be cheap to clone, and the 
    /// underlying connection pool will be reused.
    /// 
    pub fn enable_https_client(
        &mut self,
        client: SecureClient
    ) -> &mut ThunkContext {
        self.client = Some(client);
        self
    }

    /// Returns a context w/ a project
    /// 
    /// Setting the project allows this context to configure itself
    /// from the blocks defined in the project. 
    /// 
    /// **Caveat** When this is called from the event runtime, the following order will 
    /// be used to determine which project is chosen, 
    /// - Entity has a `Project` component
    /// - Entity has a `Runtime` component
    /// - World's project resource
    /// 
    /// ## The `previous` graph event message
    /// 
    /// When the event runtime executes a sequence, the event runtime will transpile
    /// the project, and send it as a graph message under the moniker `previous`. When
    /// a plugin executes, it can apply this state to it's current graph in order to use the results.
    /// 
    /// i.e. `tc.as_mut().apply("previous")`
    /// 
    /// This method of passing state forward is an explicit gesture. And has high-overhead because the project must be 
    /// transpiled in order to send state forward. 
    /// 
    /// If multiple plugins need to share state, it is more performant to create a composite plugin using combine::<A, B>
    /// A composite plugin will share the same thunk context between each plugin, which bypasses the need to transpile the project.
    /// However, a composite plugin **must** take into account how each plugin will interpret attributes during execution. This is usually
    /// not really an issue, as long as stable attributes are used and declared consistently. 
    /// 
    pub fn enable_project(
        &mut self,
        project: Project
    ) -> &mut ThunkContext {
        self.project = Some(project);
        self

        // TODO: Check block name
    }

    /// Returns a context w/ a dispatcher 
    /// 
    /// Plugins using this context will be able to dispatch attribute graphs to the underlying
    /// runtime.
    /// 
    pub fn enable_dispatcher(
        &mut self,
        dispatcher: Sender<AttributeGraph>,
    ) -> &mut ThunkContext {
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

    /// Enables output to a char_device, a plugin can use to output bytes to. 
    /// 
    /// The implementation of the char_device, can choose how to handle this output, 
    /// but in general this is mainly useful for tty type of situations. For example,
    /// the Process and Remote plugins write to this device.
    /// 
    pub fn enable_output(&mut self, tx: Sender<(u32, u8)>) {
        self.char_device = Some(tx);
    }

    /// Enables a tcp listener for this context to listen to. accepts the first listener, creates a connection
    /// and then exits after the connection is dropped.
    /// 
    /// Returns a buffered reader over each line sent over the stream.
    /// 
    pub async fn enable_listener(
        &self,
        cancel_source: &mut oneshot::Receiver<()>,
    ) -> Option<io::Lines<BufReader<tokio::net::TcpStream>>> {
       if let Some(_) = self.dispatcher {
            let address = self.as_ref()
                .find_text("address")
                .unwrap_or("127.0.0.1:0".to_string());
            
            let listener = TcpListener::bind(address).await.expect("needs to be able to bind to an address");
            let local_addr = listener.local_addr().expect("was just created").to_string();

            event!(Level::DEBUG, "Thunk context is listening on {local_addr}");
            self.update_status_only(
                format!("Lisenting on {local_addr}")
            ).await;

            select! {
                Ok((stream, address)) = listener.accept() => {
                    event!(Level::DEBUG, "{address} is connecting");
                    
                    Some(BufReader::new(stream).lines())
                },
                _ = cancel_source => {
                    event!(Level::WARN, "{local_addr} is being cancelled");
                    None 
                }
            }
       } else {
            event!(Level::ERROR, "Did not have a dispatcher to enable this w/ the runtime ");
            None
       }
    }

    /// Creates a UDP socket for this context, and saves the address to the underlying graph
    /// 
    pub async fn enable_socket(&mut self) -> Option<Arc<UdpSocket>> {
        let address = self.as_ref()
            .find_text("address")
            .unwrap_or("127.0.0.1:0".to_string());

        match UdpSocket::bind(address).await {
            Ok(socket) => {
                if let Some(address) = socket.local_addr().ok().and_then(|a| Some(a.to_string())) {
                    event!(Level::DEBUG, "created socket at {address}");

                    // Add the socket address as a transient value
                    self.as_mut()
                        .define("socket", "address")
                        .edit_as(Value::TextBuffer(address));
                    self.udp_socket = Some(Arc::new(socket));
                }

                self.udp_socket.clone()
            },
            Err(err) => {
                event!(Level::ERROR, "could not enable socket {err}");
                None
            },
        }
    }

    /// Sends a character to a the char_device if it exists 
    /// 
    /// Caveat: If `enable_output`/`enable_async` haven't been called this is a no-op
    pub async fn send_char(&self, c: u8) {
        if let Some(entity) = self.entity {
            event!(Level::TRACE, "sending message to {}", entity.id());
            if let Some(char_device) = &self.char_device {
                event!(Level::TRACE, "has char device");
                match char_device.send((entity.id(), c)).await {
                    Ok(_) => event!(Level::TRACE, "sent byte for {:?}", entity),
                    Err(err) => event!(Level::ERROR, "error sending byte to char_device, {err}, {:?}", entity),
                }
            } else {
                event!(Level::TRACE, "missing char device");
            }
        } else {
            event!(Level::WARN, "entity is not set to send_char");
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
    pub async fn dispatch(&self, runmd: impl AsRef<str>) {
        if let Some(dispatcher) = &self.dispatcher {
            let graph = AttributeGraph::from(0);
            match graph.batch(runmd) {
                Ok(msg) => {
                    dispatcher.send(msg).await.ok();
                },
                Err(_) => todo!(),
            }
        }
    }

    /// Returns the underlying dispatch transmitter
    /// 
    pub fn dispatcher(&self) -> Option<sync::mpsc::Sender<AttributeGraph>> {
        self.dispatcher.clone()
    }

    /// Spawns and executes a task that will be managed by the event runtime
    /// 
    /// Caveat: async must be enabled for this api to work, otherwise it will result in a 
    /// no-op
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
    }

    /// Updates status of thunk execution
    ///
    /// TODO: The ergonomics of this api and the one above need some improvement,
    ///  
    pub async fn update_status_only(&self, status: impl AsRef<str>) {
        self.update_progress(&status, 0.0).await;

        if self.as_ref().is_enabled("debug").unwrap_or_default() {
            let block_name = &self.block.block_name;
            let status = status.as_ref();
            event!(Level::DEBUG, "{block_name}\t{status}"); 
        }
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
        self.block.get_block("error").and_then(|b| { 
            let mut b = b;
            
            if self.as_ref().is_enabled("stop_on_error").unwrap_or_default() {
                b.add_bool_attr("stop_on_error", true);

                if let Some(stopped) = self.entity {
                    return Some(ErrorContext::new(BlockContext::from(b), Some(stopped)))
                }
            }

            Some(ErrorContext::new(BlockContext::from(b), None)) 
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
        if let Some(socket_addr) = self.socket_address() {
            let address = BlockAddress::new(self);
            Some(address.with_socket_addr(socket_addr))
        } else {
            None
        }
    }
}

/// Some utility methods
/// 
impl ThunkContext {
    /// Updates error block
    pub fn error(&mut self, record: impl Fn(&mut AttributeGraph)) {
        if !self.block.update_block("error", &record) {
            self.block.add_block("error", record);
        }
    }

    /// Formats a label that is unique to this state
    pub fn label(&self, label: impl AsRef<str>) -> impl AsRef<str> {
        format!(
            "{} {:#2x}",
            label.as_ref(),
            self.as_ref().hash_code() as u16
        )
    }
}

impl From<AttributeGraph> for ThunkContext {
    fn from(g: AttributeGraph) -> Self {
        Self {
            block: BlockContext::from(g),
            project: None,
            entity: None,
            handle: None,
            client: None,
            status_updates: None,
            dispatcher: None,
            operation_dispatcher: None,
            char_device: None,
            udp_socket: None,
        }
    }
}

impl AsRef<AttributeGraph> for ThunkContext {
    fn as_ref(&self) -> &AttributeGraph {
        self.block.as_ref()
    }
}

impl AsMut<AttributeGraph> for ThunkContext {
    fn as_mut(&mut self) -> &mut AttributeGraph {
        self.block.as_mut()
    }
}
