use hyper::client::HttpConnector;
use reality::BlockProperties;
use specs::{storage::DenseVecStorage, Component, Entity};

mod error;
pub use error::ErrorContext;

mod thunk_context;
pub use thunk_context::ThunkContext;

use crate::editor::General;

use super::Plugin;
use std::fmt::Debug;
use std::hash::Hash;
use tokio::task::JoinHandle;

/// Thunk is a function that can be passed around for the system to call later
#[derive(Component, Clone)]
#[storage(DenseVecStorage)]
pub struct Thunk(
    // Symbol that represents this thunk
    pub &'static str,
    // thunk fn
    pub fn(&mut ThunkContext) -> Option<(JoinHandle<ThunkContext>, CancelToken)>,
);

impl Thunk {
    /// Returns the symbol for the thunk
    pub fn symbol(&self) -> &'static str {
        self.0
    }

    /// Generates a thunk from a plugin impl
    pub fn from_plugin<P>() -> Self
    where
        P: Plugin + ?Sized,
    {
        Self(P::symbol(), P::call)
    }
}

impl PartialEq for Thunk {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Thunk {}

impl Hash for Thunk {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl Debug for Thunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Thunk").field(&self.0).finish()
    }
}

impl Into<General> for &Thunk {
    fn into(self) -> General {
        General {
            name: self.0.to_string(),
        }
    }
}

/// Config for a thunk context
#[derive(Component, Clone)]
#[storage(DenseVecStorage)]
pub struct Config(BlockProperties);

impl Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Config").field(&self.0).finish()
    }
}

impl AsRef<Config> for Config {
    fn as_ref(&self) -> &Config {
        self
    }
}

/// StatusUpdate for stuff like progress bars
pub type StatusUpdate = (
    // entity with an update
    Entity,
    // progress
    f32,
    // status message
    String,
);

/// Cancel token stored by the event runtime
pub type CancelToken = tokio::sync::oneshot::Sender<()>;

/// Cancel source stored by the thunk
pub type CancelSource = tokio::sync::oneshot::Receiver<()>;

/// Secure client for making http requests
pub type SecureClient = hyper::Client<hyper_tls::HttpsConnector<HttpConnector>>;
