use std::path::PathBuf;

use clap::{Parser, Subcommand};
use lifec::{host::HostSettings, prelude::*};

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

    match &cli {
        Lifec {
            command: Some(c), ..
        } => match c {
            // Examples
            // lifec --workspace <workspace-uri> start
            // lifec --runmd_path <path to runmd file> start
            // lifec --url <url to runmd content> start
            // lifec start
            Commands::Start(start) => {
                let mut host_settings = cli.host_settings();
                host_settings.set_command(lifec::host::Commands::Start(start.clone()));
                host_settings.handle::<Lifec>().await;
            }
            Commands::Open => {
                let mut host_settings = cli.host_settings();
                host_settings.set_command(lifec::host::Commands::Open);
                host_settings.handle::<Lifec>().await;
            }
            // Examples
            // lifec host -- ..
            Commands::Host(settings) => {
                settings.handle::<Lifec>().await;
            }
            Commands::PrintEngines => {
                let host_settings = cli.host_settings();
                let mut host = host_settings
                    .create_host::<Lifec>()
                    .await
                    .expect("Should be able to create host");
                host.print_engine_event_graph();
            }
            Commands::PrintLifecycle => {
                let host_settings = cli.host_settings();
                let mut host = host_settings
                    .create_host::<Lifec>()
                    .await
                    .expect("Should be able to create host");
                host.print_lifecycle_graph();
            }
        },
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
    /// Root directory, defaults to current directory,
    ///
    #[clap(long)]
    pub root: Option<PathBuf>,
    /// URL to runmd content used to configure this host,
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
    /// Turns on debug logging
    #[clap(long, action)]
    debug: bool,
    #[clap(subcommand)]
    command: Option<Commands>,
}

impl Lifec {
    /// Converts top level arguments into host settings,
    ///
    pub fn host_settings(&self) -> HostSettings {
        let Self {
            root,
            url,
            runmd_path,
            workspace,
            ..
        } = self;

        let mut host_settings = HostSettings::default();
        match (workspace, runmd_path, url) {
            (Some(workspace), _, _) => {
                host_settings.set_workspace(workspace);
            }
            (_, Some(runmd_path), _) => {
                host_settings.set_path(runmd_path);
            }
            (_, _, Some(url)) => {
                host_settings.set_url(url);
            }
            _ => {}
        }

        if root.is_some() {
            host_settings.root = root.clone();
        }
        host_settings
    }
}

/// Enumeration of commands,
///
#[derive(Debug, Subcommand)]
enum Commands {
    /// Prints the lifecycle graph,
    PrintLifecycle,
    /// Prints the engine event graph,
    PrintEngines,
    /// `host` subcommand,
    Host(HostSettings),
    /// Shortcut for `host start` command,
    Start(Start),
    /// Shortcut for `host open` command,
    Open,
}

impl Project for Lifec {
    fn interpret(_world: &World, _block: &Block) {
        // No-op
    }
}
