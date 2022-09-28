mod guest_runtime;
use std::{error::Error, path::PathBuf};

pub use guest_runtime::GuestRuntime;
use specs::{DispatcherBuilder, World};
use tracing::{event, Level};

use crate::{plugins::EventRuntime, Project};

/// Struct for starting engines compiled from a
/// project type,
///
pub struct Host {
    world: World,
}

impl Host {
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

    /// Opens a file, compiles, and returns a host
    ///
    pub fn load_content<P>(content: impl AsRef<str>) -> Self
    where
        P: Project,
    {
        Self {
            world: P::compile(content),
        }
    }
}

impl Into<World> for Host {
    fn into(self) -> World {
        self.world
    }
}
