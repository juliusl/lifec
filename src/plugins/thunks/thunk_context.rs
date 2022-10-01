use std::{net::SocketAddr, sync::Arc, future::Future};

use crate::{ AttributeGraph, Operation, AttributeIndex, plugins::network::BlockAddress};

use specs::{Component, DenseVecStorage, Entity};
use tokio::{
    io::{self, AsyncBufReadExt, BufReader},
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

use super::{SecureClient, StatusUpdate, CancelSource, CancelToken, ErrorContext};

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
    error_graph: Option<AttributeGraph>,
    /// Previous graph used w/ this context
    previous_graph: Option<AttributeGraph>,
    /// Underlying state for this thunk,
    graph: AttributeGraph,
    /// Entity that owns this context
    entity: Option<Entity>,
    /// Tokio runtime handle, to spawn additional tasks
    handle: Option<Handle>,
    /// Sender for status updates for the thunk
    status_updates: Option<Sender<StatusUpdate>>,
    /// Client for sending secure http requests
    client: Option<SecureClient>,
    /// Dispatcher for runmd
    dispatcher: Option<Sender<String>>,
    /// Dispatcher for operations
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
    ///
    udp_socket: Option<Arc<UdpSocket>>,
}

/// This block has all the async related features
impl ThunkContext {
    /// Returns the current state of this thunk context,
    /// 
    pub fn state(&self) -> &impl AttributeIndex {
       &self.graph
    }

    /// Returns a mutable reference to the underlying state,
    /// 
    pub fn state_mut(&mut self) -> &mut impl AttributeIndex {
        &mut self.graph
    }

    /// Returns an immutable reference to the previous state,
    /// 
    pub fn previous(&self) -> Option<&impl AttributeIndex> {
        self.previous_graph.as_ref()
    }

    /// Returns a new context with state, 
    /// 
    pub fn with_state(&self, state: impl Into<AttributeGraph>) -> Self {
        let mut context = self.clone();
        context.graph = state.into();
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
    pub fn enable_dispatcher(&mut self, dispatcher: Sender<String>) -> &mut ThunkContext {
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
            let address = self
                .state()
                .find_symbol("address")
                .unwrap_or("127.0.0.1:0".to_string());

            let listener = TcpListener::bind(address)
                .await
                .expect("needs to be able to bind to an address");
            let local_addr = listener.local_addr().expect("was just created").to_string();

            event!(Level::DEBUG, "Thunk context is listening on {local_addr}");
            self.update_status_only(format!("Lisenting on {local_addr}"))
                .await;

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
            event!(
                Level::ERROR,
                "Did not have a dispatcher to enable this w/ the runtime "
            );
            None
        }
    }

    /// Creates a UDP socket for this context, and saves the address to the underlying graph
    ///
    pub async fn enable_socket(&mut self) -> Option<Arc<UdpSocket>> {
        let address = self
            .state()
            .find_symbol("address")
            .unwrap_or("127.0.0.1:0".to_string());

        match UdpSocket::bind(address).await {
            Ok(socket) => {
                if let Some(address) = socket.local_addr().ok().and_then(|a| Some(a.to_string())) {
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
                    Err(err) => event!(
                        Level::ERROR,
                        "error sending char, {err}, {:?}",
                        entity
                    ),
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
    pub async fn dispatch(&self, runmd: impl AsRef<str>) {
        if let Some(dispatcher) = &self.dispatcher {
            dispatcher.send(runmd.as_ref().to_string()).await.ok();
        }
    }

    /// Returns the underlying dispatch transmitter
    ///
    pub fn dispatcher(&self) -> Option<sync::mpsc::Sender<String>> {
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
