use std::ops::Deref;

use specs::{prelude::*, Read};
use tokio::sync::mpsc::Sender;

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
    pub fn enable(&self, context: &mut ThunkContext) {
        let Broker(status_sender, runmd_sender, operation_sender, start_sender) = self; 
        
        context
        .enable_dispatcher(runmd_sender.deref().clone())
        .enable_operation_dispatcher(operation_sender.deref().clone())
        .enable_status_updates(status_sender.deref().clone())
        .enable_start_command_dispatcher(start_sender.deref().clone());
    }
}