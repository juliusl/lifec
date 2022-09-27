use reality::{BlockProperty, Interpreter, SpecialAttribute};
use specs::{Component, VecStorage, World, WorldExt};

mod event;
pub use event::Event;

mod sequence;
pub use sequence::Sequence;

mod connection;
pub use connection::Connection;

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
#[derive(Default, Debug, Component)]
#[storage(VecStorage)]
pub struct Engine {}

impl SpecialAttribute for Engine {
    fn ident() -> &'static str {
        "engine"
    }

    fn parse(parser: &mut reality::AttributeParser, _todo: impl AsRef<str>) {
        // Install the event special attribute
        parser.with_custom::<Event>();
    }
}

impl Interpreter for Engine {
    fn initialize(&self, world: &mut specs::World) {
        world.register::<Event>();
        world.register::<Sequence>();
        world.register::<Connection>();
    }

    fn interpret(&self, world: &World, block: &reality::Block) {
        if block.name().is_empty() && block.symbol().is_empty() {
            return;
        }

        let mut engines: Vec<(specs::Entity, Sequence)> = vec![];

        for index in block
            .index()
            .iter()
            .filter(|i| i.root().name() == "runtime")
        {
            if let Some(BlockProperty::List(plugins)) = index.properties().property("sequence").and_then(|b| match b {
                BlockProperty::Single(s) => Some(BlockProperty::List(vec![s.clone()])),
                BlockProperty::List(_) => Some(b.clone()),
                _ => None,
            }) {
                let mut sequence = Sequence::default();

                for plugin in plugins.iter().filter_map(|p| match p {
                    atlier::system::Value::Int(id) => Some(id),
                    _ => None,
                }) {
                    let plugin = world.entities().entity(*plugin as u32);
                    sequence.add(plugin);

                    // TODO: Can assert that the .runtime attribute worked
                }
                let parent = world.entities().entity(index.root().id());

                // Connect the engines
                if let Some((last, mut previous_sequence)) = engines.pop() {
                    let connection = previous_sequence.connect(&sequence);
                    world.write_component().insert(last, connection).ok();

                    previous_sequence.set_cursor(parent);
                    world.write_component().insert(last, previous_sequence).ok();
                }

                engines.push((parent, sequence));
            }
        }

        if let Some((last, previous_sequence)) = engines.pop() {
            world.write_component().insert(last, previous_sequence).ok();
            world
                .write_component()
                .insert(last, Connection::default())
                .ok();
        }
    }
}

#[test]
fn test_engine() {
    // TODO: Write assertions
    use crate::Process;
    use crate::Runtime;
    use crate::Timer;
    use specs::WorldExt;
    use reality::BlockProperties;
    use reality::BlockIndex;

    let mut runtime = Runtime::default();
    runtime.install_with_custom::<Process>("call");
    runtime.install_with_custom::<Timer>("call");

    let mut world = specs::World::new();
    world.register::<Runtime>();
    world.register::<Event>();
    world.insert(runtime);

    let parser = reality::Parser::new_with(world)
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
            .read_component::<reality::Block>()
            .get(process)
            .unwrap()
            .clone();
        eprintln!("{:#?}", block);
        eprintln!("{:#?}", block.index());
        eprintln!("{:#?}", block.map_control());

        let step_one = world.entities().entity(2);
        {
            let block = world
                .read_component::<reality::Block>()
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
                .read_component::<reality::Block>()
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
