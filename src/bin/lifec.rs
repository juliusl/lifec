use clap::{Parser, Subcommand};
use lifec::prelude::*;

use tracing_subscriber::EnvFilter;

/// Simple program for parsing runmd into a World
///
#[tokio::main]
async fn main() {
    let cli = Lifec::parse();

    tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter(if !cli.debug {
            EnvFilter::builder()
                .with_default_directive("lifec=info".parse().expect("should parse"))
                .from_env()
                .expect("should work")
        } else {
            EnvFilter::builder()
                .with_default_directive("lifec=debug".parse().expect("should parse"))
                .from_env()
                .expect("should work")
        })
        .compact()
        .init();

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
                }
                (None, Some(url)) => {
                    host.set_url(url);
                }
                _ => {}
            }

            match c {
                Commands::Start(start) => {
                    host.set_command(lifec::host::Commands::Start(start));
                    let mut host = host
                        .create_host::<Lifec>()
                        .await
                        .expect("Should be able to create host");
                    host.handle_start::<Lifec>();
                }
                Commands::Host(Host {
                    workspace: Some(workspace),
                    command: Some(lifec::host::Commands::Open),
                    ..
                }) => {
                    host.workspace = Some(workspace);
                    let host = host
                        .create_host::<Lifec>()
                        .await
                        .expect("Should be able to create host");
                    tokio::task::block_in_place(move || {
                        host.open_runtime_editor::<Lifec>();
                    });
                }
                Commands::Host(host) => {
                    let mut host = host
                        .create_host::<Lifec>()
                        .await
                        .expect("Should be able to create host");
                    host.handle_start::<Lifec>();
                }
                Commands::PrintEngines => {
                    let mut host = host
                        .create_host::<Lifec>()
                        .await
                        .expect("Should be able to create host");
                    host.print_engine_event_graph();
                }
                Commands::PrintLifecycle => {
                    let mut host = host
                        .create_host::<Lifec>()
                        .await
                        .expect("Should be able to create host");
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
#[derive(Debug, Parser, Default)]
#[clap(name = "lifec")]
#[clap(arg_required_else_help = true)]
#[clap(
    about = "Utilities for working with the World created by lifec, limited to process, install, println, timer plugins"
)]
struct Lifec {
    /// URL to runmd to fetch to create host
    #[clap(long)]
    url: Option<String>,
    /// Path to runmd file to create host
    #[clap(long)]
    runmd_path: Option<String>,
    /// Turns on debug logging
    #[clap(long, action)]
    debug: bool,
    #[clap(subcommand)]
    command: Option<Commands>,
}

/// Enumeration of commands,
///
#[derive(Debug, Subcommand)]
enum Commands {
    /// Prints the lifecycle graph,
    PrintLifecycle,
    /// Prints the engine event graph,
    PrintEngines,
    /// Host commands,
    Host(Host),
    /// Shortcut for `host start` command,
    Start(Start),
}

impl Project for Lifec {
    fn interpret(_world: &World, _block: &Block) {
        // No-op
    }
}
