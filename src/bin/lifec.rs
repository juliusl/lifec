use std::{collections::HashMap, ops::Deref};

use lifec::{
    Block, Engine, Entity, Event, Host, Join, LifecycleOptions, Project, ReadStorage, Sequence,
    ThunkContext, WorldExt, BlockIndex,
};

use specs::Read;
use tracing_subscriber::EnvFilter;

/// Simple program for parsing runmd into a World
///
fn main() {
    tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .compact()
        .init();

    let mut host = Host::load_content::<Lifec>(
        r#"
    ``` test_block1
    + .engine
    : .event test
    : .event test azure
    : .fork test_block2, azure
    ```

    ``` test test_block1
    + .runtime
    : .println test_block1
    ```

    ``` test_block2
    + .engine
    : .event test
    : .repeat 5
    ```

    ``` test test_block2
    + .runtime
    : .println test_block2
    ```

    ``` azure
    + .engine 
    : .event test
    : .next containerd
    ```

    ``` test azure
    + .runtime
    : .println testing
    ```

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
    "#,
    );

    // Print lifecycle options
    //
    host.world_mut().exec(
        |(options, blocks): (Read<HashMap<Entity, LifecycleOptions>>, ReadStorage<Block>)| {
            for (e, option) in options.iter() {
                if let Some(block) = blocks.get(*e) {
                    let mut block_name = block.name().to_string();
                    if block_name.is_empty() {
                        block_name = "```".to_string();
                    }
                    println!(
                        "Engine control block: {} {} @ {:?}",
                        block_name,
                        block.symbol(),
                        e
                    );
                    println!("  {:?}", option);
                    println!("");
                }
            }
        },
    );

    host.world_mut().exec(
        |(blocks, engines, sequences): (
            ReadStorage<Block>,
            ReadStorage<Engine>,
            ReadStorage<Sequence>,
        )| {
            for (block, _, sequence) in (&blocks, &engines, &sequences).join() {
                let mut block_name = block.name().to_string();
                if block_name.is_empty() {
                    block_name = "```".to_string();
                }
                println!("Engine events: {} {}", block_name, block.symbol());
                for e in sequence.iter_entities() {
                    let runtime_block = blocks.get(e).expect("should exist");
                    println!("```{} {}", runtime_block.name(), runtime_block.symbol());
                    for index in runtime_block.index().iter().filter(|i| i.root().name() == "runtime") {
                        for (e, props) in index.iter_children() {
                            println!("\tplugin event - {e}");
                            for (name, prop) in props.iter_properties(){
                                println!("\t{name}{:?}", prop);
                            }
                            println!();
                        }
                    }
                }
                println!("");
            }
        },
    );

    // host.world_mut().exec(
    //     |blocks: Read<HashMap<String, Entity>>| {
    //         eprintln!("{:#?}", blocks.deref()); 
    //     },
    // );

    // let mut dispatcher = {
    //     let dispatcher = Host::dispatcher_builder();
    //     dispatcher.build()
    // };
    // dispatcher.setup(host.world_mut());

    // // TODO - Turn this into an api

    // // -- Ex
    // /*
    //     host.start_engine("containerd').await;

    // */
    // let event = host.world().entities().entity(3);
    // if let Some(event) = host.world().write_component::<Event>().get_mut(event) {
    //     event.fire(ThunkContext::default());
    // }
    // host.world_mut().maintain();

    // // TODO, typically you would use an event loop here,
    // while !host.should_exit() {
    //     dispatcher.dispatch(host.world());
    // }
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
