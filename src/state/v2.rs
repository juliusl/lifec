use crate::{plugins::{ThunkContext, AsyncContext}, Item};
use atlier::system::Value;
use specs::{Component, DefaultVecStorage, Entity};
use tokio::{sync::oneshot::Receiver, select};
use tracing::{event, Level};

mod attribute_index;
pub use attribute_index::AttributeIndex;

mod query;
pub use query::Query;

pub mod protocol;

/// [V2 Concept] - An operation encapsulates an async task and it's context
/// Where the result of the task is the next version of the context.
/// 
/// If a task exists, a join handle, and a oneshot that can be used to signal 
/// cancellation will be provided.
/// 
/// The fields of the operation are also the elements of executing an Event w/ 
/// an Engine/Plugin.
/// 
/// This component also implements, Item, Into<ThunkContext>, Clone so it can be used w/ 
/// Query<I>::thunk() as the item implementation. This allows operation to be a good starting point
/// for Systems using CatalogReader/CatalogWriter. Also, since thunk context can also be used as a src, 
/// operations can also transform into an attribute index.
/// 
/// Although, Operation implements Clone, it will not try to clone the underlying task if one exists.
/// This is useful for introspection on the initial_context used w/ an existing task.
/// 
#[derive(Component, Default)]
#[storage(DefaultVecStorage)]
pub struct Operation {
    pub context: ThunkContext,
    pub task: Option<AsyncContext>,
}

impl Clone for Operation {
    fn clone(&self) -> Self {
        Self { context: self.context.clone(), task: None }
    }
}

impl Operation {
    /// Returns a new empty operation w/o an existing task which can be used 
    /// as an Item implementation
    /// 
    pub fn item(entity: Entity, handle: tokio::runtime::Handle) -> Self {
        let tc = ThunkContext::default();
        let context = tc.enable_async(entity, handle, None, None, None, None);
        Self { context, task: None }
    }

    /// **Destructive method** - calling this method will take and resolve the task, and update the operations
    /// context if applicable. Returns a clone of the updated context if the task was waited on.
    /// 
    /// If None is returned, then the operation's context is the latest.
    /// 
    pub async fn task(&mut self, cancel_source: Receiver<()>) -> Option<ThunkContext>  {
        if let Some((task, cancel)) = self.task.take() {
            select! {
                r = task => {
                    match r {
                        Ok(tc) => {
                            self.context = tc.clone();
                            Some(tc)
                        },
                        Err(err) => {
                            event!(Level::ERROR, "error executing task {err}");
                            None
                        },
                    }
                }
                _ = cancel_source => {
                    event!(Level::INFO, "cancelling operation");
                    cancel.send(()).ok();
                    None
                }
            }
        } else {
            None
        }
    }

    /// Blocks the current thread indefinitely, until the task completes
    /// 
    pub fn wait(&mut self) -> Option<ThunkContext> {
        if let Some((task, _)) = self.task.as_mut() {
            if let Some(handle) = self.context.handle() {
                return handle.block_on(async {
                    match task.await {
                        Ok(tc) => Some(tc),
                        Err(_) => {
                            None
                        },
                    }
                })
            }
        }

        None
    }
}

impl Into<ThunkContext> for Operation {
    fn into(self) -> ThunkContext {
        self.context.clone()
    }
}

impl Item for Operation {
    fn visit_bool(&mut self, _name: impl AsRef<str>, _value: bool) {
        self.context.add_bool_attr(_name, _value);
    }

    fn visit_int(&mut self, _name: impl AsRef<str>, _value: i32) {
        self.context.add_int_attr(_name, _value);
    }

    fn visit_int_pair(&mut self, _name: impl AsRef<str>, _value: [i32; 2]) {
        self.context.add_int_pair_attr(_name, &_value);
    }

    fn visit_int_range(&mut self, _name: impl AsRef<str>, _value: [i32; 3]) {
        self.context.add_int_range_attr(_name, &_value);
    }

    fn visit_float(&mut self, _name: impl AsRef<str>, _value: f32) {
        self.context.add_float_attr(_name, _value);
    }

    fn visit_float_pair(&mut self, _name: impl AsRef<str>, _value: [f32; 2]) {
        self.context.add_float_pair_attr(_name, &_value);
    }

    fn visit_float_range(&mut self, _name: impl AsRef<str>, _value: [f32; 3]) {
        self.context.add_float_range_attr(_name, &_value);
    }

    fn visit_binary_vec(&mut self, _name: impl AsRef<str>, _value: impl Into<Vec<u8>>) {
        self.context.add_binary_attr(_name, _value);
    }

    fn visit_symbol(&mut self, _name: impl AsRef<str>, _value: impl AsRef<str>) {
        self.context.add_symbol(_name, _value);
    }

    fn visit_text(&mut self, _name: impl AsRef<str>, _value: impl AsRef<str>) {
        self.context.add_text_attr(_name, _value);
    }

    fn visit_reference(&mut self, _name: impl AsRef<str>, _value: u64) {        
        self.context.add_reference(_name, Value::Reference(_value));
    }
}

mod tests {
    use specs::Entity;
    use tokio::runtime::Handle;

    use crate::{catalog::Item, plugins::{ThunkContext, Plugin}, AttributeGraph};

    #[test]
    fn test_query() {
        use specs::{World, WorldExt};
        use crate::{AttributeGraph, plugins::ThunkContext, plugins::Thunk, state::AttributeIndex};
        use std::sync::Arc;

        // Test simple case where the src is just a thunk context
        let src = AttributeGraph::from(0)
            .with_text("name", "bob")
            .with_int("age", 99)
            .with_bool("is_alias", true)
            .with_binary("test_bin", vec![b'h', b'e', b'l', b'l', b'o'])
            .with_float_range("test_float_range", &[3.1, 1.4, 4.5])
            .with_float_pair("test_float_pair", &[3.1, 1.4])
            .with_float("test_float", 3.1)
            .with_int_pair("test_int_pair", &[3, 1])
            .with_int_range("test_int_range", &[3, 1, 4])
            .with_symbol("test_symbol", "cool_symbol")
            .to_owned();
        let src = ThunkContext::from(src);
        
        let query = src.query();
        let query = query
            .find_text("name")
            .find_int("age")
            .find_bool("is_alias")
            .find_binary("test_bin")
            .find_float_range("test_float_range")
            .find_float_pair("test_float_pair")
            .find_float("test_float")
            .find_int_pair("test_int_pair")
            .find_int_range("test_int_range")
            .find_symbol("test_symbol");
    
        let mut person = Person::default();
        query.evaluate(&mut person);

        assert_eq!(person.name, "bob");
        assert_eq!(person.age, 99);
        assert_eq!(person.is_alias, true);
        assert_eq!(person.test_bin, vec![b'h', b'e', b'l', b'l', b'o']);
        assert_eq!(person.test_float_range, [3.1, 1.4, 4.5]);
        assert_eq!(person.test_float_pair, [3.1, 1.4]);
        assert_eq!(person.test_float, 3.1);
        assert_eq!(person.test_int_pair, [3, 1]);
        assert_eq!(person.test_int_range, [3, 1, 4]);
        assert_eq!(person.test_symbol, "cool_symbol");
        eprintln!("{:#?}", person);
        
        let cached = &mut query.cache();

        let mut person_from_cached = Person::default();
        cached.cached(&mut person_from_cached);

        let person = person_from_cached;
        assert_eq!(person.name, "bob");
        assert_eq!(person.age, 99);
        assert_eq!(person.is_alias, true);
        assert_eq!(person.test_bin, vec![b'h', b'e', b'l', b'l', b'o']);
        assert_eq!(person.test_float_range, [3.1, 1.4, 4.5]);
        assert_eq!(person.test_float_pair, [3.1, 1.4]);
        assert_eq!(person.test_float, 3.1);
        assert_eq!(person.test_int_pair, [3, 1]);
        assert_eq!(person.test_int_range, [3, 1, 4]);
        assert_eq!(person.test_symbol, "cool_symbol");
        eprintln!("{:#?}", person);

        // Test thunk version of the query
        let person = Person::default();
        let thunk_query = query.thunk(person, Some(Thunk::from_plugin::<Announce>()));
        let src = Arc::new(src);

        let operation = thunk_query(src.clone());
        let context = operation.context;
        assert!(context.find_bool("is_alias").unwrap_or_default());
        assert_eq!(context.find_float_range("test_float_range"), Some((3.1, 1.4, 4.5)));
        assert!(operation.task.is_none());

        let world = World::new();
        let entity = world.entities().create();
        let runtime = tokio::runtime::Runtime::new().unwrap();

        // Test async enabled person
        let mut person = Person::default();
        person.handle = Some(runtime.handle().clone());
        person.entity = Some(entity);
        let thunk_query = query.thunk(person, Some(Thunk::from_plugin::<Announce>()));

        let mut operation = thunk_query(src.clone());
        let context = &operation.context;
        assert!(context.find_bool("is_alias").unwrap_or_default());
        assert_eq!(context.find_float_range("test_float_range"), Some((3.1, 1.4, 4.5)));

        let context = operation.wait().expect("completes");
        assert!(context.find_bool("is_alias").unwrap_or_default());
        assert_eq!(context.find_float_range("test_float_range"), Some((3.1, 1.4, 4.5)));
        assert_eq!(context.find_int("announced"), Some(10));

        // Test async enabled operation as the item 
        let entity = world.entities().create();
        let operation = crate::state::v2::Operation::item(entity, runtime.handle().clone());
        let thunk_query = query.thunk(operation, Some(Thunk::from_plugin::<Announce>()));
        let operation = thunk_query(src.clone());
        let context = &operation.context;

        assert!(context.find_bool("is_alias").unwrap_or_default());
        assert_eq!(context.find_float_range("test_float_range"), Some((3.1, 1.4, 4.5)));

        // Test using operation.context as src index
        let mut person_from_tc = Person::default();
        query.evaluate_with(&Arc::new(context.clone()), &mut person_from_tc);

        let person = person_from_tc;
        assert_eq!(person.name, "bob");
        assert_eq!(person.age, 99);
        assert_eq!(person.is_alias, true);
        assert_eq!(person.test_bin, vec![b'h', b'e', b'l', b'l', b'o']);
        assert_eq!(person.test_float_range, [3.1, 1.4, 4.5]);
        assert_eq!(person.test_float_pair, [3.1, 1.4]);
        assert_eq!(person.test_float, 3.1);
        assert_eq!(person.test_int_pair, [3, 1]);
        assert_eq!(person.test_int_range, [3, 1, 4]);
        assert_eq!(person.test_symbol, "cool_symbol");
        eprintln!("{:#?}", person);
    }

    #[derive(Debug, Default, Clone)]
    struct Person {
        name: String,
        age: u32,
        is_alias: bool,
        test_bin: Vec<u8>,
        test_float: f32,
        test_float_pair: [f32; 2],
        test_float_range: [f32; 3],
        test_symbol: String,
        test_int_pair: [i32; 2],
        test_int_range: [i32; 3],
        handle: Option<Handle>,
        entity: Option<Entity>,
    }

    struct Announce;

    impl Plugin for Announce {
        fn symbol() -> &'static str {
            "annonuce"
        }

        fn call_with_context(context: &mut ThunkContext) -> Option<crate::plugins::AsyncContext> {
            context.clone().task(|_| {
                let mut tc = context.clone();
                async move {
                    let mut a = 0;
                    for attr in tc.as_ref().iter_attributes() {
                        println!("announcing {}, {}", attr.name(), attr.value());
                        a += 1;
                    }

                    tc.as_mut().with_int("announced", a);

                    Some(tc)
                }
            })
        }
    }

    impl Into<ThunkContext> for Person {
        fn into(self) -> ThunkContext {
            let src = AttributeGraph::from(0)
                .with_text("name", self.name)
                .with_int("age", self.age as i32)
                .with_bool("is_alias", self.is_alias)
                .with_binary("test_bin", self.test_bin)
                .with_float_range("test_float_range", &self.test_float_range)
                .with_float_pair("test_float_pair", &self.test_float_pair)
                .with_float("test_float", self.test_float)
                .with_int_pair("test_int_pair", &self.test_int_pair)
                .with_int_range("test_int_range", &self.test_int_range)
                .with_symbol("test_symbol", self.test_symbol)
                .to_owned();
            let mut tc = ThunkContext::from(src);
            if let (Some(handle), Some(entity)) = (self.handle, self.entity) {
                tc = tc.enable_async(entity, handle, None, None, None, None);
            }
            tc 
        }
    }

    /// TODO: add a proc macro that derives this
    /// TODO: could try reusing serde traits
    impl Item for Person {
        fn visit_text(&mut self, _name: impl AsRef<str>, _value: impl AsRef<str>) {
            if _name.as_ref() == "name" {
                self.name = _value.as_ref().to_string();
            }
        }

        fn visit_symbol(&mut self, _name: impl AsRef<str>, _value: impl AsRef<str>) {
            if _name.as_ref() == "test_symbol" {
                self.test_symbol = _value.as_ref().to_string();
            }
        }

        fn visit_int(&mut self, _name: impl AsRef<str>, _value: i32) {
            if _name.as_ref() == "age" {
                self.age = _value as u32; 
            }
        }

        fn visit_int_range(&mut self, _name: impl AsRef<str>, _value: [i32; 3]) {
            if _name.as_ref() == "test_int_range" {
                self.test_int_range = _value; 
            }
        }

        fn visit_int_pair(&mut self, _name: impl AsRef<str>, _value: [i32; 2]) {
            if _name.as_ref() == "test_int_pair" {
                self.test_int_pair = _value; 
            }
        }

        fn visit_bool(&mut self, _name: impl AsRef<str>, _value: bool) {
            if _name.as_ref() == "is_alias" {
                self.is_alias = _value;
            }
        }

        fn visit_float_pair(&mut self, _name: impl AsRef<str>, _value: [f32; 2]) {
            if _name.as_ref() == "test_float_pair" {
                self.test_float_pair = _value;
            }
        }

        fn visit_float_range(&mut self, _name: impl AsRef<str>, _value: [f32; 3]) {
            if _name.as_ref() == "test_float_range" {
                self.test_float_range = _value;
            }
        }

        fn visit_float(&mut self, _name: impl AsRef<str>, _value: f32) {
            if _name.as_ref() == "test_float" {
                self.test_float = _value;
            }
        }

        fn visit_binary_vec(&mut self, _name: impl AsRef<str>, _value: impl Into<Vec<u8>>) {
            if _name.as_ref() == "test_bin" {
                self.test_bin = _value.into();
            }
        }
    }
}


