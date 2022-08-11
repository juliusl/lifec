use std::time::Duration;

use atlier::system::Extension;
use specs::{
    Builder, DispatcherBuilder, Entity, Join, ReadStorage, System, World, WorldExt, WriteStorage,
};
use tokio::runtime::Handle;
use tracing::{event, Level};

use crate::{
    plugins::{BlockContext, Event, EventRuntime, NetworkRuntime, Project, ThunkContext},
    CatalogReader, CatalogWriter, EventSource, Operation, Runtime,
};

/// Exit instructions for the callers of Host
///
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum HostExitCode {
    OK,
    RestartRequested,
    Error(String),
}

/// Consolidates common procedures for hosting a runtime
///
pub trait Host
where
    Self: Extension,
{
    /// Returns a new runtime for this host
    ///
    /// Types implementing this trait should install any plugins
    /// and add any configs, that will be needed to start engine
    /// blocks defined in the project
    ///
    fn create_runtime(project: Project) -> Runtime;

    /// Creates a new event source for the engine
    ///
    /// Implementing types receive the entire block context, and can
    /// choose to interpret the block in any way they choose when returning
    /// the event source.
    ///
    fn create_event_source(
        &mut self,
        runtime: Runtime,
        block_name: impl AsRef<str>,
        block_context: &BlockContext,
    ) -> EventSource;

    /// Creates a setup event source if needed
    ///
    fn create_setup_event_source(
        &mut self,
        runtime: Runtime,
        block_name: impl AsRef<str>,
        block_context: &BlockContext,
    ) -> Option<EventSource>;

    /// Returns some operation if additional setup is required before starting the event,
    /// otherwise No-OP.
    ///
    /// The initial context passed here will contain the attributes and blocks
    /// defined in the project.
    ///
    fn prepare_engine(
        &mut self,
        engine: Entity,
        handle: Handle,
        initial_context: &ThunkContext,
        setup_event_source: Option<EventSource>,
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

        Self::configure_app_systems(&mut dispatcher);
        Self::configure_app_world(world);

        dispatcher.add(HostSetup {}, "", &["event_runtime"]);
        dispatcher.add(HostStartup {}, "", &["event_runtime"]);

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
            let engine_event_source =
                self.create_event_source(runtime.clone(), block_name, block_context);

            thunk_context
                .as_mut()
                .add_text_attr("event_symbol", engine_event_source.event.symbol());

            let engine = world
                .create_entity()
                .with(thunk_context.clone())
                .with(runtime.clone())
                .with(engine_event_source.event)
                .build();
            
            let setup_event_source = self.create_setup_event_source(runtime, block_name, block_context);

            if let Some(setup_operation) =
                self.prepare_engine(engine, handle.clone(), &thunk_context, setup_event_source)
            {
                match world.write_component().insert(engine, setup_operation) {
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

/// System that handles engine setup operations
///
/// If an engine requires an operation before operating, this system
/// will monitor that operation, and start the engine after the
/// operation completes.
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

/// System that starts engines
///
/// Checks to see if an engine has an outstanding operation, if not starts
/// the engine if hasn't started already.
///
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

mod test {
    use crate::{
        plugins::{Println, Test},
        Extension, Host, HostExitCode, Runtime,
    };

    struct TestHost(usize);

    impl Extension for TestHost {}

    impl Host for TestHost {
        fn create_runtime(project: crate::plugins::Project) -> Runtime {
            let mut runtime = Runtime::new(project);
            runtime.install::<Test, Println>();
            runtime
        }

        fn create_event_source(
            &mut self,
            runtime: crate::Runtime,
            block_name: impl AsRef<str>,
            _block_context: &crate::plugins::BlockContext,
        ) -> crate::EventSource {
            assert_eq!(block_name.as_ref(), "test_host");

            if _block_context.get_block("test").is_some() {
                let mut src = runtime.event_source::<Test, Println>();
                // Tests that the context will be configured from the correct
                // block in the project (test_host test)
                src.set_config_from_project();
                return src;
            }

            panic!("block context did not have a `test` block");
        }

        fn should_exit(&mut self) -> Option<crate::HostExitCode> {
            self.0 -= 1;

            if self.0 == 0 {
                Some(HostExitCode::OK)
            } else {
                None
            }
        }

        // TODO:
        fn create_setup_event_source(
            &mut self,
            _runtime: crate::Runtime,
            block_name: impl AsRef<str>,
            _block_context: &crate::plugins::BlockContext,
        ) -> Option<crate::EventSource> {
            assert_eq!(block_name.as_ref(), "test_host");
            None
        }

        fn prepare_engine(
            &mut self,
            _engine: specs::Entity,
            _handle: tokio::runtime::Handle,
            _initial_context: &crate::plugins::ThunkContext,
            _event_builder: Option<crate::EventSource>,
        ) -> Option<crate::Operation> {
            None
        }
    }

    #[test]
    #[tracing_test::traced_test]
    fn test_host() {
        use crate::Project;
        let code = TestHost(10).start(
            Project::load_content(
                r#"
            # Project Settings
            ```
            - add debug   .enable
            ```

            # Test Host Impl 
            ``` test_host test
            add name    .text bob
            ```
            "#,
            )
            .expect("valid .runmd"),
        );

        assert_eq!(code, HostExitCode::OK)
    }
}
