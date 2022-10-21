use crate::LifecycleOptions;
use crate::{Engine, Event, Host, Sequence};
use reality::Block;
use specs::{Entities, Join, ReadStorage, WriteStorage};
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
            |(_entities, blocks, _events, engines, mut sequences, _lifecycle_options): (
                Entities,
                ReadStorage<Block>,
                ReadStorage<Event>,
                WriteStorage<Engine>,
                WriteStorage<Sequence>,
                WriteStorage<LifecycleOptions>,
            )| {
                let mut links = vec![];

                // Process engines
                for (_, _, sequence) in (&blocks, &engines, &sequences).join() {
                    let events = sequence.iter_entities(); 
                    for (from, to) in events.zip(sequence.iter_entities().skip(1)) {
                        event!(Level::TRACE, "Linking event {} -> {}", from.id(), to.id());
                        links.push((from, to));
                    }
                }

                for (from, to) in links {
                    let peek = sequences.get(to).clone().and_then(|s| s.peek());

                    match (sequences.get_mut(from), peek) {
                        (Some(from), Some(to)) => {
                            from.set_cursor(to);
                        },
                        _ => {
                        }
                    }
                }
            },
        );
    }
}

mod test {
    use crate::Project;

    struct Test;
    impl Project for Test {
        fn interpret(_: &specs::World, _: &reality::Block) {
            // no-op
        }
    }

    #[test]
    #[tracing_test::traced_test]
    fn test_sequencer() {
        let _ = crate::Host::load_content::<Test>(r#"
        ``` test
        + .engine
        : .event step1
        : .event step2
        : .event step3
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
        : .event step1
        : .event step2
        : .event step3
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
        "#);
    }
}
