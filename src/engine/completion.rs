use std::collections::BTreeMap;

use reality::{Attribute, Value};
use chrono::{Utc, DateTime};
use reality::{Block, BlockProperties};
use specs::{Entity, Component, VecStorage};

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

mod tests {
    use super::Completion;
    use crate::prelude::{Listener, Project};

    #[derive(Default, Debug)]
    struct Test {
        completion: Option<Completion>,
    }

    impl Project for Test {
        fn interpret(_: &specs::World, _: &reality::Block) {}
    }

    impl Listener for Test {
        fn create(_: &specs::World) -> Self {
            Test { completion: None }
        }
        fn on_status_update(&mut self, _: &crate::prelude::StatusUpdate) {}
        fn on_operation(&mut self, _: crate::prelude::Operation) {}
        fn on_completion(&mut self, completion: super::Completion) {
            self.completion = Some(completion);
        }
        fn on_error_context(&mut self, _: &crate::prelude::ErrorContext) {}
        fn on_completed_event(&mut self, _: &specs::Entity) {}
    }

    #[test]
    fn test() {
        use crate::host::Host;
        use std::ops::Deref;

        let mut host = Host::load_content::<Test>(
            r#"
        ``` test
        + .engine
        : .start a
        : .exit
        ```

        ``` a test
        + .runtime
        : .println hello world
        ```
        "#,
        );
        host.enable_listener::<Test>();
        host.start_with::<Test>("test");

        let test = host.world().fetch::<Option<Test>>();
        if let Some(test) = test.deref().clone() {
            assert!(test.completion.is_some());
            let completion = test.completion.clone().unwrap();

            let returns = completion.returns.unwrap();
            let println = returns.property("println").unwrap();
            assert_eq!(println.symbol(), Some(&String::from("hello world")));
        }
    }
}
