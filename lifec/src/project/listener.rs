use specs::shred::Resource;

use crate::{engine::Completion, prelude::*};

/// Event handler trait for messages brokered from the runtime,
///
pub trait Listener
where
    Self: Default + Resource + Send + Sync + 'static,
{
    /// Returns a new listener,
    ///
    fn create(world: &World) -> Self;

    /// Called when an operation is being dispatched to the listener to handle,
    ///
    fn on_operation(&mut self, operation: Operation);

    /// Called when a completion is received,
    ///
    fn on_completion(&mut self, completion: Completion);

    /// Called when a plugin completes,
    ///
    fn on_completed_event(&mut self, entity: &Entity);

    /// Called when a status update is received,
    ///
    fn on_status_update(&mut self, (entity, progress, msg): &StatusUpdate) {
        event!(Level::TRACE, "entity: {:?}, progress: {}, message: {}", entity, progress, msg);
    }

    /// Called when an error context is received,
    ///
    fn on_error_context(&mut self, error: &ErrorContext) {
        for err in error.errors() {
            event!(Level::ERROR, "Plugin error encountered, {err}");
        }
    }
}

/// Enabling listener enables dispatching node commands,
///
/// This implementation is so that Users aren't required to use a Listener in order to enable_listener on the host
///
impl Listener for () {
    fn create(_: &World) -> Self {
        ()
    }
    fn on_operation(&mut self, _: Operation) {}
    fn on_completion(&mut self, _: Completion) {}
    fn on_completed_event(&mut self, _: &Entity) {}
}
