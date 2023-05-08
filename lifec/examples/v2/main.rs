use reality::v2::prelude::*;

mod example;
use example::*;

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
/// <engine>    .example    initial
/// :           .setup      folder
/// :           .greet      world
/// :           .greet      world 2
/// :           .greet      world 3
/// <engine>    .example    cache
/// :           .setup      cache
/// :           .greet      world 4
/// ```
/// 
#[reality::parse_docs]
#[tokio::main]
async fn main() -> Result<()> {
    // enable_logging();
    
    let mut compiler = Compiler::new().with_docs();

    let _ = lifec::v2::compile(&mut compiler)?;
    let _ = compile_runmd_main(&mut compiler)?;

    export_toml(&mut compiler, ".test/lifec-example.toml").await?;

    compiler.link(Example::new())?;

    compiler.as_mut().exec(|examples: ExampleProvider|{
        for (e, _) in examples.state_vec::<ExampleInstance>().iter() {
            println!("{:?}", e);
        }
    });

    Ok(())
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
