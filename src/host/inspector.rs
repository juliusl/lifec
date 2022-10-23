use std::collections::HashMap;

use reality::Block;
use specs::{Entities, Entity, Join, Read, ReadStorage};

use crate::{Engine, Host, Sequence, Thunk};

/// Extension methods for inspecting World state after the world is done building,
///
pub trait Inspector {
    /// Prints the lifecycle graph
    ///
    fn print_lifecycle_graph(&mut self);

    /// Prints the engine event graph
    ///
    fn print_engine_event_graph(&mut self);
}

impl Inspector for Host {
    fn print_lifecycle_graph(&mut self) {
        // self.world_mut().exec(
        //     |blocks: ReadStorage<Block>| {
        //         for (e, option) in options.iter() {
        //             if let Some(block) = blocks.get(*e) {
        //                 let mut block_name = block.name().to_string();
        //                 if block_name.is_empty() {
        //                     block_name = "```".to_string();
        //                 }
        //                 println!(
        //                     "Engine control block: {} {} @ {:?}",
        //                     block_name,
        //                     block.symbol(),
        //                     e
        //                 );
        //                 println!("  {:?}", option);
        //                 println!("");
        //             }
        //         }
        //     },
        // );
    }

    fn print_engine_event_graph(&mut self) {
        self.world_mut().exec(
            |(entities, blocks, engines, sequences, thunks): (
                Entities,
                ReadStorage<Block>,
                ReadStorage<Engine>,
                ReadStorage<Sequence>,
                ReadStorage<Thunk>,
            )| {
                for (block, _, sequence) in (&blocks, &engines, &sequences).join() {
                    println!("{}: {}", block.entity(), block.symbol());
                    for e in sequence.iter_entities() {
                        let runtime_block = blocks.get(e).expect("should exist");
                        println!(
                            "  {}: {} {}",
                            e.id(),
                            runtime_block.name(),
                            runtime_block.symbol()
                        );
                        let control_values = runtime_block.map_control();
                        if !control_values.is_empty() {
                            control_values.iter().for_each(|(name, value)| {
                                println!("\t# {name}: {value}");
                            });
                        }
                        for index in runtime_block
                            .index()
                            .iter()
                            .filter(|i| i.root().name() == "runtime")
                        {
                            for (e, props) in index.iter_children() {
                                let event = entities.entity(*e);
                                let thunk = thunks.get(event).expect("should be an event");
                                let thunk = thunk.0;
                                println!(
                                    "    {e}: {} {:?}",
                                    thunk,
                                    props
                                        .property(thunk)
                                        .expect("should be a symbol")
                                        .symbol()
                                        .unwrap()
                                );
                                for (name, prop) in props.iter_properties().filter(|p| p.0 != thunk) {
                                    println!("      {name} {:?}", prop);
                                }
                                println!();
                            }
                        }
                        println!();
                    }
                    println!();
                }
            },
        );
    }
}
