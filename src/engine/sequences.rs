use std::collections::{HashMap, HashSet};

use reality::Block;
use specs::prelude::*;
use specs::{Entity, Join, Read, SystemData};

use super::{Adhoc, Connection, Cursor, Engine, Event, Limit, Profiler, Sequence};

/// System data for configuring sequences for engines and events,
///
#[derive(SystemData)]
pub struct Sequences<'a> {
    /// Entity map
    ///
    entity_map: Read<'a, HashMap<String, Entity>>,
    /// Entities,
    ///
    entities: Entities<'a>,
    /// Block storage,
    ///
    blocks: ReadStorage<'a, Block>,
    /// Engine storage,
    ///
    engines: ReadStorage<'a, Engine>,
    /// Event storage,
    ///
    events: ReadStorage<'a, Event>,
    /// Adhoc configurations,
    ///
    adhocs: ReadStorage<'a, Adhoc>,
    /// Profiler storage,
    ///
    profilers: WriteStorage<'a, Profiler>,
    /// Engine limits,
    ///
    limits: WriteStorage<'a, Limit>,
    /// Sequence storage,
    ///
    sequences: WriteStorage<'a, Sequence>,
    /// Connection storage,
    ///
    connections: WriteStorage<'a, Connection>,
    /// Cursor storage,
    ///
    cursors: WriteStorage<'a, Cursor>,
}

impl<'a> Sequences<'a> {
    /// Scan for engines,
    ///
    pub fn scan_engines(&self) -> Vec<Entity> {
        (&self.entities, &self.engines)
            .join()
            .map(|(e, _)| e)
            .collect()
    }

    /// Build event transitions for an engine,
    ///
    pub fn build_engine(&mut self, engine: Entity) {
        if let Some((block, engine)) = (&self.blocks, &self.engines)
            .join()
            .get(engine, &self.entities)
        {
            let transitions = engine.iter_transitions();

            for (from, to) in transitions.zip(engine.iter_transitions().skip(1)) {
                for t in
                    to.1.iter()
                        .filter_map(|f| self.entity_map.get(&format!("{f} {}", block.symbol())))
                {
                    let mut incoming = HashSet::<Entity>::default();

                    for f in from
                        .1
                        .iter()
                        .filter_map(|f| self.entity_map.get(&format!("{f} {}", block.symbol())))
                    {
                        incoming.insert(*f);
                    }
                    let connection = Connection::new(incoming, *t);
                    self.connections
                        .insert(*t, connection)
                        .expect("should be able to insert connection");
                }
            }
        }
    }

    /// Links event transitions for each engine and build connection components,
    ///
    pub fn build_engines(&mut self) {
        let engines = self.scan_engines();

        // Setup connections/transitions
        for engine in engines.iter() {
            self.build_engine(*engine);
        }

        self.setup_adhoc_profiler();

        // Process cursors
        for (_, connection) in (&self.entities, &self.connections).join() {
            for (from, to) in connection.connections() {
                if let Some(sequence) = self.sequences.get_mut(*from) {
                    sequence.set_cursor(*to);
                }
            }
        }

        // Unpack built cursors
        for (entity, _, sequence) in (&self.entities, &self.events, &self.sequences).join() {
            if let Some(cursor) = sequence.cursor() {
                self.cursors
                    .insert(entity, cursor.clone())
                    .expect("should be able to insert cursor");
            }
        }

        for engine in engines.iter() {
            self.configure_lifecycles(*engine);
        }
    }

    pub fn configure_lifecycles(&mut self, engine: Entity) {
        if let Some((engine, sequence)) = (&self.engines, &self.sequences)
            .join()
            .get(engine, &self.entities)
        {
            if let Some(last) = sequence.last() {
                if let Some(cursor) = sequence.cursor().cloned() {
                    // Translate engine cursors into events
                    let cursor = match cursor {
                        Cursor::Next(next) => {
                            let engine = self.engines.get(next).expect("should have an engine");
                            let start = engine.start().expect("should have a start");
                            Cursor::Next(*start)
                        }
                        Cursor::Fork(forks) => Cursor::Fork(
                            forks
                                .iter()
                                .filter_map(|f| self.engines.get(*f))
                                .filter_map(|e| e.start())
                                .cloned()
                                .collect(),
                        ),
                    };

                    // Assign limits
                    if let Some(limit) = engine.limit() {
                        match &cursor {
                            Cursor::Next(next) => {
                                self.limits
                                    .insert(*next, limit.clone())
                                    .expect("should be able to insert limit");
                            }
                            Cursor::Fork(forks) => {
                                for fork in forks {
                                    self.limits
                                        .insert(*fork, limit.clone())
                                        .expect("should be able to insert limit");
                                }
                            }
                        }
                    }

                    match &cursor {
                        Cursor::Next(next) => {
                            if let Some(connection) = self.connections.get_mut(*next) {
                                connection.add_incoming(last);
                            } else {
                                let mut from = HashSet::new();
                                from.insert(last);
                                let connection = Connection::new(from, *next);
                                self.connections
                                    .insert(*next, connection)
                                    .expect("should be able to insert connection");
                            }
                        }
                        Cursor::Fork(forks) => {
                            for fork in forks.iter() {
                                if let Some(connection) = self.connections.get_mut(*fork) {
                                    connection.add_incoming(last);
                                } else {
                                    let mut from = HashSet::new();
                                    from.insert(last);
                                    let connection = Connection::new(from, *fork);
                                    self.connections
                                        .insert(*fork, connection)
                                        .expect("should be able to insert connection");
                                }
                            }
                        }
                    }

                    self.cursors
                        .insert(last, cursor)
                        .expect("should be able to insert cursor");
                }
            }
        }
    }

    /// Setup adhoc profiler connections,
    ///
    pub fn setup_adhoc_profiler(&mut self) {
        let Sequences {
            entities,
            profilers,
            connections,
            ..
        } = self;
        // Unpack adhoc operations, link to profiler
        // Since adhoc operations are not part of an engine, they need a connection so
        // that activity can be measured.
        let adhoc_profiler = entities.create();
        profilers
            .insert(adhoc_profiler, Profiler::default())
            .expect("should be able to insert component");

        let mut profiler_connections = HashSet::<Entity>::default();

        for (entity, _, _) in (&self.entities, &self.adhocs, &self.events).join() {
            profiler_connections.insert(entity);
        }
        connections
            .insert(
                adhoc_profiler,
                Connection::new(profiler_connections, adhoc_profiler),
            )
            .expect("should be able to insert connection");
    }
}
