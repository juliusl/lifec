use std::ops::Deref;

use specs::{prelude::*, Read};
use tokio::sync::mpsc::{error::SendError, error::TrySendError, Sender};

use crate::{prelude::*, guest::Guest};

/// Resources for sending messages from plugins,
///
#[derive(SystemData)]
pub struct Broker<'a> {
    status_sender: Read<'a, Sender<StatusUpdate>, EventRuntime>,
    runmd_sender: Read<'a, Sender<RunmdFile>, EventRuntime>,
    operation_sender: Read<'a, Sender<Operation>, EventRuntime>,
    start_sender: Read<'a, Sender<Start>, EventRuntime>,
    guest_sender: Read<'a, Sender<Guest>, EventRuntime>,
    node_sender: Read<'a, Sender<NodeCommand>, EventRuntime>,
}

impl<'a> Broker<'a> {
    /// Enables message senders on a thunk context,
    ///
    pub fn enable(&self, context: &mut ThunkContext) {
        let Broker { status_sender, runmd_sender, operation_sender, start_sender, guest_sender, node_sender } = self;

        context
            .enable_dispatcher(runmd_sender.deref().clone())
            .enable_operation_dispatcher(operation_sender.deref().clone())
            .enable_status_updates(status_sender.deref().clone())
            .enable_start_command_dispatcher(start_sender.deref().clone())
            .enable_guest_dispatcher(guest_sender.deref().clone())
            .enable_node_dispatcher(node_sender.deref().clone());
    }

    /// Sends a status update,
    ///
    pub async fn send_status_update(
        &self,
        status_update: StatusUpdate,
    ) -> Result<(), SendError<StatusUpdate>> {
        let Broker { status_sender, .. } = self;

        status_sender.send(status_update).await
    }

    /// Sends a runmd file,
    ///
    pub async fn send_runmd_file(&self, runmd: RunmdFile) -> Result<(), SendError<RunmdFile>> {
        let Broker { runmd_sender, .. } = self;

        runmd_sender.send(runmd).await
    }

    /// Sends an operation,
    ///
    pub async fn send_operation(&self, operation: Operation) -> Result<(), SendError<Operation>> {
        let Broker { operation_sender, .. } = self;

        operation_sender.send(operation).await
    }

    /// Sends a start command,
    ///
    pub async fn send_start(&self, start: Start) -> Result<(), SendError<Start>> {
        let Broker { start_sender, .. } = self;

        start_sender.send(start).await
    }

    /// Sends a status update,
    ///
    pub fn try_send_status_update(
        &self,
        status_update: StatusUpdate,
    ) -> Result<(), TrySendError<StatusUpdate>> {
        let Broker {  status_sender, .. } = self;

        status_sender.try_send(status_update)
    }

    /// Sends a runmd file,
    ///
    pub fn try_send_runmd_file(&self, runmd: RunmdFile) -> Result<(), TrySendError<RunmdFile>> {
        let Broker { runmd_sender, .. } = self;

        runmd_sender.try_send(runmd)
    }

    /// Sends an operation,
    ///
    pub fn try_send_operation(&self, operation: Operation) -> Result<(), TrySendError<Operation>> {
        let Broker { operation_sender, .. } = self;

        operation_sender.try_send(operation)
    }

    /// Sends a start command,
    ///
    pub fn try_send_start(&self, start: Start) -> Result<(), TrySendError<Start>> {
        let Broker { start_sender, .. }  = self;

        start_sender.try_send(start)
    }

    /// Sends a node command,
    /// 
    pub fn try_send_node_command(&self, node_command: NodeCommand) -> Result<(), TrySendError<NodeCommand>> {
        let Broker { node_sender, .. } = self; 

        node_sender.try_send(node_command)
    }
}
