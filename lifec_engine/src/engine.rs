use std::{sync::Arc, path::Path, collections::BTreeMap};
use reality::{AsyncStorageTarget, Project, Shared};
use tokio_util::sync::CancellationToken;
use crate::plugin::{ThunkContext, Plugin, Thunk};

/// Table of plugins by engine uuid,
/// 
static PLUGINS_TABLE: std::sync::RwLock<BTreeMap<uuid::Uuid, Vec<reality::BlockPlugin<Shared>>>> = std::sync::RwLock::new(BTreeMap::new());

/// Uuid for the primary engine uuid,
/// 
const PRIMARY: u128 = 0;

pub struct EngineBuilder {
    /// Plugins to register w/ the Engine
    /// 
    plugins: Vec<reality::BlockPlugin<Shared>>,
    /// Runtime builder,
    /// 
    runtime_builder: tokio::runtime::Builder,
}

impl EngineBuilder {
    /// Creates a new engine builder,
    /// 
    pub fn new(runtime_builder: tokio::runtime::Builder) -> Self {
        Self { plugins: vec![], runtime_builder }
    }

    /// Registers a plugin w/ this engine builder,
    /// 
    pub fn register<P: Plugin + Send + Sync + 'static>(&mut self) {
        self.plugins.push(|parser| {
            parser.with_object_type::<Thunk<P>>();
        });
    }

    /// Builds the current engine under the primary uuid (0),
    /// 
    pub fn build_primary(self) -> Engine<PRIMARY> {
        self.build()
    }

    /// Consumes the builder and returns a new engine,
    /// 
    pub fn build<const UUID: u128>(mut self) -> Engine<UUID> {
        let runtime = self.runtime_builder.build().unwrap();

        let mut engine = Engine::new_with(runtime);
        {
            if let Ok(mut plugins) = PLUGINS_TABLE.write() {
                plugins.insert(uuid::Uuid::from_u128(UUID), self.plugins.clone());
            }
        }

        if let Some(project) = engine.project.as_mut() {
            project.add_block_plugin(None, None, |_| {});
            project.add_node_plugin("operation", move |_, _, target| {
                if let Some(plugins) = PLUGINS_TABLE.read().ok().and_then(|plugins| plugins.get(&uuid::Uuid::from_u128(UUID)).cloned()) {
                    for p in plugins.iter() {
                        p(target);
                    }
                }
            });
        }

        engine
    }
}

/// Struct containing engine config/state,
///
/// # Background
///
/// By definition an engine is a sequence of event. This struct will be built by defining events and sequencing in a seperate file using runmd.
///
/// Events will be configured via a plugin model. Plugins will execute when the event is loaded in the order they are defined.
///
/// Plugins are executed as "Thunks" in a "call-by-name" fashion. Plugins belonging to an event share state linearly, meaning after a plugin executes, it can modify state before the next plugin executes.
///
/// An event may have 1 or more plugins.
///
/// ```md
/// # Example engine definition
///
/// ```runmd <application/lifec.engine> mirror
/// <..start> start
/// <..start> cleanup
/// <..loop>
/// ```
///
/// ```runmd <application/lifec.engine.event> start
/// + .runtime
/// ```
///
/// ```runmd <application/lifec.engine.event> cleanup
/// + .runtime
/// ```
///
/// ```
///
pub struct Engine<const UUID: u128> {
    /// Project,
    ///
    pub project: Option<Project<Shared>>,
    /// Wrapped w/ a runtime so that it can be dropped properly
    ///
    runtime: Option<tokio::runtime::Runtime>,
    /// Cancelled when the engine is dropped,
    ///
    cancellation: CancellationToken,
}

impl Engine<0> {
    /// Creates a new engine builder,
    /// 
    pub fn builder() -> EngineBuilder {
        EngineBuilder::new(tokio::runtime::Builder::new_multi_thread())
    }
}

impl<const UUID: u128> Engine<UUID> {
    /// Loads a file,
    /// 
    pub async fn load_file(&mut self, path: impl AsRef<Path>) {
        if let Some(project) = self.project.take() {
            self.project = project.load_file(path).await.ok();
        }
    }

    /// Creates a new engine,
    ///
    /// **Note** By default creates a new multi_thread runtime w/ all features enabled
    ///
    pub fn new() -> Self {
        let mut runtime = tokio::runtime::Builder::new_multi_thread();
        runtime.enable_all();
        let runtime = runtime.build().expect("should have an engine");

        Engine::new_with(runtime)
    }

    /// Creates a new engine w/ runtime,
    ///
    pub fn new_with(runtime: tokio::runtime::Runtime) -> Self {
        Engine {
            project: Some(Project::new(Shared::default())),
            runtime: Some(runtime),
            cancellation: CancellationToken::new(),
        }
    }

    /// Creates a new context on this engine,
    /// 
    /// **Note** Each time a thunk context is created a new output storage target is generated, however the original storage target is used.
    /// 
    pub fn new_context(&self, storage: Arc<tokio::sync::RwLock<Shared>>) -> ThunkContext {
        let mut context = ThunkContext::from(AsyncStorageTarget::from_parts(
            storage,
            self.runtime
                .as_ref()
                .map(|r| r.handle().clone())
                .expect("should have a runtime"),
        ));
        context.cancellation = self.cancellation.child_token();
        context
    }
}

impl<const UUID: u128> Default for Engine<UUID> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const UUID: u128>  Drop for Engine<UUID> {
    fn drop(&mut self) {
        self.cancellation.cancel();

        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_background();
        }
    }
}
