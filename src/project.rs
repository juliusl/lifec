use std::collections::BTreeMap;

use crate::engine::Adhoc;
use crate::engine::NodeCommandHandler;
use crate::engine::Profiler;
use crate::engine::Yielding;
use crate::guest::Guest;
use crate::prelude::*;

mod source;
pub use source::RunmdFile;
pub use source::WorkspaceSource;

mod workspace;
pub use workspace::Operations;
pub use workspace::Workspace;
pub use workspace::WorkspaceConfig;

mod listener;
pub use listener::Listener;

/// Trait to facilitate
///
pub trait Project
where
    Self: Default + Send,
{
    /// Override to initialize the world,
    ///
    fn initialize(_world: &mut World) {}

    /// Interpret a compiled block, this will run after the Engine
    /// has a chance to interpret.
    ///
    fn interpret(world: &World, block: &Block);

    /// Override to customize the dispatcher,
    ///
    fn configure_dispatcher(_world: &World, _dispatcher_builder: &mut DispatcherBuilder) {}

    /// Override to provide a custom Runtime,
    ///
    fn runtime() -> Runtime {
        let mut runtime = default_runtime();
        runtime.install_with_custom::<Run<Self>>("");
        runtime
    }

    /// Override to provide a custom Parser,
    ///
    fn parser() -> Parser {
        let mut world = Self::world();
        let mut handlers = Self::node_handlers();
        {
            let runtime = world.fetch::<Runtime>();

            for (name, handler) in runtime.iter_handlers() {
                handlers.insert(name.to_string(), handler.clone());
            }
        }

        world.insert(handlers);

        default_parser(world)
    }

    /// Override to provide a custom World when compile is called,
    ///
    fn world() -> World {
        let mut world = default_world();
        world.insert(Self::runtime());
        Self::initialize(&mut world);
        world
    }

    /// Override to provide custom node command handlers,
    ///
    fn node_handlers() -> BTreeMap<String, NodeCommandHandler> {
        default_node_handlers()
    }

    /// When compiling in the context of a project directory, the file name is taken into consideration when parsing
    /// runmd. The file name becomes the implicit symbol w/in the context of the file.
    ///
    /// In this context the root block can only be defined within in the .runmd file in the directory.
    ///
    fn compile_workspace<'a>(
        workspace: &Workspace,
        files: impl Iterator<Item = &'a RunmdFile>,
        parser: Option<Parser>,
    ) -> World {
        let mut workspace = workspace.clone();

        let mut parser = parser
            .unwrap_or(Self::parser())
            .with_special_attr::<WorkspaceConfig>()
            .with_special_attr::<Operations>();

        for RunmdFile { symbol, source } in files {
            parser.set_implicit_symbol(&symbol);

            if let Some(runmd) = source {
                parser = parser.parse(runmd);
            } else {
                let file = workspace.work_dir().join(format!("{symbol}.runmd"));
                match std::fs::read_to_string(&file) {
                    Ok(runmd) => {
                        parser = parser.parse(runmd);
                    }
                    Err(err) => {
                        event!(Level::ERROR, "Could not read file {:?} {err}", file);
                    }
                }
            }

            // Add source file for each engine
            let entity = parser.get_block("", symbol).entity();
            let entities = parser.as_ref().entities();
            let entity = entities.entity(entity);
            let runmd_file = RunmdFile {
                symbol: symbol.to_string(),
                source: source.clone(),
            };
            parser
                .as_ref()
                .write_component()
                .insert(entity, runmd_file.clone())
                .expect("should be able to insert");

            workspace.cache_file(&runmd_file);
        }

        // Parse the root file without the implicit symbol set
        parser.unset_implicit_symbol();

        let mut world = if let Some(root_runmd) = workspace.root_runmd() {
            Self::compile(root_runmd, Some(parser))
        } else {
            let root = workspace.work_dir().join(".runmd");
            match std::fs::read_to_string(&root) {
                Ok(runmd) => {
                    workspace.set_root_runmd(&runmd);
                    Self::compile(runmd, Some(parser))
                }
                Err(err) => {
                    panic!("Could not compile workspace, root .runmd file required, {err}");
                }
            }
        };

        world.insert(Some(workspace.clone()));

        // Apply config defined in root block
        {
            let mut config_data = world.system_data::<WorkspaceConfig>();
            config_data.apply();
        }

        return world;
    }

    /// Compiles runmd into blocks, interprets those blocks,
    /// and returns the World,
    ///
    /// In addition, sets up attribute graphs for each entity with a plugin.
    ///
    /// Override with care as this adds critical components for the event runtime,
    ///
    fn compile(runmd: impl AsRef<str>, parser: Option<Parser>) -> World {
        let parser = parser.unwrap_or(Self::parser()).parse(runmd.as_ref());

        let mut world = parser.commit();

        let engine = &mut Engine::default();
        engine.initialize(&mut world);

        // Setup graphs for all plugin entities
        for (entity, block_index) in
            (&world.entities(), &world.read_component::<BlockIndex>()).join()
        {
            let mut graph = AttributeGraph::new(block_index.clone());
            if entity.id() != block_index.root().id() {
                graph = graph.scope(entity.id()).expect("invalid block index state");
            }
            world
                .write_component()
                .insert(entity, graph)
                .expect("Should be able to insert graph for entity");
        }

        let blocks = { 
            world.read_component::<Block>().join().cloned().collect::<Vec<_>>()
        };

        for block in  blocks.iter() {
            engine.interpret(&world, block);

            Self::interpret(&world, block);
            event!(
                Level::TRACE,
                "Interpreted block {} {}",
                block.name(),
                block.symbol()
            );
        }

        world.maintain();
        world
    }
}

/// Returns a basic runtime with standard plugins,
///
pub fn default_runtime() -> Runtime {
    let mut runtime = Runtime::default();
    runtime.install_with_custom::<Process>("");
    runtime.install_with_custom::<Request>("");
    runtime.install_with_custom::<Install>("");
    runtime.install_with_custom::<Timer>("");
    runtime.install_with_custom::<Readln>("");
    runtime.install_with_custom::<Println>("");
    runtime.install_with_custom::<Watch>("");
    runtime.install_with_custom::<Publish>("");
    runtime.install_with_custom::<Listen>("");
    runtime.install_with_custom::<Monitor>("");

    // Plugins for testing
    runtime.install_with_custom::<Chaos>("");
    runtime.install_with_custom::<TestHost>("");
    runtime.install_with_custom::<TestHostSender>("");
    runtime
}

/// Returns a basic reality parser,
///
pub fn default_parser(world: World) -> Parser {
    Parser::new_with(world)
        .with_special_attr::<Runtime>()
        .with_special_attr::<Engine>()
}

/// Retursn the default lifec world,
///
pub fn default_world() -> World {
    let mut world = specs::World::new();
    world.register::<Thunk>();
    world.register::<Adhoc>();
    world.register::<Limit>();
    world.register::<Event>();
    world.register::<Cursor>();
    world.register::<Engine>();
    world.register::<Guest>();
    world.register::<Runtime>();
    world.register::<Sequence>();
    world.register::<Activity>();
    world.register::<Profiler>();
    world.register::<Connection>();
    world.register::<Operation>();
    world.register::<RunmdFile>();
    world.register::<Yielding>();
    world.register::<EventStatus>();
    world.register::<Transition>();
    world.register::<AttributeGraph>();
    world.insert(None::<Workspace>);
    world
}

/// Returns teh default custom node command handlers,
///
pub fn default_node_handlers() -> BTreeMap<String, NodeCommandHandler> {
    let mut handlers = BTreeMap::<String, NodeCommandHandler>::default();

    handlers.insert("delete_spawned".to_string(), |state, entity| {
        state.delete(entity);
    });

    handlers.insert("cleanup_connection".to_string(), |state, entity| {
        state.cleanup_connection(entity);
    });

    handlers
}
