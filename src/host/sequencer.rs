use std::collections::HashMap;

use crate::LifecycleOptions;
use crate::{Engine, Event, Host, Sequence};
use reality::Block;
use specs::{Entities, Entity, Join, ReadStorage, WriteStorage};
use tracing::event;
use tracing::Level;

/// Extension of Host to handle linking engine sequences together
///
pub trait Sequencer {
    /// Link event sequences for each engine
    ///
    fn link_sequences(&mut self);
}

impl Sequencer for Host {
    fn link_sequences(&mut self) {
        self.world_mut().exec(
            |(entities, blocks, _events, mut engines, sequences, mut lifecycle_options): (
                Entities,
                ReadStorage<Block>,
                ReadStorage<Event>,
                WriteStorage<Engine>,
                WriteStorage<Sequence>,
                WriteStorage<LifecycleOptions>,
            )| {
                let mut control_atlas = HashMap::<Entity, Vec<(Entity, Option<Entity>)>>::default();
                for (block, _, sequence) in (&blocks, &engines, &sequences).join() {
                    let control_entity = entities.entity(block.entity());
                    let mut atlas = vec![];
                    let mut stack = vec![];

                    for event_entity in sequence.iter_entities() {
                        let runtime_block = blocks.get(event_entity).expect("should exist");
                        for index in runtime_block
                            .index()
                            .iter()
                            .filter(|i| i.root().name() == "runtime")
                        {
                            for (plugin_entity, _) in index.iter_children() {
                                let plugin_entity = entities.entity(*plugin_entity);
                                if stack.is_empty() {
                                    stack.push(plugin_entity);
                                } else {
                                    if let Some(popped) = stack.pop() {
                                        atlas.push((popped, Some(plugin_entity)));
                                        stack.push(plugin_entity);
                                    }
                                }
                            }
                        }
                    }

                    if let Some(popped) = stack.pop() {
                        atlas.push((popped, None));
                    }
                    control_atlas.insert(control_entity, atlas);
                }

                for (control, atlas) in control_atlas.iter() {
                    let mut start = None;
                    if let Some((from, _)) = atlas.first() {
                        if start.is_none() {
                            start = Some(from);
                        }
                    }

                    let start = start.take().expect("Should have a start");
                    if let Some(engine) = engines.get_mut(*control) {
                        engine.set_start(*start);
                    }

                    if atlas.is_empty() {
                        continue;
                    }

                    let (from, to) = atlas.iter().last().expect("Should have a last entry");
                    let lifecycle_option = lifecycle_options
                        .get(*control)
                        .expect("Should have a lifecycle option");

                    if let Some(to) = to {
                        event!(
                            Level::DEBUG,
                            "Setting lifecycle_option, {:?} -> {:?}",
                            to,
                            lifecycle_option
                        );
                        lifecycle_options
                            .insert(*to, lifecycle_option.clone())
                            .expect("Should be able to insert");
                    } else {
                        event!(
                            Level::DEBUG,
                            "Setting lifecycle_option, {:?} -> {:?}",
                            from,
                            lifecycle_option
                        );
                        lifecycle_options
                            .insert(*from, lifecycle_option.clone())
                            .expect("Should be able to insert");
                    }
                }
            },
        );
    }
}
