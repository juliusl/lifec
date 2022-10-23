use std::ops::Deref;
use specs::{prelude::*, SystemData};

use crate::prelude::*;

/// System data with plugin feature resources,
///
#[derive(SystemData)]
pub struct Features<'a>(
    Read<'a, Option<Workspace>>,
    Read<'a, tokio::runtime::Runtime, EventRuntime>,
    Read<'a, SecureClient, EventRuntime>,
    PluginBroker<'a>,
);

impl<'a> Features<'a> {
    /// Enables features on a thunk context,
    ///
    pub fn enable(&self, entity: Entity, context: &ThunkContext) -> ThunkContext {
        let Features(workspace, runtime, client, sender) = self;

        let mut context = context.enable_async(entity, runtime.handle().clone());

        context.enable_https_client(client.deref().clone());

        sender.enable(&mut context);

        if let Some(workspace) = workspace.as_ref() {
            context.enable_workspace(workspace.clone());
        }

        context
    }

    /// Returns a tokio runtime handle,
    ///
    pub fn handle(&self) -> Handle {
        self.1.handle().clone()
    }
}