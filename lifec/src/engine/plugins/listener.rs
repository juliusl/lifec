use specs::prelude::*;
use tokio::sync::mpsc::Receiver;

use crate::{prelude::*, guest::Guest, engine::{Yielding, Completion, NodeCommand}};

/// Resources for consuming messages from plugins,
///
/// Can only be a single consumer per world,
///
#[derive(SystemData)]
pub struct PluginListener<'a> { 
    status_updates: Write<'a, Receiver<StatusUpdate>, EventRuntime>,
    completions: Write<'a, Receiver<Completion>, EventRuntime>,
    operations: Write<'a, Receiver<Operation>, EventRuntime>,
    guests: Write<'a, Receiver<Guest>, EventRuntime>,
    nodes: Write<'a, Receiver<(NodeCommand, Option<Yielding>)>, EventRuntime>,
}

impl<'a> PluginListener<'a> {
    /// Waits for the next status update,
    /// 
    pub async fn next_status_update(&mut self) -> Option<StatusUpdate> {
        let PluginListener { status_updates, .. } = self;

        status_updates.recv().await
    }

    /// Waits for the next operation,
    /// 
    pub async fn next_operation(&mut self) -> Option<Operation> {
        let PluginListener { operations, .. } = self;

        operations.recv().await
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

    /// Waits for the next completion,
    /// 
    pub async fn next_completion(&mut self) -> Option<Completion> {
        let PluginListener { completions, .. } = self;

        completions.recv().await
    }

    /// Tries to wait for the next status update,
    /// 
    pub fn try_next_status_update(&mut self) -> Option<StatusUpdate> {
        let PluginListener { status_updates, .. } = self;

        status_updates.try_recv().ok()
    }

    /// Tries to wait for the next operation,
    /// 
    pub fn try_next_operation(&mut self) -> Option<Operation> {
        let PluginListener { operations, .. } = self;

        operations.try_recv().ok()
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

      /// Tries to wait for the next completion,
    /// 
    pub fn try_next_completion(&mut self) -> Option<Completion> {
        let PluginListener { completions, .. } = self;

        completions.try_recv().ok()
    }
}
