use std::future::Future;

use reality::{AsyncStorageTarget, AttributeType, Shared, StorageTarget};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

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

impl From<SpawnResult> for PluginOutput {
    fn from(value: SpawnResult) -> Self {
        PluginOutput::Spawn(value)
    }
}

/// Struct containing shared context between plugins,
///
#[derive(Clone)]
pub struct ThunkContext {
    /// Storage mapping to this context,
    ///
    /// **Note**: Storage will be initialized by runmd.
    ///
    target: AsyncStorageTarget<Shared>,
    /// Cancellation token that can be used by the engine to signal shutdown,
    ///
    pub cancellation: tokio_util::sync::CancellationToken,
}

impl From<AsyncStorageTarget<Shared>> for ThunkContext {
    fn from(value: AsyncStorageTarget<Shared>) -> Self {
        Self {
            target: value,
            cancellation: CancellationToken::new(),
        }
    }
}

impl ThunkContext {
    /// Get read access to storage,
    ///
    pub async fn storage(&self) -> tokio::sync::RwLockReadGuard<Shared> {
        self.target.storage.read().await
    }

    /// Get mutable access to storage,
    ///
    pub async fn storage_mut(&self) -> tokio::sync::RwLockWriteGuard<Shared> {
        self.target.storage.write().await
    }

    /// Tries to get access to storage,
    ///
    pub fn try_storage(&self) -> Option<tokio::sync::RwLockReadGuard<Shared>> {
        self.target.storage.try_read().ok()
    }

    /// Tries to get mutable access to storage,
    ///
    pub fn try_storage_mut(&mut self) -> Option<tokio::sync::RwLockWriteGuard<Shared>> {
        self.target.storage.try_write().ok()
    }

    /// Spawn a task w/ this context,
    ///
    /// Returns a join-handle if the task was created.
    ///
    pub fn spawn<F>(self, task: impl FnOnce(ThunkContext) -> F + 'static) -> SpawnResult
    where
        F: Future<Output = anyhow::Result<ThunkContext>> + Send + 'static,
    {
        self.target
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
        self.target
            .storage
            .read()
            .await
            .resource::<P>(None)
            .map(|r| r.clone())
            .unwrap_or_default()
    }
}

/// Type-alias for the fn exported by a type that implements Plugin
///
pub type Thunk = fn(ThunkContext) -> PluginOutput;

/// Allows users to export logic as a simple fn,
///
pub trait Plugin: AttributeType<Shared> {
    /// Called when an event executes,
    ///
    /// Returning PluginOutput determines the behavior of the Event.
    ///
    fn call(context: ThunkContext) -> PluginOutput;
}

/// Executes a plugin immediately,
/// 
pub async fn call_plugin<P: Plugin + Send + Sync>(tc: ThunkContext) -> anyhow::Result<ThunkContext> {
    match <P as Plugin>::call(tc) {
        PluginOutput::Spawn(Some(spawned)) => {
            spawned.await?
        },
        _ => {
            Err(anyhow::anyhow!("Could not spawn plugin call"))
        }
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
impl<T: CallAsync + AttributeType<Shared> + Send + Sync> Plugin for T {
    fn call(context: ThunkContext) -> PluginOutput {
        context.spawn(|mut tc| async {
            <Self as CallAsync>::call(&mut tc).await?;
            Ok(tc)
        }).into()
    }
}

#[allow(unused_imports)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use reality::derive::*;
    use reality::*;

    use crate::{engine::Engine, plugin::call_plugin};
    use crate::plugin::ThunkContext;

    use super::{Plugin, CallAsync};

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
                args: vec![]
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
        let mut project = reality::Project::new(reality::Shared::default());

        project.add_block_plugin(None, None, |_| {
        });

        project.add_node_plugin("test", |_, _, parser| {
            parser.with_object_type::<TestPlugin>();
        });

        let runmd = r#"
        ```runmd
        + .test
        <test_plugin> cargo
        : .name hello-world-2
        : RUST_LOG .env lifec=debug
        : HOME .env /home/test
        : .args --name
        : .args test
        ```
        "#;

        tokio::fs::create_dir_all(".test").await.unwrap();

        tokio::fs::write(".test/test_plugin.md", runmd)
            .await
            .unwrap();

        let project = project.load_file(".test/test_plugin.md").await.unwrap();

        let nodes = project.nodes.into_inner().unwrap();

        let engine = Engine::new();

        for (_, target) in nodes.iter() {
            let tc = engine.new_context(target.clone());

            let _ =  call_plugin::<TestPlugin>(tc).await;
        }

        ()
    }
}