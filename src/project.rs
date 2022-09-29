use reality::{Block, BlockIndex, Interpreter, Parser};
use specs::{Join, World, WorldExt};
use tracing::event;
use tracing::Level;

use crate::LifecycleOptions;
use crate::{plugins::Println, AttributeGraph, Engine, Event, Install, Process, Runtime, Timer, engine::{Fork, Next, Repeat, LifecycleResolver}, Exit};

/// Trait to facilitate
///
pub trait Project {
    /// TODO: Currently Engine is stateless, but leaving this here as an extension point
    /// 
    fn configure_engine(engine: &mut Engine);

    /// Interpret a compiled block, this will run after the Engine
    /// has a chance to interpret.
    ///
    fn interpret(world: &World, block: &Block);

    /// Override to provide a custom Runtime,
    ///
    fn runtime() -> Runtime {
        default_runtime()
    }

    /// Override to provide a custom World when compile is called,
    ///
    fn world() -> World {
        let mut world = specs::World::new();
        world.register::<Event>();
        world.register::<Engine>();
        world.register::<Runtime>();
        world.register::<AttributeGraph>();
        world.register::<LifecycleOptions>();
        world.insert(Self::runtime());
        world
    }

    /// Compiles runmd into blocks, interprets those blocks,
    /// and returns the World,
    ///
    /// In addition, sets up attribute graphs for each entity with a plugin.
    /// 
    /// Override with care as this adds critical components for the event runtime,
    ///
    fn compile(runmd: impl AsRef<str>) -> World {
        let parser = Parser::new_with(Self::world())
            .with_special_attr::<Runtime>()
            .with_special_attr::<Engine>()
            .parse(runmd);
        
        let mut world = parser.commit();

        let engine = &mut Engine::default();

        Self::configure_engine(engine);
        engine.initialize(&mut world);

        // Engine lifecycle options
        let fork = Fork::default();
        fork.initialize(&mut world);

        let next = Next::default();
        next.initialize(&mut world);

        let repeat = Repeat::default();
        repeat.initialize(&mut world);

        let exit = Exit::default();
        exit.initialize(&mut world);

        // Setup graphs for all plugin entities
        for (entity, block_index) in
            (&world.entities(), &world.read_component::<BlockIndex>()).join()
        {
            let mut graph = AttributeGraph::new(block_index.clone());
            if entity.id() != block_index.root().id() {
                graph = graph.scope(entity).expect("invalid block index state");
            }
            world
                .write_component()
                .insert(entity, graph)
                .expect("Could not insert graph for entity");
        }

        for block in world.read_component::<Block>().join() {
            engine.interpret(&world, block);
            repeat.interpret(&world, block);
            fork.interpret(&world, block);
            next.interpret(&world, block);
            exit.interpret(&world, block);
            Self::interpret(&world, block);
            event!(Level::TRACE, "Interpreted block {} {}", block.name(), block.symbol());
        }

        // Resolve lifecycle settings before returning
        {
            let lifecycle_resolver = world.system_data::<LifecycleResolver>();
            let settings = lifecycle_resolver.resolve_lifecycle();
            world.insert(settings);
        }

        world
    }
}

/// Returns a basic runtime with standard plugins,
///
pub fn default_runtime() -> Runtime {
    let mut runtime = Runtime::default();
    runtime.install_with_custom::<Process>("");
    runtime.install_with_custom::<Println>("");
    runtime.install_with_custom::<Install>("");
    runtime.install_with_custom::<Timer>("");
    runtime
}
