use std::{
    ops::{Deref, DerefMut},
};
use crate::{
    plugins::ErrorContext,
    AttributeGraph, Operation,
};
use specs::{shred::Resource, Component, DenseVecStorage, World, WorldExt};
use tokio::sync::mpsc::{Receiver, Sender};

/// Guest runtime that can be use to receive objects from
/// thunk contexts at runtime.
///
/// If an entity has this component, the event runtime will configure
/// the thunk context to use dispatchers from the guest runtime
///
#[derive(Component)]
#[storage(DenseVecStorage)]
pub struct GuestRuntime {
    /// Proxy transport to the real transport
    ///
    ///  transport: ProxyTransport,
    /// Guest world, isolates objects dispatched from plugins,
    ///
    /// **Note** The dispatcher for this world is managed by the host.
    ///
    world: World,
    /// Enable to allow the transport to receive attribute graphs
    ///
    enable_graph_receiver: bool,
    /// Enable to allow the transport to receive operations
    ///
    enable_operation_receiver: bool,
    /// Enable to allow the transport to receive error contexts
    ///
    enable_error_context_receiver: bool,
}

impl GuestRuntime {
    /// Creates a new guest runtime
    /// 
    /// # Arguments
    /// * `engine` - This is the engine entity on the host 
    /// * `host` - A reference to the host impl for setup 
    /// * `index` - The attribute index source 
    /// * `transport` - The transport the guest will communicate with 
    // pub fn new<H, A, T>(
    //     engine: Entity,
    //     host: &mut H, 
    //     index: A, 
    //     transport: &mut T, 
    // )  -> Self 
    // where
    //     H: Host,
    //     A: AttributeIndex,
    //     T: Transport,
    // {
    //     let enable_graph_receiver = index
    //         .find_bool("enable_graph_receiver")
    //         .unwrap_or_default();
    //     let enable_operation_receiver = index
    //         .find_bool("enable_operation_receiver")
    //         .unwrap_or_default();
    //     let enable_error_context_receiver = index
    //         .find_bool("enable_error_context_receiver")
    //         .unwrap_or_default();

    //     let (mut world, dispatcher) = H::new_world();

    //     // Install deps
    //     let runtime = host.get_runtime(engine); 
    //     world.insert(runtime.clone());
    //     // world.insert(runtime.project.clone());

    //     let mut dispatcher = dispatcher.build();
    //     dispatcher.setup(&mut world);
    //     host.add_guest(engine, dispatcher);

    //     let transport = transport.proxy();
    //     Self {
    //         transport,
    //         enable_graph_receiver,
    //         enable_operation_receiver,
    //         enable_error_context_receiver,
    //         world,
    //     }
    // }

    /// Returns the guest's world
    ///
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Returns a mutable reference to the guest's world
    ///
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Returns a graph sender from the guest's world
    ///
    pub fn get_graph_sender(&self) -> Option<Sender<String>> {
        if self.enable_graph_receiver {
            self.sender()
        } else {
            None
        }
    }

    /// Returns a operation sender from the guest's world
    ///
    pub fn get_operation_sender(&self) -> Option<Sender<Operation>> {
        if self.enable_graph_receiver {
            self.sender()
        } else {
            None
        }
    }

    /// Returns a error sender from the guest's world
    ///
    pub fn get_error_sender(&self) -> Option<Sender<ErrorContext>> {
        if self.enable_graph_receiver {
            self.sender()
        } else {
            None
        }
    }

    /// Visits the guest world's graph receiver
    ///
    pub fn visit_graph_receiver(&mut self, visitor: impl FnOnce(&mut Receiver<AttributeGraph>)) {
        if self.enable_graph_receiver {
            self.visit(visitor);
        }
    }

    /// Visits the guest world's operation receiver
    ///
    pub fn visit_operation_receiver(&mut self, visitor: impl FnOnce(&mut Receiver<Operation>)) {
        if self.enable_operation_receiver {
            self.visit(visitor);
        }
    }

    /// Visits the guest world's error context receiver
    ///
    pub fn visit_error_context_receiver(
        &mut self,
        visitor: impl FnOnce(&mut Receiver<ErrorContext>),
    ) {
        if self.enable_error_context_receiver {
            self.visit(visitor);
        }
    }

    /// Returns the sender for T
    ///
    fn sender<T>(&self) -> Option<T>
    where
        T: Resource + Clone,
    {
        let sender = self.world().read_resource::<T>();
        let sender = sender.deref();
        Some(sender.clone())
    }

    /// Visits the receiver of T
    ///
    fn visit<T>(&mut self, visitor: impl FnOnce(&mut T))
    where
        T: Resource,
    {
        let mut rx = self.world().write_resource::<T>();
        let rx = rx.deref_mut();
        visitor(rx);
    }
}

// impl<'a> System<'a> for GuestRuntime {
//     type SystemData = ();

//     fn run(&mut self, _: Self::SystemData) {
//         let GuestRuntime {
//             transport,
//             world,
//             enable_graph_receiver,
//             enable_operation_receiver,
//             enable_error_context_receiver,
//         } = self;

//         if *enable_graph_receiver {
//             world
//                 .system_data::<TransportReceiver>()
//                 .receive_graph(transport);
//         }

//         if *enable_operation_receiver {
//             world
//                 .system_data::<TransportReceiver>()
//                 .receive_operation(transport);
//         }

//         if *enable_error_context_receiver {
//             world
//                 .system_data::<TransportReceiver>()
//                 .receive_error_context(transport);
//         }
//     }
// }

// impl Transport for GuestRuntime {
//     fn transport_graph(&mut self, graph: AttributeGraph) {
//         if let Some(tx) = self.get_graph_sender() {
//             match tx.try_send(graph) {
//                 Ok(_) => event!(Level::TRACE, "guest runtime transported graph"),
//                 Err(err) => event!(Level::ERROR, "could not send graph, {err}"),
//             }
//         }
//     }

//     fn transport_operation(&mut self, operation: Operation) {
//         if let Some(tx) = self.get_operation_sender() {
//             match tx.try_send(operation) {
//                 Ok(_) => event!(Level::TRACE, "guest runtime transported operation"),
//                 Err(err) => event!(Level::ERROR, "could not send operation, {err}"),
//             }
//         }
//     }

//     fn transport_error_context(&mut self, error_context: ErrorContext) {
//         if let Some(tx) = self.get_error_sender() {
//             match tx.try_send(error_context) {
//                 Ok(_) => event!(Level::TRACE, "guest runtime transported error_context"),
//                 Err(err) => event!(Level::ERROR, "could not send error_context, {err}"),
//             }
//         }
//     }

//     fn proxy(&mut self) -> ProxyTransport {
//         self.transport.clone()
//     }
// }
