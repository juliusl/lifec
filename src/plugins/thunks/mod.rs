
use crate::Operation;
use hyper::client::HttpConnector;
use specs::Component;
use specs::{storage::DenseVecStorage, Entity};

mod error;
pub use error::ErrorContext;

mod thunk_context;
pub use thunk_context::ThunkContext;


use super::Plugin;
use tokio::task::JoinHandle;

/// Thunk is a function that can be passed around for the system to call later
#[derive(Component, Clone)]
#[storage(DenseVecStorage)]
pub struct Thunk(
    // Symbol that represents this thunk
    pub &'static str,
    // thunk fn
    pub fn(&ThunkContext) -> Option<(JoinHandle<ThunkContext>, CancelToken)>,
    /// setup thunk fn
    pub fn(&ThunkContext) -> Operation,
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
    /// Returns the symbol for the thunk
    pub fn symbol(&self) -> &'static str {
        self.0
    }
    
    /// Generates a thunk from a plugin impl
    pub fn from_plugin<P>() -> Self
    where
        P: Plugin + ?Sized,
    {
        Self(P::symbol(), P::call, P::setup_operation)
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
