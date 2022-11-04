use specs::shred::Resource;

use crate::prelude::*;

/// Event handler trait for messages brokered from the runtime,
///
pub trait Listener
where
    Self: Default + Resource + Send + Sync + 'static
{
    /// Returns a new listener,
    ///
    fn create(world: &World) -> Self;

    /// Called when a status update is received,
    ///
    fn on_status_update(&mut self, status_update: &StatusUpdate);

    /// Called when a completed operation is received,
    ///
    fn on_operation(&mut self, operation: Operation);

    /// Called when an error context is received,
    ///
    fn on_error_context(&mut self, error: &ErrorContext);

    /// Called when a plugin completes,
    ///
    fn on_completed_event(&mut self, entity: &Entity);
}
