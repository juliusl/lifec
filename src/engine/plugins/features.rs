use std::ops::Deref;
use specs::{prelude::*, SystemData};

use crate::prelude::*;

/// System data with plugin feature resources,
///
#[derive(SystemData)]
pub struct Features<'a> {
    workspace: Read<'a, Option<Workspace>>,
    tokio_runtime: Read<'a, tokio::runtime::Runtime, EventRuntime>,
    secure_client: Read<'a, SecureClient, EventRuntime>,
    host_editor: Read<'a, tokio::sync::watch::Receiver<HostEditor>, EventRuntime>,
    broker: PluginBroker<'a>,
}

impl<'a> Features<'a> {
    /// Enables features on a thunk context,
    ///
    pub fn enable(&self, entity: Entity, context: &ThunkContext) -> ThunkContext {
        let Features { workspace, tokio_runtime, secure_client, broker, .. } = self;

        let mut context = context.enable_async(entity, tokio_runtime.handle().clone());

        context.enable_https_client(secure_client.deref().clone());

        broker.enable(&mut context);

        if let Some(workspace) = workspace.as_ref() {
            context.enable_workspace(workspace.clone());
        }

        context.enable_host_editor_watcher(self.host_editor.deref().clone());
        context
    }

    /// Returns a tokio runtime handle,
    ///
    pub fn handle(&self) -> Handle {
        self.tokio_runtime.handle().clone()
    }

    /// Returns a broker,
    /// 
    pub fn broker(&self) -> &PluginBroker<'a> {
        let Features { broker, .. } = self; 

        broker
    }

    /// Returns a clone of the host editor if there was a change,
    /// 
    pub fn try_next_host_editor(&self) -> Option<HostEditor> {
        let Features { host_editor, .. } = self; 

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
}