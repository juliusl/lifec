use specs::prelude::*;
use tokio::sync::mpsc::Receiver;

use crate::prelude::*;

/// Resources for consuming messages from plugins,
///
/// Can only be a single consumer per world,
///
#[derive(SystemData)]
pub struct PluginListener<'a>(
    Write<'a, Receiver<StatusUpdate>, EventRuntime>,
    Write<'a, Receiver<RunmdFile>, EventRuntime>,
    Write<'a, Receiver<Operation>, EventRuntime>,
    Write<'a, Receiver<Start>, EventRuntime>,
);

impl<'a> PluginListener<'a> {
    /// Waits for the next status update,
    /// 
    pub async fn next_status_update(&mut self) -> Option<StatusUpdate> {
        let PluginListener(status_updates, ..) = self;

        status_updates.recv().await
    }


    /// Waits for the next runmd file,
    /// 
    pub async fn next_runmd_file(&mut self) -> Option<RunmdFile> {
        let PluginListener(_, runmd_files, ..) = self;

        runmd_files.recv().await
    }

    /// Waits for the next operation,
    /// 
    pub async fn next_operation(&mut self) -> Option<Operation> {
        let PluginListener(.., operations, _) = self;

        operations.recv().await
    }

    /// Waits for the next start command,
    /// 
    pub async fn next_start_command(&mut self) -> Option<Start> {
        let PluginListener(.., starts) = self;

        starts.recv().await
    }

    /// Tries to wait for the next status update,
    /// 
    pub fn try_next_status_update(&mut self) -> Option<StatusUpdate> {
        let PluginListener(status_updates, ..) = self;

        status_updates.try_recv().ok()
    }


    /// Tries to wait for the next runmd file,
    /// 
    pub fn try_next_runmd_file(&mut self) -> Option<RunmdFile> {
        let PluginListener(_, runmd_files, ..) = self;

        runmd_files.try_recv().ok()
    }

    /// Tries to wait for the next operation,
    /// 
    pub fn try_next_operation(&mut self) -> Option<Operation> {
        let PluginListener(.., operations, _) = self;

        operations.try_recv().ok()
    }

    /// Tries to wait for the next start command,
    /// 
    pub fn try_next_start_command(&mut self) -> Option<Start> {
        let PluginListener(.., starts) = self;

        starts.try_recv().ok()
    }
}
