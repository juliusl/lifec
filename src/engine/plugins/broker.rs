use std::ops::Deref;

use specs::{prelude::*, Read};
use tokio::sync::mpsc::{error::SendError, error::TrySendError, Sender};

use crate::{prelude::*, guest::Guest, engine::{Yielding, Completion}};

/// Resources for sending messages from plugins,
///
#[derive(SystemData)]
pub struct Broker<'a> {
    completion_sender: Read<'a, Sender<Completion>, EventRuntime>,
    status_sender: Read<'a, Sender<StatusUpdate>, EventRuntime>,
    operation_sender: Read<'a, Sender<Operation>, EventRuntime>,
    guest_sender: Read<'a, Sender<Guest>, EventRuntime>,
    node_sender: Read<'a, Sender<(NodeCommand, Option<Yielding>)>, EventRuntime>,
}

impl<'a> Broker<'a> {
    /// Returns a new command dispatcher,
    /// 
    pub fn command_dispatcher(&self) -> Sender<(NodeCommand, Option<Yielding>)> {
        self.node_sender.deref().clone()
    }

    /// Enables message senders on a thunk context,
    ///
    pub fn enable(&self, context: &mut ThunkContext) {
        let Broker { status_sender, operation_sender, guest_sender, node_sender, completion_sender } = self;

        context
            .enable_operation_dispatcher(operation_sender.deref().clone())
            .enable_status_updates(status_sender.deref().clone())
            .enable_guest_dispatcher(guest_sender.deref().clone())
            .enable_completion_dispatcher(completion_sender.deref().clone())
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

    /// Sends an operation,
    ///
    pub async fn send_operation(&self, operation: Operation) -> Result<(), SendError<Operation>> {
        let Broker { operation_sender, .. } = self;

        operation_sender.send(operation).await
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

    /// Sends an operation,
    ///
    pub fn try_send_operation(&self, operation: Operation) -> Result<(), TrySendError<Operation>> {
        let Broker { operation_sender, .. } = self;

        operation_sender.try_send(operation)
    }

    /// Sends a node command,
    /// 
    pub fn try_send_node_command(&self, node_command: NodeCommand, yielding: Option<Yielding>) -> Result<(), TrySendError<(NodeCommand, Option<Yielding>)>> {
        let Broker { node_sender, .. } = self; 

        node_sender.try_send((node_command, yielding))
    }
}
