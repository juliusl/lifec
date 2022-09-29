use std::collections::HashMap;

use crate::{AttributeParser, Block, BlockProperty, Interpreter, SpecialAttribute};
use specs::{Component, Entity, VecStorage, World, WorldExt};

mod event;
pub use event::Event;

mod sequence;
pub use sequence::Sequence;

mod connection;
pub use connection::Connection;

mod exit;
pub use self::exit::ExitListener;
pub use exit::Exit;

mod repeat;
pub use repeat::Repeat;

mod next;
pub use next::Next;

mod fork;
pub use fork::Fork;

mod once;
pub use once::Once;

mod lifecycle;
pub use lifecycle::LifecycleOptions;
pub use lifecycle::LifecycleResolver;


/// An engine is a sequence of events, this component manages
/// sequences of events
///
/// # Example runmd usage
///
/// First in a control block, an engine attribute is defined, with
/// two `event` properties with the values step_one and step_two
/// ```runmd
/// ``` test
/// + .engine
/// : .event step_one
/// : .event step_two
/// ```
/// ```
///
/// When the engine starts, it will fire the first event `step_one`, which corresponds
/// with the below block.
///
/// ``` runmd
/// ``` step_one test
/// + .runtime
/// : .timer    50
/// : .println  done
/// ```
/// ```
///
/// These blocks configure the sequence of plugin calls that will be executed
/// on their start.
///
/// ``` runmd
/// ``` step_two test
/// + .runtime
/// : .timer    20
/// : .println  done
/// ```
/// ```
///
#[derive(Clone, Default, Debug, Component)]
#[storage(VecStorage)]
pub struct Engine;

impl Engine {
    /// Finds the entity for a block,
    ///
    pub fn find_block(world: &World, expression: impl AsRef<str>) -> Option<Entity> {
        let block_list = world.read_resource::<HashMap<String, Entity>>();

        tracing::event!(tracing::Level::TRACE, "Looking for block {}", expression.as_ref());

        block_list.get(expression.as_ref()).cloned()
    }
}

impl SpecialAttribute for Engine {
    fn ident() -> &'static str {
        "engine"
    }

    fn parse(parser: &mut AttributeParser, _todo: impl AsRef<str>) {
        // Event types
        parser.with_custom::<Event>();
        parser.with_custom::<Once>();
        // Lifecycle options
        parser.with_custom::<Exit>();
        parser.with_custom::<Next>();
        parser.with_custom::<Fork>();
        parser.with_custom::<Repeat>();
    }
}

impl Interpreter for Engine {
    fn initialize(&self, world: &mut specs::World) {
        world.register::<Event>();
        world.register::<Sequence>();
        world.register::<Connection>();
    }

    /// Handles interpreting blocks and setting up sequences
    ///
    fn interpret(&self, world: &World, block: &Block) {
        if block.is_root_block() {
            return;
        }

        if block.is_control_block() {
            let block_entity = world.entities().entity(block.entity());
            world
                .write_component()
                .insert(block_entity, self.clone())
                .expect("should be able to insert engine component");

            if let Some(engine) = block.index().iter().find(|b| b.root().name() == "engine") {
                let events = engine
                    .properties()
                    .property("event")
                    .and_then(BlockProperty::symbol_vec)
                    .expect("events must be symbols");

                let mut engine_sequence = Sequence::default();
                for event in events {
                    let mut expression = event.to_string();
                    if event.starts_with(" ") {
                        expression = format!("{} {}", event.trim(), block.symbol());
                    }

                    if let Some(event_entity) = Engine::find_block(world, expression) {
                        engine_sequence.add(event_entity);
                    }
                }

                world
                    .write_component()
                    .insert(block_entity, engine_sequence)
                    .expect("should be able to insert component");
            }

            return;
        }

        for index in block
            .index()
            .iter()
            .filter(|i| i.root().name() == "runtime")
        {
            if let Some(plugins) = index
                .properties()
                .property("sequence")
                .and_then(BlockProperty::int_vec)
            {
                let mut sequence = Sequence::default();

                for plugin in plugins.iter().map(|p| *p) {
                    let plugin = world.entities().entity(*plugin as u32);
                    sequence.add(plugin);
                }

                if let Some(parent) = sequence.next() {
                    world
                        .write_component()
                        .insert(parent, sequence)
                        .expect("Should be able to insert");
                }
            }
        }
    }
}

#[test]
fn test_engine() {
    // TODO: Write assertions
    use crate::*;
    use specs::WorldExt;

    let mut runtime = Runtime::default();
    runtime.install_with_custom::<Process>("call");
    runtime.install_with_custom::<Timer>("call");

    let mut world = specs::World::new();
    world.register::<Runtime>();
    world.register::<Event>();
    world.insert(runtime);

    let parser = Parser::new_with(world)
        .with_special_attr::<Runtime>()
        .with_special_attr::<Engine>();

    let parser = parser.parse(
        r#"
    ``` test
    + .engine 
    : .event step_one 
    : .event step_two 
    ```

    ``` step_one test
    : coolness .int 100

    + .runtime
    : .process called step one
    : .process called step one again
    : .process called step one again again
    ```

    ``` step_two test
    + .runtime
    : .process called step two
    : .timer 50 s
    : .process called step two again
    : .process called step two again again
    ``` 
    "#,
    );

    let mut world = parser.commit();
    let process = world.entities().entity(1);
    {
        // TODO: Write assertions
        let block = world
            .read_component::<Block>()
            .get(process)
            .unwrap()
            .clone();
        eprintln!("{:#?}", block);
        eprintln!("{:#?}", block.index());
        eprintln!("{:#?}", block.map_control());

        let step_one = world.entities().entity(2);
        {
            let block = world
                .read_component::<Block>()
                .get(step_one)
                .unwrap()
                .clone();

            eprintln!("{:#?}", block);
            eprintln!("{:#?}", block.index());
            eprintln!("{:#?}", block.map_control());

            Engine::default().initialize(&mut world);
            Engine::default().interpret(&world, &block);
            world.maintain();

            let sequence = world
                .read_component::<Sequence>()
                .get(step_one)
                .unwrap()
                .clone();

            for e in sequence.iter_entities() {
                let properties = world.read_component::<BlockProperties>();
                let properties = properties.get(e);
                eprintln!("{:#?}", properties);

                let index = world.read_component::<BlockIndex>();
                let index = index.get(e);
                eprintln!("{:#?}", index);

                let event = world.read_component::<Event>();
                let event = event.get(e).expect("should have been added");
                eprintln!("{event}");
            }
        }

        let step_two = world.entities().entity(6);
        {
            let block = world
                .read_component::<Block>()
                .get(step_two)
                .unwrap()
                .clone();

            // eprintln!("{:#?}", block);
            // eprintln!("{:#?}", block.index());
            // eprintln!("{:#?}", block.map_control());

            Engine::default().initialize(&mut world);
            Engine::default().interpret(&world, &block);
            world.maintain();

            let sequence = world
                .read_component::<Sequence>()
                .get(step_two)
                .unwrap()
                .clone();

            for e in sequence.iter_entities() {
                let properties = world.read_component::<BlockProperties>();
                let properties = properties.get(e);
                eprintln!("{:#?}", properties);
            }
        }
    }
}
