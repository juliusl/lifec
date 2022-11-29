use specs::storage::HashMapStorage;
use specs::{Component, Entity};
use tracing::{event, Level};

use crate::prelude::*;

/// Component for handling errors
///
/// This component is generated by the event runtime
///
#[derive(Component, Debug, Default, Clone, Hash, Eq, PartialEq, PartialOrd)]
#[storage(HashMapStorage)]
pub struct ErrorContext(
    /// error block
    AttributeGraph,
    /// stopped entity
    Option<Entity>,
    /// fix entity
    Option<Entity>,
);

impl ErrorContext {
    /// Creates a new error context
    ///
    pub fn new(graph: AttributeGraph, stopped: Option<Entity>) -> Self {
        Self(graph, stopped, None)
    }

    /// Returns true if the processing for this entity should stop
    ///
    pub fn stop_on_error(&self) -> bool {
        self.stopped().is_some()
    }

    /// Looks for an error block symbol, and returns all text attributes
    ///
    pub fn errors(&self) -> Vec<(String, String)> {
        // self.0.get_block("error").unwrap_or_default().iter_attributes().filter_map(|a| {
        //     if a.name().starts_with("block_") {
        //         return None;
        //     }

        //     let name = a.name();
        //     match a.value() {
        //         atlier::system::Value::TextBuffer(error) => {
        //             Some((name.to_string(), error.to_string()))
        //         },
        //         _ => {
        //             None
        //         }
        //     }
        // }).collect()

        todo!()
    }

    /// Sets the entity that is able to fix errors in the error context
    ///
    pub fn set_fix_entity(&self, fixer: Entity) -> Self {
        Self(self.0.clone(), self.1.clone(), Some(fixer))
    }

    /// Set's the data from the previous attempt to fix this error context
    ///
    pub fn set_previous_attempt(&mut self, previous: AttributeGraph) {
        event!(Level::INFO, "previous {:#?}", previous);

        // self.0.add_block("previous_attempt", |c| {
        //     c.copy(&previous);
        // });
    }

    /// Returns the stopped event
    ///
    pub fn stopped(&self) -> Option<Entity> {
        self.1
    }

    /// Returns the entity that can attempt to fix the current error context
    ///
    pub fn fixer(&self) -> Option<Entity> {
        self.2
    }

    /// Returns data from the previous attempt to fix the error context
    ///
    pub fn previous_attempt(&self) -> Option<AttributeGraph> {
        // self.0.get_block("previous_attempt")

        todo!()
    }
}
