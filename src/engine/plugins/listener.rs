use std::ops::Deref;

use specs::prelude::*;
use tokio::sync::mpsc::Receiver;

use crate::{prelude::*, guest::Guest};

/// Resources for consuming messages from plugins,
///
/// Can only be a single consumer per world,
///
#[derive(SystemData)]
pub struct PluginListener<'a> { 
    status_updates: Write<'a, Receiver<StatusUpdate>, EventRuntime>,
    runmd_files: Write<'a, Receiver<RunmdFile>, EventRuntime>,
    operations: Write<'a, Receiver<Operation>, EventRuntime>,
    starts: Write<'a, Receiver<Start>, EventRuntime>,
    guests: Write<'a, Receiver<Guest>, EventRuntime>,
    host_editor: Write<'a, tokio::sync::watch::Receiver<HostEditor>, EventRuntime>,
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

    /// Waits for the host editor to change and returns a clone,
    /// 
    pub async fn next_host_editor(&mut self) -> Option<HostEditor> {
        match self.host_editor.changed().await {
            Ok(_) => Some(self.host_editor.borrow().clone()),
            Err(err) => {
                event!(Level::ERROR, "Error waiting for a host change {err}");
                None
            },
        }
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

    /// Returns a clone of the host editor if there was a change,
    /// 
    pub fn try_next_host_editor(&self) -> Option<HostEditor> {
        let PluginListener { host_editor, .. } = self; 

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
        let channel = self.host_editor.deref();
        channel.borrow().clone()
    }

    /// Enables features on the thunk context,
    /// 
    pub fn enable(&self, context: &mut ThunkContext) {
        context.enable_host_editor_watcher(self.host_editor.deref().clone());
    }
}
