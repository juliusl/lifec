use specs::Component;
use specs::storage::HashMapStorage;

use crate::AttributeGraph;
use crate::plugins::BlockContext;

/// Component for handling errors
#[derive(Component, Default, Clone)]
#[storage(HashMapStorage)]
pub struct ErrorContext(BlockContext); 

impl ErrorContext {
    /// Returns true if the processing for this entity should stop
    pub fn stop_on_error(&self) -> bool {
        self.0.get_block("error")
            .and_then(|b| b.as_ref().is_enabled("stop_on_error"))
            .unwrap_or_default()
    }
}

impl From<AttributeGraph> for ErrorContext {
    fn from(graph: AttributeGraph) -> Self {
        Self(BlockContext::from(graph))
    }
}