use lifec::{Project, Event, ThunkContext, Host, WorldExt};

use tracing_subscriber::EnvFilter;

/// Simple program for parsing runmd into a World
/// 
fn main() {
    tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .compact()
        .init();

    let mut host = Host::load_content::<Lifec>(r#"
    ``` containerd
    + .engine
    : .event test
    : .exit
    ```
    
    ``` test containerd
    :  src_dir  .symbol ./
    :  work_dir .symbol .work/acr

    + .runtime
    : .process  sh lib/sh/login-acr.sh
    :  REGISTRY_NAME .env obddemo

    : .install  access_token
    ```
    "#);

    let mut dispatcher = {
        let dispatcher = Host::dispatcher_builder();
        dispatcher.build()
    };
    dispatcher.setup(host.world_mut());
    
    // TODO - Turn this into an api
    let event = host.world().entities().entity(3);
    if let Some(event) = host.world().write_component::<Event>().get_mut(event) {
        event.fire(ThunkContext::default());
    }
    host.world_mut().maintain();

    while !host.should_exit() {
        dispatcher.dispatch(host.world());
    }
}

struct Lifec;

impl Project for Lifec {
    fn configure_engine(_engine: &mut lifec::Engine) {
        // No-op
    }

    fn interpret(_world: &lifec::World, _block: &lifec::Block) {
       // No-op
    }
}