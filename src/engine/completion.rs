use std::collections::BTreeMap;

use atlier::system::{Attribute, Value};
use reality::{Block, BlockProperties};
use specs::{Entity, Component, VecStorage};

/// Struct that contains the results of a single thunk completion,
///
#[derive(Component, Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
#[storage(VecStorage)]
pub struct Completion {
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

impl Into<Block> for Completion {
    fn into(self) -> Block {
        let mut block = Block::new(self.thunk, "", "completion");
        let mut attribute = Attribute::new(self.thunk.id(), format!("completion::event"), Value::Empty);
        attribute.edit_as(Value::Int(self.event.id() as i32));
        block.add_attribute(&attribute);

        let mut attribute = Attribute::new(self.thunk.id(), format!("completion::thunk"), Value::Empty);
        attribute.edit_as(Value::Int(self.thunk.id() as i32));
        block.add_attribute(&attribute);
        
        for (name, value) in self.control_values {
            let mut attribute =
                Attribute::new(self.thunk.id(), format!("completion::{name}"), Value::Empty);
            attribute.edit_as(value.clone());
            block.add_attribute(&attribute);
        }

        block.add_attribute(&Attribute::new(self.thunk.id(), "query", Value::Empty));
        for (name, query) in self.query.iter_properties() {
            match query {
                reality::BlockProperty::Single(value) => {
                    let mut attribute =
                        Attribute::new(self.thunk.id(), format!("query::{name}"), Value::Empty);
                    attribute.edit_as(value.clone());
                    block.add_attribute(&attribute);
                }
                reality::BlockProperty::List(values) => {
                    for value in values {
                        let mut attribute =
                            Attribute::new(self.thunk.id(), format!("query::{name}"), Value::Empty);
                        attribute.edit_as(value.clone());
                        block.add_attribute(&attribute);
                    }
                }
                _ => {}
            }
        }

        block.add_attribute(&Attribute::new(self.thunk.id(), "returns", Value::Empty));
        if let Some(returns) = self.returns {
            for (name, returns) in returns.iter_properties() {
                match returns {
                    reality::BlockProperty::Single(value) => {
                        let mut attribute = Attribute::new(
                            self.thunk.id(),
                            format!("returns::{name}"),
                            Value::Empty,
                        );
                        attribute.edit_as(value.clone());
                        block.add_attribute(&attribute);
                    }
                    reality::BlockProperty::List(values) => {
                        for value in values {
                            let mut attribute = Attribute::new(
                                self.thunk.id(),
                                format!("returns::{name}"),
                                Value::Empty,
                            );
                            attribute.edit_as(value.clone());
                            block.add_attribute(&attribute);
                        }
                    }
                    _ => {}
                }
            }
        }

        block
    }
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
        use atlier::system::Value;
        use crate::state::{AttributeGraph, AttributeIndex};
        use crate::host::Host;
        use reality::Block;
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
            let block: Block = test
                .completion
                .clone()
                .expect("should have a completion")
                .into();
            
            // Assert control values
            assert_eq!( block.map_control().get("event"), Some(&Value::Int(2)));
            assert_eq!( block.map_control().get("thunk"), Some(&Value::Int(3)));

            // Assert index count
            let indexes = block.index();
            assert_eq!(indexes.len(), 2);

            // Assert query root/index
            let query = indexes.iter().find(|i| i.root().name() == "query").expect("should have a query root");
            let query = AttributeGraph::new(query.clone());
            assert_eq!(query.find_symbol("println"), Some("hello world".to_string()));

            // Assert returns root/index
            let returns = indexes.iter().find(|i| i.root().name() == "returns").expect("should have a returns root");
            let returns = AttributeGraph::new(returns.clone());
            assert_eq!(returns.find_symbol("println"), Some("hello world".to_string()));
        }
    }
}
