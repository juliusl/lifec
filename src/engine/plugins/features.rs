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
    broker: PluginBroker<'a>,
    listener: PluginListener<'a>,
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

        self.listener.enable(&mut context);

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
}