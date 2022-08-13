use specs::prelude::*;
use tokio::sync::mpsc::Receiver;

use crate::{plugins::{EventRuntime, ErrorContext, Engine}, 
    AttributeGraph, 
    Operation, 
};

mod proxy;
pub use proxy::ProxyTransport;

/// A transport is an abstraction over receiving objects that have 
/// been dispatched from a thunk_context.
/// 
/// By implementing this type, it can be used w/ a system in order to handle
/// dispatched runtime elements. 
/// 
/// # Background
/// 
/// The motivation behind this is to replace the implementation provided by 
/// RuntimeEditor, as well as to allow for the interop of a 
/// variety of storage and network stack implementations. 
/// 
/// 
pub trait Transport : Engine + Sized {
    /// Transports a received graph 
    /// 
    fn transport_graph(&mut self, graph: AttributeGraph);
    
    /// Transports a received operation
    /// 
    fn transport_operation(&mut self, operation: Operation);

    /// Transports a received error context
    /// 
    fn transport_error_context(&mut self, error_context: ErrorContext);
}

/// System data type for systems enabling transporting runtime elements
/// 
#[derive(SystemData)]
pub struct TransportReceiver<'a> {
    /// Mpsc receiver for attribute graphs
    /// 
    pub graph_receiver: Write<'a, Receiver<AttributeGraph>, EventRuntime>,
    /// Mpsc receiver for operations
    /// 
    pub operation_receiver: Write<'a, Receiver<Operation>, EventRuntime>,
    /// Mpsc receiver for error contexts
    /// 
    pub error_context_receiver: Write<'a, Receiver<ErrorContext>, EventRuntime>,
}

impl<'a> TransportReceiver<'a> {
    /// Tries to receive graph to send to a transport
    /// 
    pub fn receive_graph(&mut self, transport: &mut impl Transport) {
        match self.graph_receiver.try_recv().ok() {
            Some(graph) => {
                transport.transport_graph(graph);
            },
            None => {
            },
        }
    }

    /// Tries to receive an operation to send to a transport
    /// 
    pub fn receive_operation(&mut self, transport: &mut impl Transport) {
        match self.operation_receiver.try_recv().ok() {
            Some(operation) => {
                transport.transport_operation(operation);
            },
            None => {
            },
        }
    }

    /// Tries to receive an error context to send to a transport
    /// 
    pub fn receive_error_context(&mut self, transport: &mut impl Transport) {
        match self.error_context_receiver.try_recv().ok() {
            Some(error_context) => {
                transport.transport_error_context(error_context);
            },
            None => {
            },
        }
    }
}
