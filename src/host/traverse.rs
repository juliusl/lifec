use std::collections::HashSet;

use specs::{prelude::*, Entities, Entity, ReadStorage, SystemData, World, WorldExt};

use crate::{Event, Host, LifecycleOptions, Sequence, Source};

pub trait Traverse {
    /// Traverse a graph of events and transitions starting from an engine,
    ///
    /// Really only relevant when running w/ the event_runtime,
    ///
    /// *Note* - the order of events is not guranteed
    ///
    fn traverse_events(&mut self, start: Entity, visit: impl Fn(&mut World, Entity));
}

impl Traverse for Host {
    fn traverse_events(&mut self, start: Entity, visit: impl Fn(&mut World, Entity)) {
        let mut events = HashSet::<Entity>::default();

        let traversal = self.world().system_data::<TraversalComponents>();

        if let Some((seq, next_seqs)) = traversal.start(start) {
            let entities = vec![seq];
            let entities = entities
                .iter()
                .chain(next_seqs.iter())
                .flat_map(|s| s.iter_entities());

            for e in entities {
                events.insert(e);
            }
        }
    }
}

/// Struct for components needed for event graph traversal
///
#[derive(SystemData)]
struct TraversalComponents<'a> {
    sequences: ReadStorage<'a, Sequence>,
    lifecycle_option: ReadStorage<'a, LifecycleOptions>,
}

impl<'a> TraversalComponents<'a> {
    /// Starting at an entity, returns the sequence and the next sequences that would be executed after this one,
    ///
    pub fn start(&self, entity: Entity) -> Option<(Sequence, Vec<Sequence>)> {
        if let Some(sequence) = self.sequences.get(entity) {
            let mut next_seq = vec![];
            if let Some(last) = sequence.last().and_then(|l| self.lifecycle_option.get(l)) {
                match last {
                    LifecycleOptions::Fork(forks) => {
                        for fork in forks.iter().filter_map(|f| self.sequences.get(*f)) {
                            next_seq.push(fork.clone());
                        }
                    }
                    LifecycleOptions::Next(next) => {
                        if let Some(sequence) = self.sequences.get(*next) {
                            next_seq.push(sequence.clone());
                        }
                    }
                    _ => {
                        // no-op
                    }
                }
            }

            if let Some(cursor) = sequence.cursor().and_then(|s| self.sequences.get(s)) {
                next_seq.push(cursor.clone());
            }

            Some((sequence.clone(), next_seq))
        } else {
            None
        }
    }
}
