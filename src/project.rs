use reality::{Block, BlockIndex, Interpreter, Parser};
use specs::{DispatcherBuilder, Entity};
use specs::{Join, World, WorldExt};
use tracing::event;
use tracing::Level;

use crate::engine::{Activity, Loop};
use crate::plugins::{ErrorContext, StatusUpdate};
use crate::prelude::{Publish, Readln, Watch};
use crate::{
    engine::{Fork, LifecycleResolver, Next, Repeat},
    plugins::Println,
    AttributeGraph, Engine, Event, Exit, Install, Process, Runtime, Timer,
};
use crate::{LifecycleOptions, Operation, Sequence, Start, Thunk, ThunkContext};

mod runmd_listener;
pub use runmd_listener::RunmdListener;

mod status_update_listener;
pub use status_update_listener::StatusUpdateListener;

mod completed_plugin_listener;
pub use completed_plugin_listener::CompletedPluginListener;

mod operation_listener;
pub use operation_listener::OperationListener;

mod error_context_listener;
pub use error_context_listener::ErrorContextListener;

mod start_command_listener;
pub use start_command_listener::StartCommandListener;

mod source;
pub use source::RunmdFile;
pub use source::Source;

mod workspace;
pub use workspace::Workspace;

/// Trait to facilitate
///
pub trait Project {
    /// Override to initialize the world,
    ///
    fn initialize(_world: &mut World) {}

    /// Interpret a compiled block, this will run after the Engine
    /// has a chance to interpret.
    ///
    fn interpret(world: &World, block: &Block);

    /// Override to customize the dispatcher,
    ///
    fn configure_dispatcher(
        _dispatcher_builder: &mut DispatcherBuilder,
        _context: Option<ThunkContext>,
    ) {
    }

    /// Override to provide a custom Runtime,
    ///
    fn runtime() -> Runtime {
        default_runtime()
    }

    /// Override to provide a custom Parser,
    ///
    fn parser() -> Parser {
        default_parser(Self::world())
    }

    /// Override to provide a custom World when compile is called,
    ///
    fn world() -> World {
        let mut world = default_world();
        world.insert(Self::runtime());
        world
    }

    /// When compiling in the context of a project directory, the file name is taken into consideration when parsing
    /// runmd. The file name becomes the implicit symbol w/in the context of the file.
    ///
    /// In this context the root block can only be defined within in the .runmd file in the directory.
    ///
    fn compile_workspace(workspace: Workspace, files: Vec<RunmdFile>) -> World {
        let mut parser = Self::parser();

        for RunmdFile { symbol } in files {
            parser.set_implicit_symbol(&symbol);

            let file = workspace.work_dir().join(format!("{symbol}.runmd"));
            match  std::fs::read_to_string(&file) {
                Ok(runmd) => {
                    parser = parser.parse(runmd);
                },
                Err(err) => {
                    event!(Level::ERROR, "Could not read file {:?} {err}", file);
                },
            }
        }

        // Parse the root file without the implicit symbol set
        parser.unset_implicit_symbol();

        let root = workspace.work_dir().join(".runmd");
        match std::fs::read_to_string(&root) {
            Ok(runmd) => {
                let world = Self::compile(runmd, Some(parser), false);
                
                // Enable workspace on any thunk context in the World
                // TODO: Pivot thunk context build to Event
                for tc in world.write_component::<ThunkContext>().as_mut_slice() {
                    tc.enable_workspace(workspace.clone());
                }
                
                return world;
            }
            Err(err) => {
                panic!("Could not compile workspace, root .runmd file required, {err}");
            },
        }
    }

    /// Compiles runmd into blocks, interprets those blocks,
    /// and returns the World,
    ///
    /// In addition, sets up attribute graphs for each entity with a plugin.
    ///
    /// Override with care as this adds critical components for the event runtime,
    ///
    fn compile(runmd: impl AsRef<str>, parser: Option<Parser>, is_single_src: bool) -> World {
        let parser = parser.unwrap_or(Self::parser()).parse(runmd.as_ref());

        let mut world = parser.commit();

        // If this is loading just one file, then a Source can be inserted
        if is_single_src {
            // Save source to world
            world.insert(Source(runmd.as_ref().to_string()));
        }

        let engine = &mut Engine::default();
        engine.initialize(&mut world);

        // Engine lifecycle options
        let fork = Fork::default();
        fork.initialize(&mut world);

        let next = Next::default();
        next.initialize(&mut world);

        let exit = Exit::default();
        exit.initialize(&mut world);

        let r#loop = Loop::default();
        r#loop.initialize(&mut world);

        Self::initialize(&mut world);

        // Setup graphs for all plugin entities
        for (entity, block_index, event) in (
            &world.entities(),
            &world.read_component::<BlockIndex>(),
            world.read_component::<Event>().maybe(),
        )
            .join()
        {
            let mut graph = AttributeGraph::new(block_index.clone());
            if entity.id() != block_index.root().id() {
                graph = graph.scope(entity.id()).expect("invalid block index state");
            }
            world
                .write_component()
                .insert(entity, graph)
                .expect("Should be able to insert graph for entity");

            if let Some(_) = event {
                world
                    .write_component()
                    .insert(entity, Activity::default())
                    .expect("Should be able to insert an activity");
            }
        }

        for block in world.read_component::<Block>().join() {
            engine.interpret(&world, block);
            fork.interpret(&world, block);
            next.interpret(&world, block);
            exit.interpret(&world, block);
            r#loop.interpret(&world, block);
            Self::interpret(&world, block);
            event!(
                Level::TRACE,
                "Interpreted block {} {}",
                block.name(),
                block.symbol()
            );
        }

        // Resolve lifecycle settings before returning
        {
            let lifecycle_resolver = world.system_data::<LifecycleResolver>();
            let settings = lifecycle_resolver.resolve_lifecycle();
            world.insert(settings);
        }

        world
    }

    /// Override to receive/handle runmd
    ///
    fn on_runmd(&mut self, _runmd: String) {}

    /// Override to receive/handle status updates
    ///
    fn on_status_update(&mut self, _status_update: StatusUpdate) {}

    /// Override to receive/handle operations
    ///
    fn on_operation(&mut self, _operation: Operation) {}

    /// Override to receive/handle errors
    ///
    fn on_error_context(&mut self, _error: ErrorContext) {}

    /// Override to receive/handle the entity when a plugin call completes
    ///
    fn on_completed_plugin_call(&mut self, _entity: Entity) {}

    /// Override to receive/handle start commands,
    ///
    fn on_start_command(&mut self, _start_command: Start) {}
}

/// Returns a basic runtime with standard plugins,
///
pub fn default_runtime() -> Runtime {
    let mut runtime = Runtime::default();
    runtime.install_with_custom::<Process>("");
    runtime.install_with_custom::<Println>("");
    runtime.install_with_custom::<Install>("");
    runtime.install_with_custom::<Timer>("");
    runtime.install_with_custom::<Readln>("");
    runtime.install_with_custom::<Watch>("");
    runtime.install_with_custom::<Publish>("");
    runtime
}

/// Returns a basic reality parser,
///
pub fn default_parser(world: World) -> Parser {
    Parser::new_with(world)
        .with_special_attr::<Runtime>()
        .with_special_attr::<Engine>()
}

pub fn default_world() -> World {
    let mut world = specs::World::new();
    world.register::<Event>();
    world.register::<Engine>();
    world.register::<Runtime>();
    world.register::<AttributeGraph>();
    world.register::<LifecycleOptions>();
    world.register::<ThunkContext>();
    world.register::<Activity>();
    world.register::<Sequence>();
    world.register::<Thunk>();
    world.register::<Repeat>();
    world
}
