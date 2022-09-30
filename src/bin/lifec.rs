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
            command: Some(Commands::Host(host)),
            ..
        } => {
            let mut host = host.create_host::<Lifec>().await.expect("Should be able to create host");
            host.handle_start();
        }
        Lifec {
            command: Some(Commands::PrintLifecycleGraph(host)),
            ..
        } => {
            let mut host = host.create_host::<Lifec>().await.expect("Should be able to create host");
            host.print_lifecycle_graph()
        }
        Lifec {
            command: Some(Commands::PrintEngineGraph(host)),
            ..
        } => {
            let mut host = host.create_host::<Lifec>().await.expect("Should be able to create host");
            host.print_engine_event_graph()
        }
        // TODO -- DRY
        Lifec {
            command: Some(Commands::Start(start)),
            runmd_path: Some(runmd_path),
            ..
        } => {
            let mut host = Host::default();
            host.set_command(lifec::Commands::Start(start));
            host.set_path(runmd_path);
            let mut host = host.create_host::<Lifec>().await.expect("Should be able to create host");
            host.handle_start();
        }
        Lifec {
            command: Some(Commands::Start(start)),
            url: Some(url),
            ..
        } => {
            let mut host = Host::default();
            host.set_command(lifec::Commands::Start(start));
            host.set_url(url);
            let mut host = host.create_host::<Lifec>().await.expect("Should be able to create host");
            host.handle_start();
        }
        Lifec {
            command: Some(Commands::Start(start)),
            url: None,
            runmd_path: None,
        } => {
            let mut host = Host::default();
            host.set_command(lifec::Commands::Start(start));
            let mut host = host.create_host::<Lifec>().await.expect("Should be able to create host");
            host.handle_start();
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
#[clap(about = "Utilities for working with the World created by lifec")]
struct Lifec {
    /// URL to runmd to use when configuring this mirror engine
    #[clap(long)]
    url: Option<String>,
    /// Path to runmd file used to configure the mirror engine
    /// Defaults to .runmd
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
    PrintLifecycleGraph(Host),
    /// Prints the engine event graph,
    PrintEngineGraph(Host),
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
