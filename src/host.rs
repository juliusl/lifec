mod guest_runtime;
use std::{path::PathBuf, error::Error};

pub use guest_runtime::GuestRuntime;
use specs::World;
use tracing::{event, Level};

use crate::Project;

/// Struct for starting engines compiled from a 
/// project type,
/// 
pub struct Host 
{
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

    /// Opens a file, compiles, and returns a host,
    /// 
    pub async fn open<P>(path: impl AsRef<PathBuf>) -> Result<Self, impl Error> 
    where
        P: Project
    {
        match tokio::fs::read_to_string(path.as_ref()).await {
            Ok(runmd) => {
                Ok(Host::load_content::<P>(runmd))
            },
            Err(err) => {
                event!(Level::ERROR, "Could not open file {err}");
                Err(err)
            },
        }
    }

    /// Opens a file, compiles, and returns a host
    /// 
    pub fn load_content<P>(content: impl AsRef<str>) -> Self
    where
        P: Project
    {
        Self { world: P::compile(content) }  
    }
}

