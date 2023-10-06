use std::{future::Future, ops::Deref, sync::Arc};

use reality::{AsyncStorageTarget, AttributeType, ResourceKey, Shared, StorageTarget};
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
    storage: AsyncStorageTarget<Shared>,
    /// Cancellation token that can be used by the engine to signal shutdown,
    ///
    pub cancellation: tokio_util::sync::CancellationToken,
}

impl From<AsyncStorageTarget<Shared>> for ThunkContext {
    fn from(value: AsyncStorageTarget<Shared>) -> Self {
        Self {
            storage: value,
            cancellation: CancellationToken::new(),
        }
    }
}

impl ThunkContext {
    /// Get read access to storage,
    ///
    pub async fn storage(&self) -> tokio::sync::RwLockReadGuard<Shared> {
        self.storage.storage.read().await
    }

    /// Get mutable access to storage,
    ///
    pub async fn storage_mut(&self) -> tokio::sync::RwLockWriteGuard<Shared> {
        self.storage.storage.write().await
    }

    /// Tries to get access to storage,
    ///
    pub fn try_storage(&self) -> Option<tokio::sync::RwLockReadGuard<Shared>> {
        self.storage.storage.try_read().ok()
    }

    /// Tries to get mutable access to storage,
    ///
    pub fn try_storage_mut(&mut self) -> Option<tokio::sync::RwLockWriteGuard<Shared>> {
        self.storage.storage.try_write().ok()
    }

    /// Spawn a task w/ this context,
    ///
    /// Returns a join-handle if the task was created.
    ///
    pub fn spawn<F>(self, task: impl FnOnce(ThunkContext) -> F + 'static) -> SpawnResult
    where
        F: Future<Output = anyhow::Result<ThunkContext>> + Send + Sync + 'static,
    {
        self.storage
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
    pub async fn initialized<P: Plugin + Default + Clone + Send + Sync + 'static>(&self) -> P {
        self.storage
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

mod tests {
    use std::sync::Arc;

    use reality::derive::*;
    use reality::*;

    use crate::plugin::ThunkContext;

    use super::Plugin;

    #[derive(BlockObjectType, Default, Debug, Clone)]
    #[reality(rename = "test/plugin")]
    struct TestPlugin {
        name: String,
    }

    impl std::str::FromStr for TestPlugin {
        type Err = anyhow::Error;

        fn from_str(_s: &str) -> Result<Self, Self::Err> {
            Ok(TestPlugin {
                name: String::default(),
            })
        }
    }

    impl Plugin for TestPlugin {
        fn call(context: super::ThunkContext) -> super::PluginOutput {
            context
                .spawn(|tc| async {
                    let _initialized = tc.initialized::<TestPlugin>().await;
                    println!("Initialized as -- {:?}", _initialized);
                    Ok(tc)
                })
                .into()
        }
    }

    #[tokio::test]
    async fn test_plugin_model() {
        let mut project = reality::Project::new(reality::Shared::default());

        project.add_node_plugin("test", |_, _, parser| {
            parser.with_object_type::<TestPlugin>();
        });

        let runmd = r#"
        ```runmd
        + .test
        <test/plugin>
        : .name hello-world-2
        ```
        "#;

        tokio::fs::create_dir_all(".test").await.unwrap();

        tokio::fs::write(".test/test_plugin.md", runmd)
            .await
            .unwrap();

        let project = project.load_file(".test/test_plugin.md").await.unwrap();

        let mut nodes = project.nodes.into_inner().unwrap();

        for (_, target) in nodes.drain() {
            if let Ok(unwrapped) = Arc::try_unwrap(target) {
                let unwrapped = unwrapped.into_inner();

                let storage = unwrapped.into_thread_safe();
                let tc = ThunkContext::from(storage);

                match TestPlugin::call(tc) {
                    crate::plugin::PluginOutput::Spawn(Some(spawned)) => {
                        let result = spawned.await;
                        println!("is_result_ok, {}", result.is_ok());
                    },
                    _ => {}
                }
            }
        }

        ()
    }
}
