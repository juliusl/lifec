use lifec::v2::*;
use reality::v2::{prelude::*, states::Object, thunk::DispatchbuildExt, WorldWrapper};

mod example;
use example::*;
use specs::Builder;

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
/// +           .setup      folder
/// +           .greet      df1
///
/// +           .host                   # Example usage of example engine and host root
/// : root_dir  .symbol     .test       # Root directory of this host
/// :
/// <engine>    .example    initial
/// :           .setup      folder
/// :           .greet      df1
/// :           .greet      df2
/// :           .greet      df3
/// <engine>    .example    cache
/// :           .setup      cache
/// :           .greet      df4
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

    compiler.as_mut().exec(|(examples, identifiers): (ExampleProvider, ReadStorage<Identifier>)|{
        for (e, i) in examples.state_vec::<ExampleInstance>().iter() {
            identifiers.get(i.entity).map(|i| {
                i.parent().map(|p| println!("parent -- {:#}", p));
            });
        }
    });

    let mut hosts = vec![];
    for (e, o) in compiler.compiled().state_vec::<Object>().iter() {
        if let Some(_) = o.ident().interpolate("#block#.#root#.host.(?name);") {
            hosts.push(*e);
        }
    }

    let host = Host::new();
    let mut newhosts = vec![];
    for h in hosts.iter() {
        let mut _host = compiler.dispatch_ref(*h);
        _host.store(host.clone())?;

        let host = host.dispatch(_host)?;

        let newhost = host.build()?;

        if let Some(e) = newhost.entity {
            newhosts.push(e);
        }
    }
    compiler.as_mut().maintain();

    println!("Processing new hosts");
    for h in newhosts.iter() {
        compiler.dispatch_ref(*h).cancel()?;
    }

    Ok(())
}

#[derive(Runmd, Component, Clone, Debug)]
#[storage(VecStorage)]
#[compile(Build, Cancel)]
pub struct Host {
    root_dir: String,
}

impl Host {
    const fn new() -> Self {
        Self {
            root_dir: String::new(),
        }
    }
}

impl Cancel for Host {
    fn cancel(&self, cancel_token: &mut lifec::v2::CancelToken) -> Result<()> {
        println!("Cancelling host --");
        cancel_token.cancel()
    }
}

impl Build for Host {
    fn build(&self, lazy_builder: LazyBuilder) -> Result<Entity> {
        Ok(lazy_builder
            .with(thunk_cancel(self.clone()))
            .with(CancelToken::new())
            .with(dispatch_cancel {})
            .build())
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
                    "reality::identifier=trace"
                        .parse()
                        .expect("should be able to parse tracing settings"),
                ),
        )
        .init();
}
