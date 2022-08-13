use tokio::sync::{mpsc::{Sender, Receiver, channel}};
use tracing::{event, Level};

use crate::{AttributeGraph, Operation, plugins::{ErrorContext, Engine}};

use super::Transport;

#[derive(Default)]
pub struct ProxyTransport {
    graphs: Option<Sender<AttributeGraph>>,
    operations: Option<Sender<Operation>>,
    error_contexts: Option<Sender<ErrorContext>>,
}

impl ProxyTransport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enable_graph_proxy(&mut self, capacity: usize) -> Receiver<AttributeGraph> {
        let (tx, rx) = channel(capacity);
        self.graphs = Some(tx);
        rx
    }

    pub fn enable_operation_proxy(&mut self, capacity: usize) -> Receiver<Operation> {
        let (tx, rx) = channel(capacity);
        self.operations = Some(tx);
        rx
    }

    pub fn enable_error_proxy(&mut self, capacity: usize) -> Receiver<ErrorContext> {
        let (tx, rx) = channel(capacity);
        self.error_contexts = Some(tx);
        rx
    }
}

impl Engine for ProxyTransport {
    fn event_symbol() -> &'static str {
        "proxy"
    }
}

impl Transport for ProxyTransport {
    fn transport_graph(&mut self, graph: AttributeGraph) {
        if let Some(graph_sender) = self.graphs.as_ref() {
            match graph_sender.try_send(graph) {
                Ok(_) => {
                    event!(Level::DEBUG, "proxy transport sent graph");
                },
                Err(_) => {
                    
                },
            }
        }
    }

    fn transport_operation(&mut self, operation: Operation) {
        if let Some(operation_sender) = self.operations.as_ref() {
            match operation_sender.try_send(operation) {
                Ok(_) => {
                    event!(Level::DEBUG, "proxy transport sent operation");
                },
                Err(_) => {
                    
                },
            }
        }
    }

    fn transport_error_context(&mut self, error_context: ErrorContext) {
        if let Some(error_context_sender) = self.error_contexts.as_ref() {
            match error_context_sender.try_send(error_context) {
                Ok(_) => {
                    event!(Level::DEBUG, "proxy transport sent error context");
                },
                Err(_) => {
                    
                },
            }
        }
    }
}