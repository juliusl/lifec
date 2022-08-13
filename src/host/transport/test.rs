use specs::System;
use tokio::sync::mpsc::Receiver;
use tracing::{event, Level};

use crate::{AttributeGraph, Operation, plugins::{ErrorContext, Engine}};

use super::{ProxyTransport, Transport};

#[derive(Default)]
pub struct TestTransport {
    rx_graphs: Option<Receiver<AttributeGraph>>,
    rx_operations: Option<Receiver<Operation>>,
    rx_error_contexts: Option<Receiver<ErrorContext>>,
    on_graph: Option<fn(AttributeGraph)>,
    on_operation: Option<fn(Operation)>,
    on_error_context: Option<fn(ErrorContext)>,
}

impl TestTransport {
    pub fn new() -> (Self, ProxyTransport) {
        let mut test_transport = TestTransport::default();
        let mut p = ProxyTransport::new();
        test_transport.rx_graphs = Some(p.enable_graph_proxy(10));
        test_transport.rx_operations = Some(p.enable_operation_proxy(10));
        test_transport.rx_error_contexts = Some(p.enable_error_proxy(10));
        (test_transport, p)
    }

    pub fn add_graph_handler(&mut self, handler: fn(AttributeGraph)) {
        self.on_graph = Some(handler);
    }

    pub fn add_operation_handler(&mut self, handler: fn(Operation)) {
        self.on_operation = Some(handler);
    }

    pub fn add_error_handler(&mut self, handler: fn(ErrorContext)) {
        self.on_error_context = Some(handler);
    }
}

impl Engine for TestTransport {
    fn event_symbol() -> &'static str {
        "test_transport"
    }
}

impl Transport for TestTransport {
    fn transport_graph(
        &mut self,
        _graph: crate::AttributeGraph,
    ) {
        if let Some(h) = self.on_graph {
            h(_graph);
        }
    }

    fn transport_operation(
        &mut self,
        operation: Operation,
    ) {
        if let Some(h) = self.on_operation {
            h(operation);
        }
    }

    fn transport_error_context(
        &mut self,
        error_context: crate::plugins::ErrorContext,
    ) {
        event!(Level::TRACE, "transport error context called");
        if let Some(h) = self.on_error_context {
            h(error_context);
        }
    }
}

impl<'a> System<'a> for TestTransport {
    type SystemData = ();

    fn run(&mut self, _: Self::SystemData) {
        if let Some(grx) = self.rx_graphs.as_mut() {
            match grx.try_recv() {
                Ok(g) => {
                    self.transport_graph(g);
                },
                Err(_) => {
                    
                },
            }
        }

        if let Some(grx) = self.rx_operations.as_mut() {
            match grx.try_recv() {
                Ok(g) => {
                    self.transport_operation(g);
                },
                Err(_) => {
                    
                },
            }
        }

        if let Some(grx) = self.rx_error_contexts.as_mut() {
            match grx.try_recv() {
                Ok(g) => {
                    self.transport_error_context(g);
                },
                Err(_) => {
                    
                },
            }
        }
    }
}