use std::collections::HashMap;
use std::fmt::Debug;

use crate::{AttributeParser, Block, Interpreter, SpecialAttribute};
use specs::{Component, Entity, VecStorage, World, WorldExt};

mod event;
pub use event::Event;

mod sequence;
pub use sequence::Sequence;

mod connection;
pub use connection::Connection;

mod activity;
pub use activity::Activity;
mod transition;
pub use transition::Transition;

mod cursor;
pub use cursor::Cursor;

mod plugins;
pub use plugins::Plugins;

mod events;
pub use events::EventStatus;
pub use events::Events;

mod lifecycle;
pub use lifecycle::Lifecycle;

mod systems;
pub use systems::install;

mod limit;
pub use limit::Limit;

use tracing::Level;

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
pub struct Engine {
    /// Pointer to the start of the engine sequence
    ///
    start: Option<Entity>,
    /// Limit this engine can repeat
    /// 
    limit: Option<Limit>,
    /// Vector of transitions
    ///
    transitions: Vec<(Transition, Vec<String>)>,
    /// Lifecycle operation to use,
    ///
    lifecycle: Option<(Lifecycle, Option<Vec<String>>)>,
}

impl Engine {
    /// Creates a new engine component w/ start,
    ///
    pub fn new(start: Entity) -> Self {
        Self {
            start: Some(start),
            limit: None,
            transitions: vec![],
            lifecycle: None,
        }
    }

    /// Returns the start of the engine,
    ///
    pub fn start(&self) -> Option<&Entity> {
        self.start.as_ref()
    }

    /// Returns the limit of this engine, if any
    /// 
    pub fn limit(&self) -> Option<&Limit> {
        self.limit.as_ref()
    }

    /// Finds the entity for a block,
    ///
    pub fn find_block(world: &World, expression: impl AsRef<str>) -> Option<Entity> {
        let block_list = world.read_resource::<HashMap<String, Entity>>();

        tracing::event!(
            tracing::Level::DEBUG,
            "Looking for block ``` {}",
            expression.as_ref()
        );

        block_list.get(expression.as_ref()).cloned()
    }

    /// Returns an iterator over transitions,
    ///
    pub fn iter_transitions(&self) -> impl Iterator<Item = &(Transition, Vec<String>)> {
        self.transitions.iter()
    }

    /// Adds a transition to the engine,
    ///
    pub fn add_transition(&mut self, transition: Transition, events: Vec<String>) {
        if self.lifecycle.is_none() {
            self.transitions.push((transition, events));
        } else {
            tracing::event!(
                Level::ERROR,
                "Tried to add a transition, after a lifecycle action has been set"
            );
        }
    }

    /// Sets a lifecycle action for this engine,
    ///
    pub fn set_lifecycle(&mut self, lifecycle: Lifecycle, engines: Option<Vec<String>>) {
        if let Lifecycle::Repeat(limit) = lifecycle {
            self.limit = Some(Limit(limit));
        }

        self.lifecycle = Some((lifecycle, engines));
    }
}

impl SpecialAttribute for Engine {
    fn ident() -> &'static str {
        "engine"
    }

    fn parse(parser: &mut AttributeParser, _todo: impl AsRef<str>) {
        if let Some(entity) = parser.entity() {
            let world = parser.world().expect("should have a world");
            world
                .write_component()
                .insert(entity, Engine::default())
                .expect("should be able to insert");

            parser.add_custom_with("once", |p, events| {
                let entity = p.entity().expect("should have entity");
                let world = p.world().expect("should have world");
                let mut engines = world.write_component::<Engine>();
                let engine = engines.get_mut(entity).expect("should have engine");

                engine.add_transition(Transition::Once, Self::parse_idents(events));
            });

            parser.add_custom_with("start", |p, events| {
                let entity = p.entity().expect("should have entity");
                let world = p.world().expect("should have world");
                let mut engines = world.write_component::<Engine>();
                let engine = engines.get_mut(entity).expect("should have engine");

                engine.add_transition(Transition::Start, Self::parse_idents(events));
            });

            parser.add_custom_with("select", |p, events| {
                let entity = p.entity().expect("should have entity");
                let world = p.world().expect("should have world");
                let mut engines = world.write_component::<Engine>();
                let engine = engines.get_mut(entity).expect("should have engine");

                engine.add_transition(Transition::Select, Self::parse_idents(events));
            });

            parser.add_custom_with("spawn", |p, events| {
                let entity = p.entity().expect("should have entity");
                let world = p.world().expect("should have world");
                let mut engines = world.write_component::<Engine>();
                let engine = engines.get_mut(entity).expect("should have engine");

                engine.add_transition(Transition::Spawn, Self::parse_idents(events));
            });

            parser.add_custom_with("buffer", |p, events| {
                let entity = p.entity().expect("should have entity");
                let world = p.world().expect("should have world");
                let mut engines = world.write_component::<Engine>();
                let engine = engines.get_mut(entity).expect("should have engine");

                engine.add_transition(Transition::Buffer, Self::parse_idents(events));
            });

            parser.add_custom_with("exit", |p, _| {
                let entity = p.entity().expect("should have entity");
                let world = p.world().expect("should have world");
                let mut engines = world.write_component::<Engine>();
                let engine = engines.get_mut(entity).expect("should have engine");

                engine.set_lifecycle(Lifecycle::Exit, None);
            });

            parser.add_custom_with("loop", |p, _| {
                let entity = p.entity().expect("should have entity");
                let world = p.world().expect("should have world");
                let mut engines = world.write_component::<Engine>();
                let engine = engines.get_mut(entity).expect("should have engine");

                engine.set_lifecycle(Lifecycle::Loop, None);
            });

            parser.add_custom_with("next", |p, e| {
                let entity = p.entity().expect("should have entity");
                let world = p.world().expect("should have world");
                let mut engines = world.write_component::<Engine>();
                let engine = engines.get_mut(entity).expect("should have engine");

                engine.set_lifecycle(Lifecycle::Next, Some(vec![e]));
            });

            parser.add_custom_with("fork", |p, e| {
                let entity = p.entity().expect("should have entity");
                let world = p.world().expect("should have world");
                let mut engines = world.write_component::<Engine>();
                let engine = engines.get_mut(entity).expect("should have engine");

                let forks = Self::parse_idents(e);

                engine.set_lifecycle(Lifecycle::Next, Some(forks));
            });

            parser.add_custom_with("repeat", |p, e| {
                let entity = p.entity().expect("should have entity");
                let world = p.world().expect("should have world");
                let mut engines = world.write_component::<Engine>();
                let engine = engines.get_mut(entity).expect("should have engine");

                if let Some(count) = e.parse::<usize>().ok() {
                    engine.set_lifecycle(Lifecycle::Repeat(count), None);
                } else {
                    tracing::event!(Level::ERROR, "Loop count must be a positive integer");
                }
            });
        }
    }
}

impl Interpreter for Engine {
    fn initialize(&self, world: &mut specs::World) {
        world.register::<Event>();
        world.register::<Sequence>();
        world.register::<Connection>();
        world.register::<Activity>();
    }

    /// Handles interpreting blocks and setting up sequences
    ///
    fn interpret(&self, world: &World, block: &Block) {
        if block.is_root_block() {
            return;
        }

        if block.is_control_block() {
            let block_entity = world.entities().entity(block.entity());

            let mut sequence = Sequence::default();

            if let Some(engine) = world.write_component::<Engine>().get_mut(block_entity) {
                tracing::event!(Level::TRACE, "{:#?}", engine);

                // Assign transitions to events
                for (transition, events) in engine.clone().iter_transitions() {
                    for event in events.iter().filter_map(|e| {
                        Engine::find_block(world, format!("{e} {}", block.symbol()))
                    }) {
                        world
                            .write_component()
                            .insert(event, transition.clone())
                            .expect("should be able to insert transition");

                        sequence.add(event);
                        engine.start.get_or_insert(event);
                    }
                }

                // Handle lifecycle settings
                if let Some((lifecycle, engines)) = engine.lifecycle.as_ref() {
                    match (lifecycle, engines) {
                        (Lifecycle::Next, Some(engines)) => {
                            if let Some(engine) = engines
                                .iter()
                                .filter_map(|e| Engine::find_block(world, format!("{e}")))
                                .next()
                            {
                                sequence.set_cursor(engine);
                            }
                        }
                        (Lifecycle::Fork, Some(engines)) => {
                            for engine in engines
                                .iter()
                                .filter_map(|e| Engine::find_block(world, format!("{e}")))
                            {
                                sequence.set_cursor(engine);
                            }
                        }
                        (Lifecycle::Exit, _) => {}
                        (Lifecycle::Loop, _) |  (Lifecycle::Repeat(_), _)  => {
                            sequence.set_cursor(block_entity);
                        }
                        _ => {
                            tracing::event!(
                                Level::ERROR,
                                "Could not parse lifecycle for engine {}",
                                block.symbol()
                            );
                        }
                    }
                }
            }

            world
                .write_component()
                .insert(block_entity, sequence)
                .expect("should have inserted a sequence");

            return;
        }

        if !block.is_root_block() && !block.is_control_block() {
            let block_entity = world.entities().entity(block.entity());

            let mut events = world.write_component::<Event>();
            if let Some(event) = events.get_mut(block_entity) {
                event.set_name(block.name());

                let sequence = event
                    .sequence()
                    .expect("should have a sequence at compile-time");

                let mut sequences = world.write_component::<Sequence>();
                sequences
                    .insert(block_entity, sequence.clone())
                    .expect("should be able to insert component");
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
