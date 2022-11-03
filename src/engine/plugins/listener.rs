use specs::prelude::*;
use tokio::sync::mpsc::Receiver;

use crate::{prelude::*, guest::Guest, engine::Yielding};

/// Resources for consuming messages from plugins,
///
/// Can only be a single consumer per world,
///
#[derive(SystemData)]
pub struct PluginListener<'a> { 
    pub status_updates: Write<'a, Receiver<StatusUpdate>, EventRuntime>,
    pub runmd_files: Write<'a, Receiver<RunmdFile>, EventRuntime>,
    pub operations: Write<'a, Receiver<Operation>, EventRuntime>,
    pub starts: Write<'a, Receiver<Start>, EventRuntime>,
    pub guests: Write<'a, Receiver<Guest>, EventRuntime>,
    pub nodes: Write<'a, Receiver<(NodeCommand, Option<Yielding>)>, EventRuntime>,
    pub completed_entities: Write<'a, tokio::sync::broadcast::Receiver<Entity>, EventRuntime>,
    pub errors: Write<'a, tokio::sync::mpsc::Receiver<ErrorContext>, EventRuntime>,
}

impl<'a> PluginListener<'a> {
    /// Waits for the next status update,
    /// 
    pub async fn next_status_update(&mut self) -> Option<StatusUpdate> {
        let PluginListener { status_updates, .. } = self;

        status_updates.recv().await
    }


    /// Waits for the next runmd file,
    /// 
    pub async fn next_runmd_file(&mut self) -> Option<RunmdFile> {
        let PluginListener { runmd_files, .. } = self;

        runmd_files.recv().await
    }

    /// Waits for the next operation,
    /// 
    pub async fn next_operation(&mut self) -> Option<Operation> {
        let PluginListener { operations, .. } = self;

        operations.recv().await
    }

    /// Waits for the next start command,
    /// 
    pub async fn next_start_command(&mut self) -> Option<Start> {
        let PluginListener { starts, .. } = self;

        starts.recv().await
    }

    /// Waits for the next guest,
    /// 
    pub async fn next_guest(&mut self) -> Option<Guest> {
        let PluginListener { guests, .. } = self;

        guests.recv().await
    }

    /// Waits for the next node,
    /// 
    pub async fn next_node_command(&mut self) -> Option<(NodeCommand, Option<Yielding>)> {
        let PluginListener { nodes, .. } = self;

        nodes.recv().await
    }

    /// Tries to wait for the next status update,
    /// 
    pub fn try_next_status_update(&mut self) -> Option<StatusUpdate> {
        let PluginListener { status_updates, .. } = self;

        status_updates.try_recv().ok()
    }


    /// Tries to wait for the next runmd file,
    /// 
    pub fn try_next_runmd_file(&mut self) -> Option<RunmdFile> {
        let PluginListener { runmd_files, .. } = self;

        runmd_files.try_recv().ok()
    }

    /// Tries to wait for the next operation,
    /// 
    pub fn try_next_operation(&mut self) -> Option<Operation> {
        let PluginListener { operations, .. } = self;

        operations.try_recv().ok()
    }

    /// Tries to wait for the next start command,
    /// 
    pub fn try_next_start_command(&mut self) -> Option<Start> {
        let PluginListener { starts, .. } = self;

        starts.try_recv().ok()
    }

    /// Tries to wait for the next guest,
    /// 
    pub fn try_next_guest(&mut self) -> Option<Guest> {
        let PluginListener { guests, .. } = self;

        guests.try_recv().ok()
    }

    /// Tries to wait for the next node command,
    /// 
    pub fn try_next_node_command(&mut self) -> Option<(NodeCommand, Option<Yielding>)> {
        let PluginListener { nodes, .. } = self;

        nodes.try_recv().ok()
    }
}
