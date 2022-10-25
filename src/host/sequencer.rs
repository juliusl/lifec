use std::collections::{HashMap, HashSet};

use crate::prelude::*;

/// Extension of Host to handle linking engine sequences together
///
pub trait Sequencer {
    /// Link event sequences,
    ///
    fn link_sequences(&mut self);
}

impl Sequencer for Host {
    fn link_sequences(&mut self) {
        self.world_mut().exec(
            |(
                block_map,
                entities,
                blocks,
                engines,
                events,
                mut limits,
                mut sequences,
                mut connections,
                mut cursors,
            ): (
                Read<HashMap<String, Entity>>,
                Entities,
                ReadStorage<Block>,
                ReadStorage<Engine>,
                ReadStorage<Event>,
                WriteStorage<Limit>,
                WriteStorage<Sequence>,
                WriteStorage<Connection>,
                WriteStorage<Cursor>,
            )| {
                // Process engines
                for (block, engine) in (&blocks, &engines).join() {
                    let transitions = engine.iter_transitions();

                    for (from, to) in transitions.zip(engine.iter_transitions().skip(1)) {
                        for t in
                            to.1.iter()
                                .filter_map(|f| block_map.get(&format!("{f} {}", block.symbol())))
                        {
                            let mut incoming = HashSet::<Entity>::default();

                            for f in from
                                .1
                                .iter()
                                .filter_map(|f| block_map.get(&format!("{f} {}", block.symbol())))
                            {
                                incoming.insert(*f);
                            }
                            let connection = Connection::new(incoming, *t);
                            connections
                                .insert(*t, connection)
                                .expect("should be able to insert connection");
                        }
                    }
                }

                // Process cursors
                for (_, connection) in (&entities, &connections).join() {
                    for (from, to) in connection.connections() {
                        if let Some(sequence) = sequences.get_mut(*from) {
                            sequence.set_cursor(*to);
                        }
                    }
                }

                // Unpack built cursors
                for (entity, _, sequence) in (&entities, &events, &sequences).join() {
                    if let Some(cursor) = sequence.cursor() {
                        cursors
                            .insert(entity, cursor.clone())
                            .expect("should be able to insert cursor");
                    }
                }

                // Unpack lifecycle cursor to link engines
                for (engine, sequence) in (&engines, &sequences).join() {
                    if let Some(last) = sequence.last() {
                        if let Some(cursor) = sequence.cursor().cloned() {
                            // Translate cursor into events
                            let cursor = match cursor {
                                Cursor::Next(next) => {
                                    let engine = engines.get(next).expect("should have an engine");
                                    let start = engine.start().expect("should have a start");
                                    Cursor::Next(*start)
                                }
                                Cursor::Fork(forks) => Cursor::Fork(
                                    forks
                                        .iter()
                                        .filter_map(|f| engines.get(*f))
                                        .filter_map(|e| e.start())
                                        .cloned()
                                        .collect(),
                                ),
                            };

                            // Assign limits
                            if let Some(limit) = engine.limit() {
                                match &cursor {
                                    Cursor::Next(next) => {
                                        limits
                                            .insert(*next, limit.clone())
                                            .expect("should be able to insert limit");
                                    }
                                    Cursor::Fork(forks) => {
                                        for fork in forks {
                                            limits
                                                .insert(*fork, limit.clone())
                                                .expect("should be able to insert limit");
                                        }
                                    }
                                }
                            }

                            cursors
                                .insert(last, cursor)
                                .expect("should be able to insert cursor");
                        }
                    }
                }
            },
        );
    }
}

mod test {
    use crate::prelude::Project;

    #[derive(Default)]
    struct Test;
    
    impl Project for Test {
        fn interpret(_: &specs::World, _: &reality::Block) {
            // no-op
        }
    }

    #[test]
    #[tracing_test::traced_test]
    fn test_sequencer() {
        let _ = crate::prelude::Host::load_content::<Test>(
            r#"
        ``` test
        + .engine
        : .start step1
        : .start step2
        : .start step3
        : .next test2
        ```
    
        ``` step1 test 
        + .runtime
        : .println abc
        : .println def
        : .println ghi
        ```
    
        ``` step2 test
        + .runtime
        : .println 2 abc
        : .println 2 def
        : .println 2 ghi
        ```
    
        ``` step3 test
        + .runtime
        : .println 3 abc
        : .println 3 def
        : .println 3 ghi
        ```

        ``` test2
        + .engine
        : .start step1
        : .start step2
        : .start step3
        : .exit
        ```

        ``` step1 test2
        + .runtime
        : .println test2 test
        ```

        ``` step2 test2
        + .runtime
        : .println test2 test2
        ```

        ``` step3 test2 
        + .runtime
        : .println test2 test3
        ```
        "#,
        );
    }
}
