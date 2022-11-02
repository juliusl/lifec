use crate::{
    engine::Sequences,
    prelude::*,
};

/// Extension of Host to handle linking engine sequences together
///
pub trait Sequencer {
    /// Link event sequences,
    ///
    fn link_sequences(&mut self);
}

impl Sequencer for Host {
    fn link_sequences(&mut self) {
        self.world_mut().exec( |mut sequences: Sequences | {
            // Build engine events and setup connections between them
            // Process engine lifecycles and connect connections between them
            sequences.build_engines();

            // Setup profilers for each adhoc operations
            sequences.setup_adhoc_profiler();
        });
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
