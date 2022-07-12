use std::future::Future;

use crate::Extension;
use crate::AttributeGraph;
use crate::RuntimeDispatcher;
use hyper::client::HttpConnector;
use imgui::Ui;
use specs::Component;
use specs::{storage::DenseVecStorage, Entity};

mod open_file;
pub use open_file::OpenFile;

mod open_dir;
pub use open_dir::OpenDir;

mod write_file;
use tokio::sync::oneshot;
pub use write_file::WriteFile;

mod timer;
pub use timer::Timer;

mod println;
pub use println::Println;

mod clear;
pub use clear::Clear;

use super::{BlockContext, Plugin, Project};
use tokio::{runtime::Handle, sync::mpsc::Sender, sync::oneshot::channel, task::JoinHandle};

/// Thunk is a function that can be passed around for the system to call later
#[derive(Component, Clone)]
#[storage(DenseVecStorage)]
pub struct Thunk(
    // thunk label
    pub &'static str,
    // thunk fn
    pub fn(&mut ThunkContext) -> Option<(JoinHandle<ThunkContext>, CancelToken)>,
);

/// Config for a thunk context
#[derive(Component, Clone)]
#[storage(DenseVecStorage)]
pub struct Config(
    /// config label
    pub &'static str,
    /// config fn
    pub fn(&mut ThunkContext),
);

impl AsRef<Config> for Config {
    fn as_ref(&self) -> &Config {
        self
    }
}

impl Thunk {
    /// Generates a thunk from a plugin impl
    pub fn from_plugin<P>() -> Self
    where
        P: Plugin<ThunkContext>,
    {
        Self(P::symbol(), P::call_with_context)
    }

    /// deprecated?
    pub fn show(&self, context: &mut ThunkContext, ui: &Ui) {
        ui.set_next_item_width(130.0);
        if ui.button(context.label(self.0)) {
            let Thunk(.., thunk) = self;
            thunk(context);
        }
    }
}

/// StatusUpdate for stuff like progress bars
pub type StatusUpdate = (
    // entity with an update
    Entity, 
    // progress
    f32, 
    // status message 
    String
);

/// Cancel token stored by the event runtime
pub type CancelToken = tokio::sync::oneshot::Sender<()>;

/// Cancel source stored by the thunk
pub type CancelSource = tokio::sync::oneshot::Receiver<()>;

/// Secure client for making http requests
pub type SecureClient = hyper::Client<hyper_tls::HttpsConnector<HttpConnector>>;

#[derive(Component)]
#[storage(DenseVecStorage)]
pub struct CancelThunk(
    // Oneshot channel that cancels the thunk
    pub CancelToken
);

impl From<CancelToken> for CancelThunk {
    fn from(token: CancelToken) -> Self {
        Self(token)
    }
}

/// ThunkContext provides common methods for updating the underlying state graph,
/// in the context of a thunk. If async is enabled, then the context will have a handle to the tokio runtime.
#[derive(Component, Default, Clone)]
#[storage(DenseVecStorage)]
pub struct ThunkContext {
    /// Underlying block context for this thunk
    pub block: BlockContext,
    /// Current project
    pub project: Option<Project>,
    /// Async fields
    /// Entity that is identifying the thunk
    pub entity: Option<Entity>,
    /// Tokio runtime handle, to spawn additional tasks 
    pub handle: Option<Handle>,
    /// Sender for status updates for the thunk
    status_updates: Option<Sender<StatusUpdate>>,
    /// Client for sending secure http requests
    client: Option<SecureClient>,
    /// Dispatcher for attribute graphs
    dispatcher: Option<Sender<AttributeGraph>>,
}

/// This block has all the async related features
impl ThunkContext {
    /// Returns true if the source has been cancelled.
    /// Note: In most cases you could just use tokio::select! macro with the source,
    /// but there are control flows where getting a boolean is more ergonomic.
    /// (Example: Timer uses this, while Process uses select!)
    pub fn is_cancelled(cancel_source: &mut oneshot::Receiver<()>) -> bool {
        match cancel_source.try_recv() {
            Ok(_) | Err(tokio::sync::oneshot::error::TryRecvError::Closed) => true,
            _ => false,
        }
    }

    /// enable async features for the context
    pub fn enable_async(
        &self,
        entity: Entity,
        handle: Handle,
        client: SecureClient,
        project: Option<Project>,
        status_updates: Option<Sender<StatusUpdate>>,
        dispatcher: Option<Sender<AttributeGraph>>,
    ) -> ThunkContext {
        let mut async_enabled = self.clone();
        async_enabled.entity = Some(entity);
        async_enabled.handle = Some(handle);
        async_enabled.client = Some(client);
        async_enabled.status_updates = status_updates;
        async_enabled.dispatcher = dispatcher;
        async_enabled.project = project;
        async_enabled
    }

    /// returns the secure client
    pub fn client(&self) -> Option<SecureClient> {
        self.client.clone()
    }

    /// returns a handle to a tokio runtime
    pub fn handle(&self) -> Option<Handle> {
        self.handle.as_ref().and_then(|h| Some(h.clone()))
    }

    /// dispatch runmd for a host to process
    pub async fn dispatch(&self, runmd: impl AsRef<str>) {
        if let Some(dispatcher) = &self.dispatcher {
            let graph = AttributeGraph::from(0);
            match graph.batch(runmd) {
                Ok(msg) => {
                    dispatcher.send(msg).await.ok();
                },
                Err(_) => todo!(),
            }
        }
    }

    /// If async is enabled on the thunk context, this will spawn the task
    /// otherwise, this call will result in a no-op
    pub fn task<F>(
        &self,
        task: impl FnOnce(CancelSource) -> F,
    ) -> Option<(JoinHandle<ThunkContext>, CancelToken)>
    where
        F: Future<Output = Option<ThunkContext>> + Send + 'static,
    {
        if let Self {
            handle: Some(handle),
            ..
        } = self
        {
            let default_return = self.clone();
            let (tx, cancel) = channel::<()>();

            let task = (task)(cancel);
            Some((
                handle.spawn(async {
                    match task.await {
                        Some(next) => next,
                        None => default_return,
                    }
                }),
                tx,
            ))
        } else {
            None
        }
    }

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

    /// optionally, update status of the thunk execution
    pub async fn update_status_only(&self, status: impl AsRef<str>) {
        self.update_progress(status, 0.0).await;
    }
}

impl From<AttributeGraph> for ThunkContext {
    fn from(g: AttributeGraph) -> Self {
        Self {
            block: BlockContext::from(g),
            project: None,
            entity: None,
            handle: None,
            client: None,
            status_updates: None,
            dispatcher: None,
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
    /// Updates error block
    pub fn error(&mut self, record: impl Fn(&mut AttributeGraph)) {
        if !self.block.update_block("error", &record) {
            self.block.add_block("error", record);
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

impl Extension for ThunkContext {
    /// table view to debug backend
    fn on_ui(&'_ mut self, _: &specs::World, ui: &'_ imgui::Ui<'_>) {
        if let Some(entity) = self.entity {
            for attr in self.as_mut().iter_mut_attributes() {
                if ui.table_next_column() {
                    ui.text(format!("{}", entity.id()));
                }

                if ui.table_next_column() {
                    ui.text(attr.name());
                }

                if ui.table_next_column() {
                    ui.text(format!("{}", attr.value())
                        .split_once("::Reference")
                        .and_then(|(a, _)| Some(a)).unwrap_or_default()
                    );
                }
    
                ui.table_next_row();
            }
        }
    }
}