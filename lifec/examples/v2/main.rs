use lifec::v2::Engine;
use reality::v2::prelude::*;

/// Example using v2 compiler with lifec extensions,
/// 
/// ```runmd
/// +       .engine Example             # Example component that uses the engine root
/// setup   .symbol                     # This property should be an identifier for a "setup" event type
/// greet   .symbol                     # This property should be an identifier for a "greet" event type
/// <once>  .setup                      # Indicates that the setup event type can only be started once
/// <start> .greet                      # Indicates that the greet event type can start immediately
/// :       .after  setup               # Schedules greet event types to execute after "setup" event types
/// ```
/// 
/// ```runmd test
/// +           .host                   # Example usage of example engine
/// <engine>    .example    
/// :           .setup      folder
/// :           .greet      world
/// :           .greet      world 2
/// :           .greet      world 3
/// ```
/// 
#[reality::parse_docs]
#[tokio::main]
async fn main() -> Result<()> {
    enable_logging();
    
    let mut compiler = Compiler::new().with_docs();

    let _ = lifec::v2::compile_runmd_engine(&mut compiler)?;
    let _ = compile_runmd_main(&mut compiler)?;

    export_toml(&mut compiler, ".test/lifec-example.toml").await?;

    compiler.link(Example::new())?;

    Ok(())
}

/// Example of a component that uses the engine extensions,
/// 
#[derive(Runmd, Debug, Component, Clone)]
#[storage(VecStorage)]
pub struct Example {
    /// List of identifiers to entities that are "setup" event types
    /// 
    #[config(ext=engine.once)]
    setup: Vec<String>,
    /// List of identifiers to entities that are "greet" event types
    /// 
    #[config(ext=engine.start)]
    greet: Vec<String>,
    /// Extensions for managing engine behavior
    /// 
    #[ext]
    engine: Engine,
}

impl Example {
    /// Returns a new example component,
    /// 
    fn new() -> Self {
        Self { setup: vec![], greet: vec![], engine: Engine::new() }
    }
}

/// Enables logging
///
#[allow(dead_code)]
fn enable_logging() {
    use tracing_subscriber::EnvFilter;
    tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter(
            EnvFilter::builder()
                .from_env()
                .expect("should be able to build from env variables")
                .add_directive(
                    "reality::v2=trace"
                        .parse()
                        .expect("should be able to parse tracing settings"),
                ),
        )
        .init();
}
