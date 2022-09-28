use crate::{AttributeParser, Block, BlockProperty, Interpreter, SpecialAttribute};
use specs::{Component, VecStorage, World, WorldExt};

mod event;
pub use event::Event;

mod sequence;
pub use sequence::Sequence;

mod connection;
pub use connection::Connection;

mod exit;
pub use exit::Exit;
pub use self::exit::ExitListener;

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
pub struct Engine();

impl SpecialAttribute for Engine {
    fn ident() -> &'static str {
        "engine"
    }

    fn parse(parser: &mut AttributeParser, _todo: impl AsRef<str>) {
        // Install the event special attribute
        parser.with_custom::<Event>();
    }
}

impl Interpreter for Engine {
    fn initialize(&self, world: &mut specs::World) {
        world.register::<Event>();
        world.register::<Sequence>();
        world.register::<Connection>();

        let (exit, listener) = Exit::new();
        world.insert(Some(exit));
        world.insert(Some(listener));

        world.fetch_mut::<Option<Exit>>().take();
        world.fetch_mut::<Option<ExitListener>>().take();
    }

    /// Handles interpreting blocks and setting up sequences
    ///
    fn interpret(&self, world: &World, block: &Block) {
        if block.is_root_block() {
            return;
        }

        if block.is_control_block() {
            for index in block.index().iter().filter(|i| i.root().name() == "engine") {
                if index
                    .find_property("exit_on_completion")
                    .and_then(|e| Some(e.is_enabled()))
                    .unwrap_or_default()
                {
                    let (exit, listener) = Exit::new();
                    let mut exit_resource = world.write_resource::<Option<Exit>>();
                    *exit_resource = Some(exit);

                    let mut exit_resource = world.write_resource::<Option<ExitListener>>();
                    *exit_resource = Some(listener);
                }

                // TODO index engines
            }
        }

        // let mut engines: Vec<(specs::Entity, Sequence)> = vec![];
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
                    // TODO: Can assert that the .runtime attribute worked
                }

                if let Some(parent) = sequence.peek() {
                    world
                        .write_component()
                        .insert(parent, sequence)
                        .expect("Should be able to insert");
                }

                // TODO - This needs to happen elsewhere
                // // Connect the engines
                // if let Some((last, mut previous_sequence)) = engines.pop() {
                //     let connection = previous_sequence.connect(&sequence);
                //     world.write_component().insert(last, connection).ok();
                //     previous_sequence.set_cursor(parent);
                //     world.write_component().insert(last, previous_sequence).ok();
                // }
                // engines.push((parent, sequence));
            }
        }

        // if let Some((last, previous_sequence)) = engines.pop() {
        //     world.write_component().insert(last, previous_sequence).ok();
        //     world
        //         .write_component()
        //         .insert(last, Connection::default())
        //         .ok();
        // }
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
