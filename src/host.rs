use std::time::Duration;

use atlier::system::Extension;
use specs::{
    Builder, DispatcherBuilder, Entity, Join, ReadStorage, System, World, WorldExt, WriteStorage,
};
use tokio::{runtime::Handle};
use tracing::{event, Level};

use crate::{
    plugins::{BlockContext, Event, EventRuntime, NetworkRuntime, Project, ThunkContext},
    AttributeIndex, CatalogReader, CatalogWriter, EventSource, Operation, Runtime,
};

mod open;
pub use open::open;

mod dashboard;
pub use dashboard::Dashboard;

mod transport;
pub use transport::Transport;
pub use transport::TransportReceiver;

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
        dispatcher: &mut DispatcherBuilder,
    ) -> EventSource;

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
    ) -> Option<Operation>;

    /// Returns true if the host should exit
    ///
    fn should_exit(&mut self) -> Option<HostExitCode>;

    /// Starts a host runtime w/ a given project
    ///
    fn start(&mut self, project: Project) -> HostExitCode {
        let mut world = World::new();
        let world = &mut world;

        // Usually the event_runtime would set this up, 
        // But we do this early because we need to setup the dispatcher late,
        // and we want the handle early
        let tokio_runtime = tokio::runtime::Runtime::new().unwrap();
        world.insert(tokio_runtime);
        
        let mut dispatcher = DispatcherBuilder::new();

        EventRuntime::configure_app_systems(&mut dispatcher);
        EventRuntime::configure_app_world(world);

        NetworkRuntime::configure_app_systems(&mut dispatcher);
        NetworkRuntime::configure_app_world(world);

        Self::configure_app_systems(&mut dispatcher);
        Self::configure_app_world(world);

        dispatcher.add(HostSetup {}, "", &[]);
        dispatcher.add(HostStartup {}, "", &[]);

        let handle = {
            let tokio_runtime = &world.read_resource::<tokio::runtime::Runtime>();
            tokio_runtime.handle().clone()
        };

        let mut engines = vec![];
        for (block_name, block_context) in project.clone().iter_block() {
            let mut thunk_context = ThunkContext::default();
            thunk_context.block = block_context.to_owned();

            let runtime = Self::create_runtime(project.clone());
            let engine_event_source = self.create_event_source(
                runtime.clone(),
                block_name,
                block_context,
                &mut dispatcher,
            );

            thunk_context
                .as_mut()
                .add_text_attr("event_symbol", engine_event_source.event.symbol());

            let engine = world
                .create_entity()
                .with(thunk_context.clone())
                .with(runtime.clone())
                .with(engine_event_source.event)
                .build();

            if let Some(setup_operation) =
                self.prepare_engine(engine, handle.clone(), &thunk_context)
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

        let mut dispatcher = dispatcher.build();
        dispatcher.setup(world);
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
    
    /// Install a host runtime w/ the given settings
    /// 
    /// Panics, if this is called more than once.
    /// 
    fn install_host_runtime<I, T>(
        settings: (I, T), 
        dispatcher: &mut DispatcherBuilder
    )
    where
        I: AttributeIndex,
        T: Transport + Send + Sync + 'static,
    {
        dispatcher.add(
            HostRuntime::from(settings), 
            "host_runtime", 
            &[]
        );
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

/// System to receive dispatched objects at runtime,
///
/// These objects include,
///     `AttributeGraph`
///     `Operation`
///     `ErrorContext`
///
struct HostRuntime<T>
where
    T: Transport,
{
    transport: T,
    enable_graph_receiver: bool,
    enable_operation_receiver: bool,
    enable_error_context_receiver: bool,
}

impl<A, T> From<(A, T)> for HostRuntime<T>
where
    A: AttributeIndex,
    T: Transport,
{
    fn from((index, transport): (A, T)) -> Self {
        let enable_graph_receiver = index
            .find_bool("enable_graph_receiver")
            .unwrap_or_default();
        let enable_operation_receiver = index
            .find_bool("enable_operation_receiver")
            .unwrap_or_default();
        let enable_error_context_receiver = index
            .find_bool("enable_error_context_receiver")
            .unwrap_or_default();
            
        Self {
            transport,
            enable_graph_receiver,
            enable_operation_receiver,
            enable_error_context_receiver,
        }
    }
}

impl<'a, T> System<'a> for HostRuntime<T>
where
    T: Transport,
{
    type SystemData = TransportReceiver<'a>;

    fn run(&mut self, mut receiver: Self::SystemData) {
        if self.enable_graph_receiver {
            receiver.receive_graph(&mut self.transport);
        }

        if self.enable_operation_receiver {
            receiver.receive_operation(&mut self.transport);
        }

        if self.enable_error_context_receiver {
            receiver.receive_error_context(&mut self.transport);
        }
    }
}

mod test {
    use std::sync::Arc;

    use specs::DispatcherBuilder;
    use tracing::{event, Level};

    use crate::{
        plugins::{Plugin, Println, Test, ThunkContext, Engine},
        AttributeIndex, Extension, Operation, Runtime,
    };

    use super::{Host, HostExitCode, Transport};

    /// Simple test host implementation, for additional coverage
    ///
    struct TestHost(
        /// iterations before exiting
        usize,
    );

    impl Extension for TestHost {}

    impl Host for TestHost {
        fn create_runtime(project: crate::plugins::Project) -> Runtime {
            let mut runtime = Runtime::new(project);
            runtime.install::<Test, Println>();
            runtime.install::<Test, ChangeName>();
            runtime.install::<Test, IsBob>();
            runtime.install::<Test, IsNotBob>();
            runtime
        }

        /// Tests that the expected block_context is passed here
        /// Tests that the expected block exists
        ///
        fn create_event_source(
            &mut self,
            runtime: crate::Runtime,
            block_name: impl AsRef<str>,
            _block_context: &crate::plugins::BlockContext,
            dispatcher: &mut DispatcherBuilder,
        ) -> crate::EventSource {
            assert_eq!(block_name.as_ref(), "test_host");

            if let Some(test_graph) = _block_context.get_block("test") {
                let mut src = runtime.event_source::<Test, (IsBob, Println)>();
                // Tests that the context will be configured from the correct
                // block in the project (test_host test)
                src.set_config_from_project();

                // Tests installing a host runtime
                Self::install_host_runtime(
                    (
                        test_graph, 
                        TestTransport{}
                    ), 
                    dispatcher
                );

                return src;
            }

            panic!("block context did not have a `test` block");
        }

        /// TODO
        ///
        fn should_exit(&mut self) -> Option<HostExitCode> {
            self.0 -= 1;

            if self.0 == 0 {
                Some(HostExitCode::OK)
            } else {
                None
            }
        }

        /// Test basic example of generating a setup operation
        ///
        fn prepare_engine(
            &mut self,
            _engine: specs::Entity,
            _handle: tokio::runtime::Handle,
            _initial_context: &crate::plugins::ThunkContext,
        ) -> Option<crate::Operation> {
            if let Some(query) = _initial_context.block.find_query() {
                event!(Level::TRACE, "found query {:#?}", query);
                // Test generating a "setup" operation

                let item = Operation::item(_engine, _handle);
                let thunk = query.thunk(
                    item,
                    Some(
                        (
                            (
                                // Tests that the name is changed
                                ChangeName(),
                                IsNotBob(),
                            ),
                            // Optional, debug println
                            Println::default(),
                        )
                            .as_thunk(),
                    ),
                );

                // Test passing a src to the thunk, so that the operation
                // is initialized before being executed
                //
                // Note: in this context, entity id doesn't matter because an alt
                // src will be used. The id matters if the initial src is being used
                // This should hopefully be an implementation detail.
                let src = _initial_context
                    .block
                    .get_block("test")
                    .expect("test block is defined");

                return Some(thunk(Arc::new(src)));
            }

            None
        }
    }

    #[derive(Default)]
    struct ChangeName();

    impl Plugin for ChangeName {
        fn symbol() -> &'static str {
            "change_name"
        }

        fn call_with_context(context: &mut ThunkContext) -> Option<crate::plugins::AsyncContext> {
            context.clone().task(|_| {
                let mut tc = context.clone();
                async move {
                    event!(Level::DEBUG, "previous name {:?}", tc.find_text("name"));

                    tc.as_mut().add_text_attr("name", "not-bob");
                    event!(Level::DEBUG, "changing names");
                    Some(tc)
                }
            })
        }
    }

    #[derive(Default)]
    struct IsBob();

    impl Plugin for IsBob {
        fn symbol() -> &'static str {
            "is_bob"
        }

        fn call_with_context(context: &mut ThunkContext) -> Option<crate::plugins::AsyncContext> {
            context.clone().task(|_| {
                let tc = context.clone();
                async move {
                    assert_eq!(tc.find_text("name"), Some("bob".to_string()));

                    event!(Level::TRACE, "checked if this is bob");

                    None
                }
            })
        }
    }

    #[derive(Default)]
    struct IsNotBob();

    impl Plugin for IsNotBob {
        fn symbol() -> &'static str {
            "is_not_bob"
        }

        fn call_with_context(context: &mut ThunkContext) -> Option<crate::plugins::AsyncContext> {
            context.clone().task(|_| {
                let tc = context.clone();
                async move {
                    assert_eq!(tc.find_text("name"), Some("not-bob".to_string()));

                    event!(Level::TRACE, "checked if this is not-bob");

                    None
                }
            })
        }
    }

    struct TestTransport;

    impl Engine for TestTransport {
        fn event_name() -> &'static str {
            "test_transport"
        }
    }

    impl Transport for TestTransport {
        type Plugin = Println;

        fn transport_graph(
            &mut self,
            _graph: crate::AttributeGraph,
            _system_data: &mut super::TransportReceiver,
        ) {
            event!(Level::TRACE, "transport graph called")
        }

        fn transport_operation(
            &mut self,
            _operation: Operation,
            _system_data: &mut super::TransportReceiver,
        ) {
            event!(Level::TRACE, "transport operation called")
        }

        fn transport_error_context(
            &mut self,
            _error_context: crate::plugins::ErrorContext,
            _system_data: &mut super::TransportReceiver,
        ) {
            event!(Level::TRACE, "transport error context called")
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

            add enable_graph_receiver           .enable
            add enable_operation_receiver       .enable
            add enable_error_context_receiver   .enable

            ``` query
            define name find .search_text
            ```
            "#,
            )
            .expect("valid .runmd"),
        );

        assert_eq!(code, HostExitCode::OK)
    }
}
