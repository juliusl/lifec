use std::ops::Deref;

use specs::{prelude::*, Read};
use tokio::sync::mpsc::{error::SendError, error::TrySendError, Sender};

use crate::prelude::*;

/// Resources for sending messages from plugins,
///
#[derive(SystemData)]
pub struct Broker<'a> {
    status_sender: Read<'a, Sender<StatusUpdate>, EventRuntime>,
    runmd_sender: Read<'a, Sender<RunmdFile>, EventRuntime>,
    operation_sender: Read<'a, Sender<Operation>, EventRuntime>,
    start_sender: Read<'a, Sender<Start>, EventRuntime>,
    host_editor: Write<'a, tokio::sync::watch::Receiver<HostEditor>, EventRuntime>,
}

impl<'a> Broker<'a> {
    /// Enables message senders on a thunk context,
    ///
    pub fn enable(&self, context: &mut ThunkContext) {
        let Broker { status_sender, runmd_sender, operation_sender, start_sender, host_editor } = self;

        context
            .enable_dispatcher(runmd_sender.deref().clone())
            .enable_operation_dispatcher(operation_sender.deref().clone())
            .enable_status_updates(status_sender.deref().clone())
            .enable_start_command_dispatcher(start_sender.deref().clone())
            .enable_host_editor_watcher(host_editor.deref().clone());
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

    /// Waits for the host editor to change and returns a clone,
    /// 
    pub async fn host_editor_changed(&mut self) -> Option<HostEditor> {
        match self.host_editor.changed().await {
            Ok(_) => Some(self.host_editor.borrow().clone()),
            Err(err) => {
                event!(Level::ERROR, "Error waiting for a host change {err}");
                None
            },
        }
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

    /// Returns a clone of the host editor if there was a change,
    /// 
    pub fn try_receive_host_editor_change(&self) -> Option<HostEditor> {
        let Broker { host_editor, .. } = self; 

         match host_editor.has_changed() {
            Ok(changed) => {
                if changed {
                    Some(host_editor.borrow().clone())
                } else {
                    None 
                }
            },
            Err(err) => {
                event!(Level::ERROR, "Error checking for host editor change {err}");
                None
            },
        }
    }

    /// Returns the current host editor,
    /// 
    pub fn host_editor(&self) -> HostEditor {
        self.host_editor.borrow().clone()
    }
}
