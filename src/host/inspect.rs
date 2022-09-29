use std::collections::HashMap;

use reality::Block;
use specs::{Read, Entity, ReadStorage, Join};

use crate::{Host, LifecycleOptions, Engine, Sequence};

/// Extension methods for inspecting World state after the world is done building,
/// 
pub trait InspectExtensions {
    /// Prints the lifecycle graph
    /// 
    fn print_lifecycle_graph(&mut self);

    /// Prints the engine event graph
    /// 
    fn print_engine_event_graph(&mut self);
}

impl InspectExtensions for Host {
    fn print_lifecycle_graph(&mut self) {
        self.world_mut().exec(
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
    }

    fn print_engine_event_graph(&mut self) {
        self.world_mut().exec(
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
                        println!("Engine control block: {} {}", block_name, block.symbol());
                    for e in sequence.iter_entities() {
                        let runtime_block = blocks.get(e).expect("should exist");
                        println!("Event block:          {} {}", runtime_block.name(), runtime_block.symbol());
                        println!("Events:");
                        for index in runtime_block.index().iter().filter(|i| i.root().name() == "runtime") {
                            for (e, props) in index.iter_children() {
                                    println!("\tentity-id: {e}");
                                for (name, prop) in props.iter_properties(){
                                    println!("\tproperty:  {name}");
                                    println!("\tvalue:     {:?}", prop);
                                    println!();
                                }
                                println!();
                            }
                        }
                    }
                    println!("");
                }
            },
        );
    }
}