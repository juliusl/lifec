use clap::Args;
use hyper::{Client, Uri};
use hyper_tls::HttpsConnector;
use specs::{DispatcherBuilder, World, WorldExt};
use std::{error::Error, fmt::Debug, path::PathBuf, str::from_utf8};
use tokio::sync::oneshot::error::TryRecvError;
use tracing::{event, Level};

use crate::{plugins::EventRuntime, Event, ExitListener, Project, ThunkContext};

mod inspect;
pub use inspect::InspectExtensions;

mod start;
pub use start::Start;

mod commands;
pub use commands::Commands;

/// Struct for starting engines compiled from a
/// project type,
///
#[derive(Default, Args)]
#[clap(arg_required_else_help=true)]
pub struct Host {
    /// URL to runmd to use when configuring this mirror engine
    #[clap(long)]
    url: Option<String>,
    /// Path to runmd file used to configure the mirror engine
    /// Defaults to .runmd
    #[clap(long)]
    runmd_path: Option<String>,
    #[clap(subcommand)]
    commands: Option<Commands>,
    #[clap(skip)]
    world: Option<World>,
}

/// CLI functions
/// 
impl Host {
    /// Handles the current command
    /// 
    pub fn handle_start(&mut self) {
        match self.command() {
            Some(Commands::Start(Start { id: Some(id), .. })) => {
                 self.start(*id);
            }
            Some(Commands::Start(Start { engine_name: Some(_engine_name), .. })) => {
                 todo!()
            }
            _ => {
                unreachable!("A command should exist by this point")
            }
        }
    }

    /// Returns the current command,
    /// 
    pub fn command(&self) -> Option<&Commands> {
        self.commands.as_ref()
    }

    /// Sets the command argument,
    /// 
    pub fn set_command(&mut self, command: Commands) {
        self.commands = Some(command);
    }

    /// Sets the runmd path argument, if None defaults to ./.runmd
    /// 
    pub fn set_path(&mut self, path: impl AsRef<str>) {
        self.runmd_path = Some(path.as_ref().to_string());
    }

    /// Sets the runmd url argument,
    /// 
    pub fn set_url(&mut self, url: impl AsRef<str>) {
        self.url = Some(url.as_ref().to_string());
    }

    /// Creates a new lifec host,
    /// 
    /// Will parse runmd from either a url, local file path, or current directory
    ///
    pub async fn create_host<P>(&self) -> Option<Host> 
    where
        P: Project
    {
        let command = self.command().cloned();
        match self {
            Self {
                url: Some(url),
                ..
            } => {
                match Host::get::<P>(url).await {
                    Ok(mut host) => {
                        host.commands = command;
                        return Some(host);
                    }
                    Err(err) => {
                        event!(Level::ERROR, "Could not get runmd from url {url}, {err}");
                        return None;
                    }
                }
            }
            Self {
                runmd_path: Some(runmd_path),
                ..
            } => {
                let mut runmd_path = PathBuf::from(runmd_path);
                if !runmd_path.ends_with(".runmd") || runmd_path.is_dir() {
                    runmd_path = runmd_path.join(".runmd");
                }
    
                match Host::open::<P>(runmd_path).await {
                    Ok(mut host) => { 
                        host.commands = command;    
                        Some(host) 
                    },
                    Err(err) => {
                        event!(Level::ERROR, "Could not load runmd from path {err}");
                        None
                    }
                }
            },
            _ => {
                match Host::runmd::<P>().await {
                    Ok(mut host) => {
                        host.commands = command;
                        Some(host)
                    },
                    Err(err) => {
                        event!(
                            Level::ERROR,
                            "Could not load `.runmd` from current directory {err}"
                        );
                        None
                    }
                }
            }
        }
    }
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
        self.world.as_ref().expect("World should exist")
    }

    /// Returns a mutable reference to the world,
    ///
    pub fn world_mut(&mut self) -> &mut World {
        self.world.as_mut().expect("World should exist")
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
        let path = path.into();
        match tokio::fs::read_to_string(&path).await {
            Ok(runmd) => Ok(Host::load_content::<P>(runmd)),
            Err(err) => {
                event!(Level::ERROR, "Could not open file {:?}, {err}", path);
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

        match uri.as_ref().parse::<Uri>() {
            Ok(uri) => match client.get(uri).await {
                Ok(mut response) => match hyper::body::to_bytes(response.body_mut()).await {
                    Ok(bytes) => {
                        let bytes = bytes.to_vec();
                        let content =
                            from_utf8(&bytes).expect("should be able to read into a string");
                        Ok(Self::load_content::<P>(&content))
                    }
                    Err(err) => {
                        panic!("Could not read bytes {err}")
                    }
                },
                Err(err) => {
                    event!(Level::ERROR, "Could not get content {err}");
                    Err(err)
                }
            },
            Err(err) => {
                panic!("Could not parse uri, {}, {err}", uri.as_ref());
            }
        }
    }

    /// Compiles runmd content into a Host,
    ///
    pub fn load_content<P>(content: impl AsRef<str>) -> Self
    where
        P: Project,
    {
        Self {
            runmd_path: None,
            url: None,
            commands: None,
            world: Some(P::compile(content)),
        }
    }

    /// Returns true if should exit,
    ///
    pub fn should_exit(&self) -> bool {
        let mut exit_listener = self.world().write_resource::<ExitListener>();
        match exit_listener.1.try_recv() {
            Ok(_) => true,
            Err(err) => match err {
                TryRecvError::Empty => false,
                TryRecvError::Closed => true,
            },
        }
    }

    /// Starts an event entity,
    ///
    pub fn start(&mut self, event_entity: u32) {
        let mut dispatcher = {
            let dispatcher = Host::dispatcher_builder();
            dispatcher.build()
        };
        dispatcher.setup(self.world_mut());

        let event = self.world().entities().entity(event_entity);
        if let Some(event) = self.world().write_component::<Event>().get_mut(event) {
            event.fire(ThunkContext::default());
        }
        self.world_mut().maintain();

        // TODO - Exit is currently in development
        while !self.should_exit() {
            dispatcher.dispatch(self.world());
        }
    }
}

impl Debug for Host {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Host")
            .field("url", &self.url)
            .field("runmd_path", &self.runmd_path)
            .finish()
    }
}


impl Into<World> for Host {
    fn into(self) -> World {
        self.world.unwrap()
    }
}
