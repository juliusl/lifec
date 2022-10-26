use std::{path::PathBuf, str::FromStr};
use hyper::Uri;

use clap::Args;
use tracing::{event, Level};

use crate::prelude::Project;

use super::{Commands, Host, Editor};

/// CLI arguments that can be configured into a host,
/// 
#[derive(Debug, Default, Args)]
#[clap(arg_required_else_help = true)]
pub struct HostSettings {
    /// Root directory, defaults to current directory
    ///
    #[clap(long)]
    pub root: Option<PathBuf>,
    /// URL to .runmd file used to configure this host,
    ///
    #[clap(long)]
    pub url: Option<String>,
    /// Path to runmd file used to configure this host,
    /// Defaults to .runmd,
    ///
    #[clap(long)]
    pub runmd_path: Option<String>,
    /// Uri for a workspace, Format: {tenant}.{host}/{optional- path}#{optional- tag}, Ex: test.localhost/calculator#demo
    ///
    /// A workspace directory is a directory of .runmd files that are compiled together. A valid workspace directory requires a root
    /// .runmd file, followed by named runmd files (ex. test.runmd).
    /// 
    /// Named files will be parsed w/ the file name used as the implicit block symbol. All named files will be parsed first and the root .runmd file will be parsed last. 
    /// 
    /// When this mode is used, the workspace feature
    /// will be enabled with thunk contexts, so all plugins will execute in the context of the same work_dir.
    ///
    #[clap(short, long)]
    pub workspace: Option<String>,
    /// The command to execute w/ this host,
    ///
    #[clap(subcommand)]
    pub command: Option<Commands>,
}

impl HostSettings {
    
    /// Returns the current command,
    ///
    pub fn command(&self) -> Option<&Commands> {
        self.command.as_ref()
    }

    /// Sets the command argument,
    ///
    pub fn set_command(&mut self, command: Commands) {
        self.command = Some(command);
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

    /// Sets the workspace uri,
    /// 
    pub fn set_workspace(&mut self, workspace: impl AsRef<str>) {
        self.workspace = Some(workspace.as_ref().to_string());
    }

    /// Handles the current command
    ///
    pub async fn handle<P>(&self)
    where
        P: Project,
    {
        match self.command() {
            Some(Commands::Start(start)) => {
                if let Some(host) = self.create_host::<P>().await {
                    host.with_start(start).start::<P>();
                }
            },
            Some(Commands::Open) => {
                if let Some(host) = self.create_host::<P>().await {
                    tokio::task::block_in_place(|| {
                        host.open_runtime_editor::<P>();
                    })
                }
            }
            _ => {
                unreachable!("A command should exist by this point")
            }
        }
    }

    /// Creates a new lifec host,
    ///
    /// Will parse runmd from either a url, local file path, or current directory
    ///
    pub async fn create_host<P>(&self) -> Option<Host>
    where
        P: Project,
    {
        match self {
            Self {
                root,
                workspace: Some(workspace),
                ..
            } => match Uri::from_str(workspace) {
                Ok(uri) => {
                    if let Some((tenant, host)) =
                        uri.host().expect("should have a host").split_once(".")
                    {
                        let root = root.clone();
                        let host = Host::load_workspace::<P>(
                            root,
                            host,
                            tenant,
                            if uri.path().is_empty() {
                                None
                            } else {
                                Some(uri.path())
                            },
                            if let Some((_, fragment)) = uri.to_string().split_once("#") {
                                Some(fragment.to_string())
                            } else {
                                None
                            }
                        );

                        Some(host)
                    } else {
                        event!(Level::ERROR, "Tenant and host are required");
                        None
                    }
                }
                Err(err) => {
                    event!(Level::ERROR, "Could not parse workspace uri, {err}");
                    None
                }
            },
            Self { url: Some(url), .. } => match Host::get::<P>(url).await {
                Ok(host) => {
                    return Some(host);
                }
                Err(err) => {
                    event!(Level::ERROR, "Could not get runmd from url {url}, {err}");
                    return None;
                }
            },
            Self {
                runmd_path: Some(runmd_path),
                ..
            } => {
                let mut runmd_path = PathBuf::from(runmd_path);
                if runmd_path.is_dir() {
                    runmd_path = runmd_path.join(".runmd");
                }

                match Host::open::<P>(runmd_path.clone()).await {
                    Ok(host) => {
                        Some(host)
                    }
                    Err(err) => {
                        event!(Level::ERROR, "Could not load runmd from path {err}");
                        None
                    }
                }
            }
            _ => match Host::runmd::<P>().await {
                Ok(host) => {
                    Some(host)
                }
                Err(err) => {
                    event!(
                        Level::ERROR,
                        "Could not load `.runmd` from current directory {err}"
                    );
                    None
                }
            },
        }
    }
}

