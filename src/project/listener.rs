use crate::prelude::*;

/// Event handler trait for messages brokered from the runtime,
///
pub trait Listener
where
    Self: Send + Sync + 'static
{
    /// Returns a new listener,
    ///
    fn create(world: &World) -> Self;

    /// Called when a runmd file is received,
    ///
    fn on_runmd(&mut self, runmd: &RunmdFile);

    /// Called when a status update is received,
    ///
    fn on_status_update(&mut self, status_update: &StatusUpdate);

    /// Called when a completed operation is received,
    ///
    fn on_operation(&mut self, operation: &Operation);

    /// Called when an error context is received,
    ///
    fn on_error_context(&mut self, error: &ErrorContext);

    /// Called when a plugin completes,
    ///
    fn on_completed_event(&mut self, entity: &Entity);

    /// Called when a start command is received,
    ///
    fn on_start_command(&mut self, start_command: &Start);
}

/// Enumeration of message types that can be listened to,
///
pub enum Messages {
    RunmdFile(RunmdFile),
    StatusUpdate(StatusUpdate),
    Operation(Operation),
    StartCommand(Start),
    ErrorContext(ErrorContext),
    CompletedPlugin(Entity),
}