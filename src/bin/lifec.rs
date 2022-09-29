use std::{error::Error, path::PathBuf};

use clap::{Args, Parser, Subcommand};
use lifec::{Host, InspectExtensions, Project};
use tracing::{event, Level};
use tracing_subscriber::EnvFilter;

/// Simple program for parsing runmd into a World
///
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .compact()
        .init();

    let cli = Lifec::parse();
    let host = cli.create_host().await;
    match host {
        Some(mut host) => match cli {
            Lifec {
                command: Some(Commands::PrintLifecycleGraph),
                ..
            } => {
                host.print_lifecycle_graph();
            }
            Lifec {
                command: Some(Commands::PrintEngineEventGraph),
                ..
            } => {
                host.print_engine_event_graph();
            }
            Lifec {
                command: Some(Commands::Start(Start { id: Some(id), .. })),
                ..
            } => {
                host.start(id);
            }
            _ => {}
        },
        None => {
            eprintln!("Could not load host, run with `RUST_LOG=lifec=debug` for more information");
        }
    }
}

/// Struct for cli state,
/// 
#[derive(Debug, Parser)]
#[clap(name = "lifec")]
#[clap(about = "Utilities for binary for inspecting World prepared by lifec")]
struct Lifec {
    /// Path to runmd file, (defaults to .runmd in the current directory if not used)
    #[clap(short, long)]
    runmd_path: Option<String>,
    /// Url to get runmd from, must use `https`
    #[clap(long)]
    url: Option<String>,
    #[clap(subcommand)]
    command: Option<Commands>,
}

/// Enumeration of commands,
/// 
#[derive(Debug, Subcommand)]
enum Commands {
    /// Prints the lifecycle graph,
    PrintLifecycleGraph,
    /// Prints the engine event graph,
    PrintEngineEventGraph,
    /// Starts an event,
    Start(Start),
}

/// Struct for `start` command arguments
/// 
#[derive(Debug, Args)]
struct Start {
    /// Entity id of the event to start,
    ///
    /// The entity id can be retrieved from the print-engine-event-graph command
    ///
    #[clap(long)]
    id: Option<u32>,
    /// Name of engine control block to search for to start,
    /// 
    /// This will start the first event in the engine sequence,
    /// 
    #[clap(long)]
    engine_name: Option<String>,
}

impl Lifec {
    /// Creates a new lifec host,
    /// 
    /// Will parse runmd from either a url, local file path, or current directory
    ///
    pub async fn create_host(&self) -> Option<Host> {
        match self {
            Self {
                url: Some(url),
                ..
            } => {
                match Host::get::<Lifec>(url).await {
                    Ok(host) => {
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
    
                match Host::open::<Lifec>(runmd_path).await {
                    Ok(host) => Some(host),
                    Err(err) => {
                        event!(Level::ERROR, "Could not load runmd from path {err}");
                        None
                    }
                }
            },
            _ => {
                match Host::runmd::<Lifec>().await {
                    Ok(host) => Some(host),
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

impl Project for Lifec {
    fn configure_engine(_engine: &mut lifec::Engine) {
        // No-op
    }

    fn interpret(_world: &lifec::World, _block: &lifec::Block) {
        // No-op
    }
}
