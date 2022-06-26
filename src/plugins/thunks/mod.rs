use std::future::Future;

use crate::AttributeGraph;
use atlier::system::Attribute;
use imgui::Ui;
use specs::Component;
use specs::{storage::DenseVecStorage, Entity};

mod println;
pub use println::Println;

mod write_files;
use tokio::{runtime::Handle, sync::mpsc::Sender, task::JoinHandle};
pub use write_files::WriteFiles;

pub mod demo {
    use super::write_files::demo;
    pub use demo::WriteFilesDemo;
}

use super::{BlockContext, Plugin};

/// Thunk is a function that can be passed around for the system to call later
#[derive(Component, Clone)]
#[storage(DenseVecStorage)]
pub struct Thunk(
    pub &'static str,
    pub fn(&mut ThunkContext) -> Option<JoinHandle<ThunkContext>>,
);

impl Thunk {
    pub fn from_plugin<P>() -> Self
    where
        P: Plugin<ThunkContext>,
    {
        Self(P::symbol(), P::call_with_context)
    }

    pub fn show(&self, context: &mut ThunkContext, ui: &Ui) {
        ui.set_next_item_width(130.0);
        if ui.button(context.label(self.0)) {
            let Thunk(.., thunk) = self;
            thunk(context);
        }
    }
}

/// StatusUpdate for stuff like progress bars
pub type StatusUpdate = (Entity, f32, String);

/// ThunkContext provides common methods for updating the underlying state graph,
/// in the context of a thunk. If async is enabled, then the context will have a handle to the tokio runtime.
#[derive(Component, Default, Clone)]
#[storage(DenseVecStorage)]
pub struct ThunkContext {
    pub block: BlockContext,
    pub entity: Option<Entity>,
    pub handle: Option<Handle>,
    pub status_updates: Option<Sender<StatusUpdate>>,
}

impl ThunkContext {
    /// If async is enabled on the thunk context, this will spawn the task
    /// otherwise, this call will result in a no-op
    pub fn task<F>(
        &self,
        task: impl FnOnce() -> F,
    ) -> Option<JoinHandle<ThunkContext>>
    where
        F: Future<Output = Option<ThunkContext>> + Send + 'static,
    {
        if let Self {
            handle: Some(handle),
            ..
        } = self
        {
            let default_return = self.clone();
            let future = (task)();
            
            Some(handle.spawn(async {
                if let Some(next) = future.await {
                    next
                } else {
                    default_return
                }
            }))
        } else {
            None
        }
    }
}

impl From<AttributeGraph> for ThunkContext {
    fn from(g: AttributeGraph) -> Self {
        Self {
            block: BlockContext::from(g),
            entity: None,
            handle: None,
            status_updates: None,
        }
    }
}

impl AsRef<AttributeGraph> for ThunkContext {
    fn as_ref(&self) -> &AttributeGraph {
        self.block.as_ref()
    }
}

impl AsMut<AttributeGraph> for ThunkContext {
    fn as_mut(&mut self) -> &mut AttributeGraph {
        self.block.as_mut()
    }
}

impl ThunkContext {
    /// optionally, update progress of the thunk execution
    pub async fn update_progress(&self, status: impl AsRef<str>, progress: f32) {
        if let ThunkContext {
            status_updates: Some(status_updates),
            entity: Some(entity),
            ..
        } = self
        {
            match status_updates
                .send((*entity, progress, status.as_ref().to_string()))
                .await
            {
                Ok(_) => {}
                Err(_) => {}
            }
        }
    }

    /// enable async features for the context
    pub fn enable_async(
        &self,
        entity: Entity,
        handle: Handle,
        status_updates: Option<Sender<StatusUpdate>>,
    ) -> ThunkContext {
        let mut async_enabled = self.clone();
        async_enabled.entity = Some(entity);
        async_enabled.handle = Some(handle);
        async_enabled.status_updates = status_updates;
        async_enabled
    }

    /// returns a handle to a tokio runtime
    pub fn handle(&self) -> Option<Handle> {
        self.handle.as_ref().and_then(|h| Some(h.clone()))
    }

    /// returns the thunk_symbol that was called with this context
    pub fn symbol(&self) -> Option<String> {
        if let Some(thunk) = self.block.get_block("thunk") {
            thunk.find_text("thunk_symbol")
        } else {
            None
        }
    }

    /// Updates error block
    pub fn error(&mut self, record: impl FnOnce(&mut AttributeGraph)) {
        self.block.update_block("error", record);
    }

    /// Update publish block
    pub fn publish(&mut self, update: impl FnOnce(&mut AttributeGraph)) {
        self.block.update_block("publish", update);
    }

    /// Receives values from the accept block, and updates the destination block with the new values
    pub fn accept(&mut self, dest_block: impl AsRef<str>, accept: impl Fn(&Attribute) -> bool) {
        if let Some(accept_block) = self.block.get_block("accept") {
            for (name, value) in accept_block
                .iter_attributes()
                .filter(|a| accept(a))
                .map(|a| (a.name(), a.value()))
            {
                self.block.update_block(dest_block.as_ref(), |u| {
                    u.with(name, value.clone());
                });
            }
        }
    }

    /// Formats a label that is unique to this state
    pub fn label(&self, label: impl AsRef<str>) -> impl AsRef<str> {
        format!(
            "{} {:#2x}",
            label.as_ref(),
            self.as_ref().hash_code() as u16
        )
    }
}
