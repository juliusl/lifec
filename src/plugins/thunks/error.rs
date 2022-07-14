use specs::{Component, Entity};
use specs::storage::HashMapStorage;

use crate::AttributeGraph;
use crate::plugins::BlockContext;

/// Component for handling errors
#[derive(Component, Default, Clone)]
#[storage(HashMapStorage)]
pub struct ErrorContext(
    /// error block
    BlockContext, 
    /// stopped entity 
    Option<Entity>,
    /// fix entity 
    Option<Entity>,
); 

impl ErrorContext {
    pub fn new(error_block: BlockContext, stopped: Option<Entity>) -> Self {
        Self(error_block, stopped, None)
    }

    /// Returns true if the processing for this entity should stop
    pub fn stop_on_error(&self) -> bool {
        self.0.get_block("error")
            .and_then(|b| b.as_ref().is_enabled("stop_on_error"))
            .unwrap_or_default()
    }

    pub fn errors(&self) -> Vec<(String, String)> {
        self.0.get_block("error").unwrap_or_default().iter_attributes().filter_map(|a| {
            if a.name().starts_with("block_") {
                return None;
            }

            let name = a.name();
            match a.value() {
                atlier::system::Value::TextBuffer(error) => {
                    Some((name.to_string(), error.to_string()))
                },
                _ => {
                    None 
                }
            }
        }).collect()
    }

    pub fn set_fix_entity(&self, fixer: Entity) -> Self {
        Self(self.0.clone(), self.1.clone(), Some(fixer))
    }

    pub fn set_previous_attempt(&mut self, previous: AttributeGraph) {

        eprintln!("previous {:#?}", previous);

        self.0.add_block("previous_attempt", |c| {
            c.copy(&previous);
        });
    }

    pub fn stopped(&self) -> Option<Entity> {
        self.1
    }

    pub fn fixer(&self) -> Option<Entity> {
        self.2
    }

    pub fn previous_attempt(&self) -> Option<AttributeGraph> {
        self.0.get_block("previous_attempt")
    }
}
