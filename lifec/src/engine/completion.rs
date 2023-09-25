use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use reality::{BlockProperties, Value};
use specs::{Component, Entity, VecStorage};

/// Struct that contains the results of a single thunk completion,
///
#[derive(Component, Hash, Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
#[storage(VecStorage)]
pub struct Completion {
    /// Timestamp,
    ///
    pub timestamp: DateTime<Utc>,
    /// Event entity that initiated this completion,
    ///
    /// If this is a completion from a spawned event, then its possible that the spawned event was cleaned up,
    ///
    pub event: Entity,
    /// Thunk entity that owns this completion,
    ///
    /// The thunk entity will have all of the relevant state components
    ///
    pub thunk: Entity,
    /// Control value state,
    ///
    pub control_values: BTreeMap<String, Value>,
    /// Block object query that resulted in this completion,
    ///
    /// When a plugin implements BlockObject, it can declare a query that represents the properties
    /// it will look for during it's call. These are the properties that were used.
    ///
    pub query: BlockProperties,
    /// Block object return that was the result of this completion,
    ///
    /// A BlockObject may optionally declare a set of block properties that might be committed to state.
    ///
    pub returns: Option<BlockProperties>,
}

#[allow(dead_code)]
#[allow(unused_imports)]
mod tests {
    use reality::{BlockObject, BlockProperties};
    use tracing::trace;

    use super::Completion;
    use crate::{prelude::{Listener, Project}, project::default_runtime, plugins::Plugin, error::Error};

    #[derive(Default, Debug)]
    struct Test {
        completion: Option<Completion>,
    }

    #[derive(Default)]
    struct Skip;

    impl Plugin for Skip {
        fn symbol() -> &'static str {
            "skip"
        }

        fn call(context: &mut crate::plugins::ThunkContext) -> Option<crate::plugins::AsyncContext> {
            context.task_with_result(|_| {
                async {
                    Err(Error::skip("testing skip"))
                }
            })
        }
    }

    impl BlockObject for Skip {
        fn query(&self) -> reality::BlockProperties {
            BlockProperties::default()
        }

        fn parser(&self) -> Option<reality::CustomAttribute> {
            Some(Self::as_custom_attr())
        }
    }

    impl Project for Test {
        fn runtime() -> crate::runtime::Runtime {
            let mut runtime = default_runtime();
            runtime.install_with_custom::<Skip>("");
            runtime
        }

        fn interpret(_: &specs::World, _: &reality::Block) {}
    }

    impl Listener for Test {
        fn create(_: &specs::World) -> Self {
            Test { completion: None }
        }
        fn on_operation(&mut self, _: crate::prelude::Operation) {}
        fn on_completion(&mut self, completion: super::Completion) {
            if self.completion.is_none() {
                self.completion = Some(completion);
            }
        }
        fn on_completed_event(&mut self, _: &specs::Entity) {}
    }

    #[test]
    #[tracing_test::traced_test]
    fn test() {
        use crate::host::Host;
        use std::ops::Deref;

        let mut host = Host::load_content::<Test>(
            r#"
        ``` test
        + .engine
        : .start a
        : .start b
        : .exit
        ```

        ``` a test
        + .runtime
        : .println hello world
        ```

        ``` b test
        + .runtime
        : .skip
        : .println testing skipped
        : .println testing skipped 2
        ```
        "#,
        );
        host.enable_listener::<Test>();
        host.start_with::<Test>("test");

        let test = host.world().fetch::<Option<Test>>();
        if let Some(test) = test.clone().deref() {
            assert!(test.completion.is_some());
            let completion = test.completion.clone().unwrap();

            let returns = completion.returns.unwrap();
            let println = returns.property("println").unwrap();
            assert_eq!(println.symbol(), Some(&String::from("hello world")));
        }

        logs_assert(|lines| {
            let count = lines.iter().filter(|l| l.contains("testing skip")).count();
            assert!(count > 0, "didn't find any skip lines");

            eprintln!("{count}");
            Ok(())
        });
    }
}
