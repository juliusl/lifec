use lifec::{Host, InspectExtensions, Project, Start};
use clap::{Parser, Subcommand};
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
    match cli {
        Lifec {
            runmd_path,
            url,
            command: Some(c),
            ..
        } => {
            let mut host = Host::default();
            match (runmd_path, url) {
                (Some(runmd_path), None) => {
                    host.set_path(runmd_path);
                }, 
                (None, Some(url)) => {
                    host.set_url(url);
                }, 
                _ => {}
            }

            match c {
                Commands::Start(start) => {
                    host.set_command(lifec::Commands::Start(start));
                    let mut host = host.create_host::<Lifec>().await.expect("Should be able to create host");
                    host.handle_start();
                }
                Commands::Host(host) => {
                    let mut host = host.create_host::<Lifec>().await.expect("Should be able to create host");
                    host.handle_start();
                }
                Commands::PrintEngineGraph => {
                    let mut host = host.create_host::<Lifec>().await.expect("Should be able to create host");
                    host.print_engine_event_graph();
                }
                Commands::PrintLifecycleGraph => {
                    let mut host = host.create_host::<Lifec>().await.expect("Should be able to create host");
                    host.print_lifecycle_graph();
                }
            }
        }
        _ => {
            eprintln!("Could not load host, run with `RUST_LOG=lifec=debug` for more information");
        }
    }
}

/// Struct for cli state,
/// 
#[derive(Debug, Parser)]
#[clap(name = "lifec")]
#[clap(about = "Utilities for working with the World created by lifec, limited to process, install, println, timer plugins")]
struct Lifec {
    /// URL to runmd to fetch to create host
    #[clap(long)]
    url: Option<String>,
    /// Path to runmd file to create host
    #[clap(long)]
    runmd_path: Option<String>,
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
    PrintEngineGraph,
    /// Host commands,
    Host(Host),
    /// Shortcut for `host start` command,
    Start(Start),
}

impl Project for Lifec {
    fn configure_engine(_engine: &mut lifec::Engine) {
        // No-op
    }

    fn interpret(_world: &lifec::World, _block: &lifec::Block) {
        // No-op
    }
}
