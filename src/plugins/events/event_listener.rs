use crate::{plugins::{StatusUpdate, ErrorContext}, Start, project::RunmdFile};
use tokio::sync::mpsc::Receiver;
use tokio::sync::broadcast::Receiver as BroadcastReceiver;
use specs::prelude::*;
use crate::Operation;

use super::EventRuntime;

/// System data for receving messages from plugins
/// 
#[derive(SystemData)]
pub struct EventListener<'a> {
    pub status_updates: Write<'a, Receiver<StatusUpdate>, EventRuntime>,
    pub runmd: Write<'a, Receiver<RunmdFile>, EventRuntime>,
    pub operations: Write<'a, Receiver<Operation>, EventRuntime>,
    pub error_contexts: Write<'a, Receiver<ErrorContext>, EventRuntime>,
    pub start_commands: Write<'a, Receiver<Start>, EventRuntime>,
    pub completed_plugins: Write<'a, BroadcastReceiver<Entity>, EventRuntime>,
}

