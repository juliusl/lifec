use std::time::Duration;

use atlier::system::Extension;
use specs::{
    Builder, DispatcherBuilder, Entity, Join, ReadStorage, System, World, WorldExt, WriteStorage,
};
use tokio::runtime::Handle;
use tracing::{event, Level};

use crate::{
    plugins::{BlockContext, Event, EventRuntime, NetworkRuntime, Project, ThunkContext},
    CatalogReader, CatalogWriter, EventBuilder, Operation, Runtime,
};

/// Exit instructions for the callers of Host
/// 
pub enum HostExitCode {
    OK,
    RestartRequested,
    Error(String),
}

/// Consolidates most common steps for starting a runtime
///
pub trait Host 
where
    Self: Extension
{
    /// Returns a new runtime, installing any required plugins
    ///
    fn create_runtime(project: Project) -> Runtime;

    /// Returns a new event builder, that can be used to manage events
    ///
    fn create_event_builder(
        &mut self, 
        runtime: Runtime,
        block_name: impl AsRef<str>,
        block_context: &BlockContext,
    ) -> EventBuilder;

    /// Returns a new setup event builder, that can be used to manage setup events
    ///
    fn create_setup_event_builder(
        &mut self, 
        runtime: Runtime,
        block_name: impl AsRef<str>,
        block_context: &BlockContext,
    ) -> Option<EventBuilder>;

    /// Returns some operation if additional setup is required before starting the event,
    /// otherwise No-OP.
    ///
    fn prepare_engine(
        &mut self, 
        engine: Entity,
        handle: Handle,
        initial_context: ThunkContext,
        event_builder: EventBuilder,
    ) -> Option<Operation>;

    /// Returns true if the host should exit
    /// 
    fn should_exit(&mut self) -> Option<HostExitCode>; 

    /// Starts a host runtime w/ a given project
    ///
    fn start(&mut self, project: Project) -> HostExitCode {
        let mut world = World::new();
        let world = &mut world;
        let mut dispatcher = DispatcherBuilder::new();

        EventRuntime::configure_app_systems(&mut dispatcher);
        EventRuntime::configure_app_world(world);

        NetworkRuntime::configure_app_systems(&mut dispatcher);
        NetworkRuntime::configure_app_world(world);

        dispatcher.add(HostSetup {}, "host_setup", &["event_runtime"]);
        dispatcher.add(HostStartup {}, "host_startup", &["event_runtime"]);

        let mut dispatcher = dispatcher.build();
        dispatcher.setup(world);

        let handle = {
            let tokio_runtime = &world.read_resource::<tokio::runtime::Runtime>();
            tokio_runtime.handle().clone()
        };

        let mut engines = vec![];
        for (block_name, block_context) in project.clone().iter_block() {
            let mut thunk_context = ThunkContext::default();
            thunk_context.block = block_context.to_owned();

            let runtime = Self::create_runtime(project.clone());
            let event_builder =
                self.create_event_builder(runtime.clone(), block_name, block_context);

            let engine = world
                .create_entity()
                .with(thunk_context.clone())
                .with(runtime.clone())
                .with(event_builder.event)
                .build();

            if let Some(setup_event_builder) =
                self.create_setup_event_builder(runtime, block_name, block_context)
            {
                if let Some(operation) = self.prepare_engine(
                    engine,
                    handle.clone(),
                    thunk_context.clone(),
                    setup_event_builder,
                ) {
                    match world.write_component().insert(engine, operation) {
                        Ok(_) => {
                            event!(Level::DEBUG, "Inserted setup operation for {block_name}");
                        }
                        Err(err) => {
                            event!(
                                Level::ERROR,
                                "Could not insert setup operation for {block_name}, {err}"
                            );
                        }
                    }
                }
            } else {
                // If a setup operation isn't required, the engine will start immediately
                engines.push(engine);
            }
        }
        world.maintain();

        loop {
            dispatcher.dispatch(&world);
            self.on_run(&world);

            world.maintain();
            self.on_maintain(world);

            if let Some(exit_code) = self.should_exit() {
                if let Some(runtime) = world.remove::<tokio::runtime::Runtime>() {
                    // dropping a tokio runtime needs to happen in a blocking context
                    handle.clone().spawn_blocking(move || {
                        runtime.shutdown_timeout(Duration::from_secs(5));
                    });
                }

                return exit_code;
            }
        }
    }
}

/// System that handles event setup and execution
///
struct HostSetup;

impl<'a> System<'a> for HostSetup {
    type SystemData = (CatalogWriter<'a, Operation>, WriteStorage<'a, Event>);

    fn run(
        &mut self,
        (
            CatalogWriter {
                entities,
                mut items,
            },
            mut events,
        ): Self::SystemData,
    ) {
        for (entity, operation) in (&entities, &mut items).join() {
            if let Some(tc) = operation.wait_if_ready() {
                match events.get_mut(entity) {
                    Some(event) => {
                        event.fire(tc);
                    }
                    None => {}
                }
            }
        }
    }
}

struct HostStartup;

impl<'a> System<'a> for HostStartup {
    type SystemData = (
        CatalogReader<'a, Operation>,
        ReadStorage<'a, ThunkContext>,
        WriteStorage<'a, Event>,
    );

    fn run(&mut self, (CatalogReader { entities, items }, contexts, mut events): Self::SystemData) {
        for (entity, operation, context) in (&entities, items.maybe(), &contexts).join() {
            if let None = operation {
                match events.get_mut(entity) {
                    Some(ref mut event) if !event.is_running() => {
                        // The event runtime is responsible for setting the entity
                        // If the event has an entity, it means that it has already ran at least once
                        if context.entity.is_none() {
                            event.fire(context.clone());
                        }
                        // TODO: Handle this case?
                    }
                    _ => {}
                }
            }
        }
    }
}
