use specs::prelude::*;
use tokio::sync::mpsc::Receiver;

use crate::prelude::*;

/// Resources for consuming messages from plugins,
///
/// Can only be a single consumer per world,
///
#[derive(SystemData)]
pub struct Listener<'a>(
    Write<'a, Receiver<StatusUpdate>, EventRuntime>,
    Write<'a, Receiver<RunmdFile>, EventRuntime>,
    Write<'a, Receiver<Operation>, EventRuntime>,
    Write<'a, Receiver<Start>, EventRuntime>,
);

impl<'a> Listener<'a> {
    /// Waits for the next status update,
    /// 
    pub async fn next_status_update(&mut self) -> Option<StatusUpdate> {
        let Listener(status_updates, ..) = self;

        status_updates.recv().await
    }


    /// Waits for the next runmd file,
    /// 
    pub async fn next_runmd_file(&mut self) -> Option<RunmdFile> {
        let Listener(_, runmd_files, ..) = self;

        runmd_files.recv().await
    }

    /// Waits for the next operation,
    /// 
    pub async fn next_operation(&mut self) -> Option<Operation> {
        let Listener(.., operations, _) = self;

        operations.recv().await
    }

    /// Waits for the next start command,
    /// 
    pub async fn next_start_command(&mut self) -> Option<Start> {
        let Listener(.., starts) = self;

        starts.recv().await
    }
}
