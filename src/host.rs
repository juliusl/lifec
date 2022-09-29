use std::{error::Error, path::PathBuf, str::from_utf8};
use specs::{DispatcherBuilder, World, WorldExt};
use hyper_tls::HttpsConnector;
use tokio::sync::oneshot::error::TryRecvError;
use tracing::{event, Level};
use hyper::{Client, Uri};

use crate::{plugins::EventRuntime, Project, ExitListener};

mod guest_runtime;
pub use guest_runtime::GuestRuntime;

/// Struct for starting engines compiled from a
/// project type,
///
pub struct Host {
    world: World,
}

impl Host {
    /// Returns a new dispatcher builder with core
    /// systems included.
    ///
    /// When adding additional systems, the below systems can be used as dependencies
    ///
    /// # Systems Included:
    /// * event_runtime - System that manages running engines.
    ///
    pub fn dispatcher_builder<'a, 'b>() -> DispatcherBuilder<'a, 'b> {
        let dispatcher_builder = DispatcherBuilder::new();

        dispatcher_builder.with(EventRuntime::default(), "event_runtime", &[])
    }

    /// Returns a immutable reference to the world,
    ///
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Returns a mutable reference to the world,
    ///
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Opens the .runmd file in the current directory,
    ///
    pub async fn runmd<P>() -> Result<Self, impl Error>
    where
        P: Project,
    {
        Self::open::<P>(".runmd").await
    }

    /// Opens a file, compiles, and returns a host,
    ///
    pub async fn open<P>(path: impl Into<PathBuf>) -> Result<Self, impl Error>
    where
        P: Project,
    {
        match tokio::fs::read_to_string(path.into()).await {
            Ok(runmd) => Ok(Host::load_content::<P>(runmd)),
            Err(err) => {
                event!(Level::ERROR, "Could not open file {err}");
                Err(err)
            }
        }
    }

    /// Opens a uri via GET, compiles the body, and returns a host,
    /// 
    pub async fn get<P>(uri: impl AsRef<str>) -> Result<Self, impl Error> 
    where
        P: Project,
    {
        let https = HttpsConnector::new();
        let client = Client::builder().build::<_, hyper::Body>(https);

        match  uri.as_ref().parse::<Uri>() {
            Ok(uri) => match client.get(uri).await {
                Ok(mut response) => {
                    match hyper::body::to_bytes(response.body_mut()).await {
                        Ok(bytes) => {
                            let bytes = bytes.to_vec();
                            let content = from_utf8(&bytes).expect("should be able to read into a string");
                            Ok(Self::load_content::<P>(&content))
                        },
                        Err(err) => {
                            panic!("Could not read bytes {err}")
                        },
                    }
                },
                Err(err) => {
                    event!(Level::ERROR, "Could not get content {err}");
                    Err(err)
                },
            },
            Err(err) => {
                panic!("Could not parse uri, {}, {err}", uri.as_ref());
            },
        }
    }

    /// Compiles runmd content into a Host, 
    /// 
    pub fn load_content<P>(content: impl AsRef<str>) -> Self
    where
        P: Project,
    {
        Self {
            world: P::compile(content),
        }
    }

    /// Returns true if should exit,
    /// 
    pub fn should_exit(&self) -> bool {
        let mut exit_listener = self.world.write_resource::<ExitListener>();
        match exit_listener.1.try_recv() {
            Ok(_) => true,
            Err(err) => match err {
                TryRecvError::Empty => false,
                TryRecvError::Closed => true,
            },
        }
    }
}

impl Into<World> for Host {
    fn into(self) -> World {
        self.world
    }
}
