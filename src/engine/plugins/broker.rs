use std::ops::Deref;

use specs::{prelude::*, Read};
use tokio::sync::mpsc::{error::SendError, error::TrySendError, Sender};

use crate::prelude::*;

/// Resources for sending messages from plugins,
///
#[derive(SystemData)]
pub struct Broker<'a>(
    Read<'a, Sender<StatusUpdate>, EventRuntime>,
    Read<'a, Sender<RunmdFile>, EventRuntime>,
    Read<'a, Sender<Operation>, EventRuntime>,
    Read<'a, Sender<Start>, EventRuntime>,
);

impl<'a> Broker<'a> {
    /// Enables message senders on a thunk context,
    ///
    pub fn enable(&self, context: &mut ThunkContext) {
        let Broker(status_sender, runmd_sender, operation_sender, start_sender) = self;

        context
            .enable_dispatcher(runmd_sender.deref().clone())
            .enable_operation_dispatcher(operation_sender.deref().clone())
            .enable_status_updates(status_sender.deref().clone())
            .enable_start_command_dispatcher(start_sender.deref().clone());
    }

    /// Sends a status update,
    ///
    pub async fn send_status_update(
        &self,
        status_update: StatusUpdate,
    ) -> Result<(), SendError<StatusUpdate>> {
        let Broker(status_updates, ..) = self;

        status_updates.send(status_update).await
    }

    /// Sends a runmd file,
    ///
    pub async fn send_runmd_file(&self, runmd: RunmdFile) -> Result<(), SendError<RunmdFile>> {
        let Broker(_, runmd_files, ..) = self;

        runmd_files.send(runmd).await
    }

    /// Sends an operation,
    ///
    pub async fn send_operation(&self, operation: Operation) -> Result<(), SendError<Operation>> {
        let Broker(.., operations, _) = self;

        operations.send(operation).await
    }

    /// Sends a start command,
    ///
    pub async fn send_start(&self, start: Start) -> Result<(), SendError<Start>> {
        let Broker(.., starts) = self;

        starts.send(start).await
    }

    /// Sends a status update,
    ///
    pub fn try_send_status_update(
        &self,
        status_update: StatusUpdate,
    ) -> Result<(), TrySendError<StatusUpdate>> {
        let Broker(status_updates, ..) = self;

        status_updates.try_send(status_update)
    }

    /// Sends a runmd file,
    ///
    pub fn try_send_runmd_file(&self, runmd: RunmdFile) -> Result<(), TrySendError<RunmdFile>> {
        let Broker(_, runmd_files, ..) = self;

        runmd_files.try_send(runmd)
    }

    /// Sends an operation,
    ///
    pub fn try_send_operation(&self, operation: Operation) -> Result<(), TrySendError<Operation>> {
        let Broker(.., operations, _) = self;

        operations.try_send(operation)
    }

    /// Sends a start command,
    ///
    pub fn try_send_start(&self, start: Start) -> Result<(), TrySendError<Start>> {
        let Broker(.., starts) = self;

        starts.try_send(start)
    }
}
