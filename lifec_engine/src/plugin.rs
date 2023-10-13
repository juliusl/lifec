use std::future::Future;
use std::marker::PhantomData;
use std::ops::DerefMut;

use reality::AsyncStorageTarget;
use reality::Attribute;
use reality::AttributeType;
use reality::BlockObject;
use reality::ResourceKey;
use reality::Shared;
use reality::StorageTarget;

use futures_util::FutureExt;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use anyhow::anyhow;

/// Type alias for the result of spawning a task,
///
pub type SpawnResult = Option<JoinHandle<anyhow::Result<ThunkContext>>>;

/// Enumeration of output a plugin can return,
///
pub enum PluginOutput {
    /// The plugin has spawned a task,
    ///
    /// If a join-handle was successfully created, then it will be polled to completion and the result will be passed to the next plugin.
    ///
    Spawn(SpawnResult),
    /// The plugin has decided to abort further execution,
    ///
    Abort(anyhow::Result<()>),
    ///
    ///
    Skip,
}

impl Future for PluginOutput {
    type Output = anyhow::Result<Option<ThunkContext>>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.deref_mut() {
            PluginOutput::Spawn(task) => match task {
                Some(handle) => match handle.poll_unpin(cx) {
                    std::task::Poll::Ready(output) => {
                        let context = output?.ok();
                        std::task::Poll::Ready(Ok(context))
                    }
                    std::task::Poll::Pending => {
                        cx.waker().wake_by_ref();
                        std::task::Poll::Pending
                    }
                },
                None => std::task::Poll::Ready(Ok(None)),
            },
            PluginOutput::Abort(result) => match result {
                Ok(_) => std::task::Poll::Ready(Ok(None)),
                Err(err) => std::task::Poll::Ready(Err(anyhow::anyhow!("{err}"))),
            },
            PluginOutput::Skip => std::task::Poll::Ready(Ok(None)),
        }
    }
}

impl From<SpawnResult> for PluginOutput {
    fn from(value: SpawnResult) -> Self {
        PluginOutput::Spawn(value)
    }
}

/// Struct containing shared context between plugins,
///
pub struct ThunkContext {
    /// Source storage mapping to this context,
    ///
    pub(crate) source: AsyncStorageTarget<Shared>,
    /// Attribute for this context,
    ///
    attribute: Option<ResourceKey<Attribute>>,
    /// Transient storage target,
    ///
    pub transient: AsyncStorageTarget<Shared>,
    /// Cancellation token that can be used by the engine to signal shutdown,
    ///
    pub cancellation: tokio_util::sync::CancellationToken,
}

impl From<AsyncStorageTarget<Shared>> for ThunkContext {
    fn from(value: AsyncStorageTarget<Shared>) -> Self {
        let handle = value.runtime.clone().expect("should have a runtime");
        Self {
            source: value,
            attribute: None,
            transient: Shared::default().into_thread_safe_with(handle),
            cancellation: CancellationToken::new(),
        }
    }
}

impl Clone for ThunkContext {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            attribute: self.attribute.clone(),
            transient: self.transient.clone(),
            cancellation: self.cancellation.clone(),
        }
    }
}

impl ThunkContext {
    /// Reset the transient storage,
    /// 
    pub fn reset(&mut self) {
        let handle = self.source.runtime.clone().expect("should have a runtime");
        self.transient = Shared::default().into_thread_safe_with(handle);
    }

    /// Calls the thunk fn related to this context,
    ///
    pub async fn call(&self) -> anyhow::Result<Option<ThunkContext>> {
        let storage = self.source.storage.read().await;
        let thunk = storage.resource::<ThunkFn>(self.attribute.map(|a| a.transmute()));

        if let Some(thunk) = thunk {
            thunk(self.clone()).await
        } else {
            Err(anyhow!("Did not execute thunk"))
        }
    }

    /// Sets the attribute for this context,
    ///
    pub fn set_attribute(&mut self, attribute: ResourceKey<Attribute>) {
        self.attribute = Some(attribute);
    }

    /// Get read access to source storage,
    ///
    pub async fn source(&self) -> tokio::sync::RwLockReadGuard<Shared> {
        self.source.storage.read().await
    }

    /// Get mutable access to source storage,
    ///
    /// **Note**: Marked unsafe because will mutate the source storage. Source storage is re-used on each execution.
    ///
    pub async unsafe fn source_mut(&self) -> tokio::sync::RwLockWriteGuard<Shared> {
        self.source.storage.write().await
    }

    /// Tries to get access to source storage,
    ///
    pub fn try_source(&self) -> Option<tokio::sync::RwLockReadGuard<Shared>> {
        self.source.storage.try_read().ok()
    }

    /// (unsafe) Tries to get mutable access to source storage,
    ///
    /// **Note**: Marked unsafe because will mutate the source storage. Source storage is re-used on each execution.
    ///
    pub unsafe fn try_source_mut(&mut self) -> Option<tokio::sync::RwLockWriteGuard<Shared>> {
        self.source.storage.try_write().ok()
    }

    /// Returns the transient storage target,
    ///
    /// **Note**: During an operation run dispatch queues are drained before each thunk execution.
    ///
    pub fn transient(&self) -> AsyncStorageTarget<Shared> {
        self.transient.clone()
    }

    /// Spawn a task w/ this context,
    ///
    /// Returns a join-handle if the task was created.
    ///
    pub fn spawn<F>(self, task: impl FnOnce(ThunkContext) -> F + 'static) -> SpawnResult
    where
        F: Future<Output = anyhow::Result<ThunkContext>> + Send + 'static,
    {
        self.source
            .runtime
            .clone()
            .as_ref()
            .map(|h| h.clone().spawn(task(self)))
    }

    /// Convenience for `PluginOutput::Skip`
    ///
    pub fn skip(&self) -> PluginOutput {
        PluginOutput::Skip
    }

    /// Convenience for `PluginOutput::Abort(..)`
    ///
    pub fn abort(&self, error: impl Into<anyhow::Error>) -> PluginOutput {
        PluginOutput::Abort(Err(error.into()))
    }

    /// Retrieves the initialized state of the plugin,
    ///
    /// **Note**: This is the state that was evaluated at the start of the application, when the runmd was parsed.
    ///
    pub async fn initialized<P: Plugin + Default + Clone + Sync + Send + 'static>(&self) -> P {
        self.source
            .storage
            .read()
            .await
            .resource::<P>(self.attribute.clone().map(|a| a.transmute()))
            .map(|r| r.clone())
            .unwrap_or_default()
    }
}

/// Allows users to export logic as a simple fn,
///
pub trait Plugin: BlockObject<Shared> {
    /// Called when an event executes,
    ///
    /// Returning PluginOutput determines the behavior of the Event.
    ///
    fn call(context: ThunkContext) -> PluginOutput;
}

/// Executes a plugin immediately,
///
pub async fn call_plugin<P: Plugin + Send + Sync>(
    tc: ThunkContext,
) -> anyhow::Result<ThunkContext> {
    match <P as Plugin>::call(tc) {
        PluginOutput::Spawn(Some(spawned)) => spawned.await?,
        _ => Err(anyhow::anyhow!("Could not spawn plugin call")),
    }
}

/// Trait for implementing call w/ an async trait,
///
/// **Note** This is a convenience if the additional Skip/Abort control-flow options
/// are not needed.
///
/// **requires** `call_async` feature
///
#[cfg(feature = "call_async")]
#[async_trait::async_trait]
pub trait CallAsync {
    /// Executed by `ThunkContext::spawn`,
    ///
    async fn call(context: &mut ThunkContext) -> anyhow::Result<()>;
}

#[cfg(feature = "call_async")]
impl<T: CallAsync + BlockObject<Shared> + Send + Sync> Plugin for T {
    fn call(context: ThunkContext) -> PluginOutput {
        context
            .spawn(|mut tc| async {
                <Self as CallAsync>::call(&mut tc).await?;
                Ok(tc)
            })
            .into()
    }
}

/// Pointer-struct for normalizing plugin types,
///
pub struct Thunk<P>(pub PhantomData<P>)
where
    P: Plugin + Send + Sync + 'static;

pub type ThunkFn = fn(ThunkContext) -> PluginOutput;

impl<P> AttributeType<Shared> for Thunk<P>
where
    P: Plugin + Send + Sync + 'static,
{
    fn ident() -> &'static str {
        <P as AttributeType<Shared>>::ident()
    }

    fn parse(parser: &mut reality::AttributeParser<Shared>, content: impl AsRef<str>) {
        <P as AttributeType<Shared>>::parse(parser, content);

        let key = parser.attributes.last().clone();
        if let Some(storage) = parser.storage() {
            storage.lazy_put_resource::<ThunkFn>(<P as Plugin>::call, key.map(|k| k.transmute()));
        }
    }
}

#[async_trait::async_trait]
impl<P> BlockObject<Shared> for Thunk<P>
where
    P: Plugin + Send + Sync + 'static,
{
    /// Called when the block object is being loaded into it's namespace,
    ///
    async fn on_load(storage: AsyncStorageTarget<Shared>) {
        <P as BlockObject<Shared>>::on_load(storage).await;
    }

    /// Called when the block object is being unloaded from it's namespace,
    ///
    async fn on_unload(storage: AsyncStorageTarget<Shared>) {
        <P as BlockObject<Shared>>::on_unload(storage).await;
    }

    /// Called when the block object's parent attribute has completed processing,
    ///
    fn on_completed(storage: AsyncStorageTarget<Shared>) -> Option<AsyncStorageTarget<Shared>> {
        <P as BlockObject<Shared>>::on_completed(storage)
    }
}

#[allow(unused_imports)]
mod tests {
    use std::collections::BTreeMap;
    use std::ops::Deref;
    use std::sync::Arc;
    use std::time::Duration;

    use async_stream::try_stream;
    use futures_util::{pin_mut, StreamExt, TryStreamExt};
    use reality::derive::*;
    use reality::*;
    use tokio::join;
    use uuid::Bytes;

    use crate::engine::EngineBuilder;
    use crate::operation::Operation;
    use crate::plugin::{PluginOutput, Thunk, ThunkContext, ThunkFn};
    use crate::{engine::Engine, plugin::call_plugin};

    use super::{CallAsync, Plugin};

    #[derive(BlockObjectType, Default, Debug, Clone)]
    #[reality(rename = "test_plugin")]
    struct TestPlugin {
        #[reality(ignore)]
        _process: String,
        name: String,
        #[reality(map_of=String)]
        env: BTreeMap<String, String>,
        #[reality(vec_of=String)]
        args: Vec<String>,
    }

    impl std::str::FromStr for TestPlugin {
        type Err = anyhow::Error;

        fn from_str(_s: &str) -> Result<Self, Self::Err> {
            Ok(TestPlugin {
                _process: _s.to_string(),
                name: String::default(),
                env: BTreeMap::new(),
                args: vec![],
            })
        }
    }

    #[async_trait::async_trait]
    impl CallAsync for TestPlugin {
        async fn call(tc: &mut super::ThunkContext) -> anyhow::Result<()> {
            let _initialized = tc.initialized::<TestPlugin>().await;
            println!("Initialized as -- {:?}", _initialized);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_plugin_model() {
        // TODO: Test Isoloation -- 7bda126d-466c-4408-b5b7-9683eea90b65
        let mut builder = Engine::builder();

        builder.register::<TestPlugin>();

        let mut engine = builder.build_primary();
        let runmd = r#"
        ```runmd
        + .operation test/operation
        <test_plugin> cargo
        : .name hello-world-2
        : RUST_LOG .env lifec=debug
        : HOME .env /home/test
        : .args --name
        : .args test
        <test_plugin> cargo
        : .name hello-world-3
        : RUST_LOG .env lifec=trace
        : HOME .env /home/test2
        : .args --name
        : .args test3

        + test_tag .operation test/operation
        <test_plugin> cargo
        : .name hello-world-2-tagged
        : RUST_LOG .env lifec=debug
        : HOME .env /home/test
        : .args --name
        : .args test
        <test_plugin> cargo
        : .name hello-world-3-tagged
        : RUST_LOG .env lifec=trace
        : HOME .env /home/test2
        : .args --name
        : .args test3
        ```
        "#;

        tokio::fs::create_dir_all(".test").await.unwrap();

        tokio::fs::write(".test/test_plugin.md", runmd)
            .await
            .unwrap();

        engine.load_file(".test/test_plugin.md").await;

        let engine = engine.compile().await;
        let _ = join!(
            engine.run("test/operation"),
            engine.run("test/operation#test_tag")
        );

        for (address, _) in engine.iter_operations() {
            println!("{address}");
        }

        ()
    }
}
