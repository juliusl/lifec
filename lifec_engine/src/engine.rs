use std::sync::Arc;

use reality::{AsyncStorageTarget, Shared};
use tokio_util::sync::CancellationToken;

use crate::plugin::ThunkContext;

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
#[derive(Default)]
pub struct Engine {
    /// Wrapped w/ a runtime so that it can be dropped properly
    /// 
    runtime: Option<tokio::runtime::Runtime>,
    /// Cancelled when the engine is dropped,
    /// 
    cancellation: CancellationToken,
}

impl Engine {
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
        Engine { runtime: Some(runtime), cancellation: CancellationToken::new() }
    }

    /// Creates a new context on this engine,
    /// 
    pub fn new_context(&self, storage: Arc<tokio::sync::RwLock<Shared>>) -> ThunkContext {
        let mut context = ThunkContext::from(AsyncStorageTarget::from_parts(
            storage,
            self.runtime.as_ref().map(|r| r.handle().clone()).expect("should have a runtime"),
        ));
        context.cancellation = self.cancellation.child_token();
        context
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.cancellation.cancel();

        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_background();
        }
    }
}
