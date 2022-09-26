// use std::{collections::HashMap, time::Duration};

// use atlier::system::Extension;
// use specs::{
//     Builder, Dispatcher, DispatcherBuilder, Entities, Entity, Join, ReadStorage, System, World,
//     WorldExt, WriteStorage,
// };

// use crate::{
//     plugins::{Event, EventRuntime, NetworkRuntime, Project, ThunkContext},
//     AttributeIndex, CatalogReader, CatalogWriter, EventSource, Operation, Runtime, AttributeGraph,
// };

// mod dashboard;
// pub use dashboard::Dashboard;

// mod transport;
// pub use transport::ProxyTransport;
// pub use transport::TestTransport;
// pub use transport::Transport;
// pub use transport::TransportReceiver;

mod guest_runtime;
pub use guest_runtime::GuestRuntime;

// /// Exit instructions for the callers of Host
// ///
// #[derive(Debug, PartialEq, Eq, Clone)]
// pub enum HostExitCode {
//     OK,
//     RestartRequested,
//     Error(String),
// }

// /// Consolidates common procedures for hosting a runtime
// /// 
// /// # Runmd interpretation
// /// 
// /// Explanation on how runmd will be interpreted by this implementation.
// /// 
// /// * Block address - {event_symbol} {plugin_symbol}  
// /// 
// pub trait Host
// where
//     Self: Extension,
// {
//     /// Returns a new runtime for this host
//     ///
//     /// Types implementing this trait should install any plugins
//     /// and add any configs, that will be needed to start engine
//     /// blocks defined in the project
//     ///
//     fn create_runtime(&mut self, project: Project) -> Runtime;

//     /// Add's a guest to the host
//     ///
//     /// Called by the guest runtime when a guest runtime is
//     /// created.
//     ///
//     fn add_guest(&mut self, host: Entity, dispatcher: Dispatcher<'static, 'static>);

//     /// Activate the guest by taking the dispatcher for the guest world
//     ///
//     /// Called by the guest runtime when a guest runtime is
//     /// created.
//     ///
//     fn activate_guest(&mut self, host: Entity) -> Option<Dispatcher<'static, 'static>>;

//     /// Gets the runtime for an engine
//     ///
//     /// Called by the guest runtime when a guest runtime is
//     /// created.
//     ///
//     fn get_runtime(&mut self, engine: Entity) -> Runtime;

//     /// Inserts the runtime for engine for later lookup
//     ///
//     /// This is called during .start()
//     ///
//     fn add_runtime(&mut self, engine: Entity, runtime: Runtime);

//     // /// Visit guests of host
//     // ///
//     // fn visit_guests(
//     //     &mut self,
//     //     visitor: impl FnOnce(&mut Dispatcher<'static, 'static>)
//     // );

//     /// Returns true if the host should exit
//     ///
//     fn should_exit(&mut self) -> Option<HostExitCode>;

//     /// Returns some operation if additional setup is required before starting the event,
//     /// otherwise No-OP.
//     ///
//     /// The initial context passed here will contain the attributes and blocks
//     /// defined in the project.
//     ///
//     fn prepare_engine(
//         &mut self,
//         guest_runtime: GuestRuntime,
//         interpreted_block: Vec<(EventSource, AttributeGraph)>,
//         world: &mut World,
//         dispatcher: &mut DispatcherBuilder,
//     ) -> Option<Operation> {

//         for (source, config) in interpreted_block {
//             let source = source.clone();
//             if let Some(guest_engine) = source.create_entity(guest_runtime.world()) {
                
//             }
//         }

//         None
//     }

//     fn prepare_guest_runtime(
//         &mut self, 
//         guest_runtime: GuestRuntime
//     ) {
        
//     }

//     /// Starts a host runtime w/ a given project
//     ///
//     fn start(&mut self, project: Project) -> HostExitCode {
//         let (mut world, mut dispatcher) = Self::new_world();

//         let handle = {
//             let tokio_runtime = &world.read_resource::<tokio::runtime::Runtime>();
//             tokio_runtime.handle().clone()
//         };

//         let mut host_runtime = HostRuntime::default();
//         for (block_name, block_context) in project.clone().iter_block() {
//             let mut thunk_context = ThunkContext::default();
//             thunk_context.block = block_context.to_owned();

//             let runtime = self.create_runtime(project.clone());
//             // let engine_event_source =
//             //     self.parse_block_context(
//             //         runtime.clone(), 
//             //         block_name, 
//             //         block_context
//             //     );

//             let engine = world
//                 .create_entity()
//                 .with(thunk_context.clone())
//                 .with(runtime.clone())
//                 // .with(engine_event_source.event)
//                 .build();

//             self.add_runtime(engine, runtime.clone());

//             // let setup_operation = self.prepare_engine(
//             //     engine,
//             //     handle.clone(),
//             //     &mut world,
//             //     &mut dispatcher,
//             //     &thunk_context,
//             // );

//             // if let Some(setup_operation) = setup_operation {
//             //     match world.write_component().insert(engine, setup_operation) {
//             //         Ok(_) => {
//             //             event!(Level::DEBUG, "Inserted setup operation for {block_name}");
//             //         }
//             //         Err(err) => {
//             //             event!(
//             //                 Level::ERROR,
//             //                 "Could not insert setup operation for {block_name}, {err}"
//             //             );
//             //         }
//             //     }
//             // }

//             // The host runtime system will drive guests
//             if let Some(guest) = self.activate_guest(engine) {
//                 host_runtime.guests.insert(engine, guest);
//             }
//         }

//         // Host runtime is definitely not send + sync
//         // therefore we add as thread local system
//         dispatcher.add_thread_local(host_runtime);

//         let mut dispatcher = dispatcher.build();
//         dispatcher.setup(&mut world);
//         world.maintain();

//         loop {
//             dispatcher.dispatch(&world);
//             self.on_run(&world);

//             world.maintain();
//             self.on_maintain(&mut world);

//             if let Some(exit_code) = self.should_exit() {
//                 if let Some(runtime) = world.remove::<tokio::runtime::Runtime>() {
//                     // dropping a tokio runtime needs to happen in a blocking context
//                     handle.clone().spawn_blocking(move || {
//                         runtime.shutdown_timeout(Duration::from_secs(5));
//                     });
//                 }

//                 return exit_code;
//             }
//         }
//     }

//     /// Creates a new world and dispatcher builder
//     ///
//     fn new_world<'a, 'b>() -> (World, DispatcherBuilder<'a, 'b>) {
//         // TODO: Get this from reality 
//         let mut world = World::new();
//         world.register::<GuestRuntime>();

//         // Usually the event_runtime would set this up,
//         // But we do this early because we need to setup the dispatcher late,
//         // and we want the handle early
//         let tokio_runtime = tokio::runtime::Runtime::new().unwrap();
//         world.insert(tokio_runtime);

//         let mut dispatcher = DispatcherBuilder::new();

//         EventRuntime::configure_app_systems(&mut dispatcher);
//         EventRuntime::configure_app_world(&mut world);

//         NetworkRuntime::configure_app_systems(&mut dispatcher);
//         NetworkRuntime::configure_app_world(&mut world);

//         Self::configure_app_systems(&mut dispatcher);
//         Self::configure_app_world(&mut world);

//         dispatcher.add(HostSetup {}, "", &[]);
//         dispatcher.add(HostStartup {}, "", &[]);

//         (world, dispatcher)
//     }

//     /// Creates a guest runtime component for a transport,
//     /// Registers the component w/ the host world and inserts the component into the world
//     ///
//     fn create_guest(
//         &mut self,
//         engine: Entity,
//         src: impl AttributeIndex,
//         transport: &mut impl Transport,
//     ) -> GuestRuntime
//     where
//         Self: Sized,
//     {
//         // Creates a new guest runtime
//         GuestRuntime::new(
//             engine, 
//             self, 
//             src, 
//             transport
//         )
//     }
// }

// /// System that handles engine setup operations
// ///
// /// If an engine requires an operation before operating, this system
// /// will monitor that operation, and start the engine after the
// /// operation completes.
// ///
// struct HostSetup;

// impl<'a> System<'a> for HostSetup {
//     type SystemData = (CatalogWriter<'a, Operation>, WriteStorage<'a, Event>);

//     fn run(
//         &mut self,
//         (
//             CatalogWriter {
//                 entities,
//                 mut items,
//             },
//             mut events,
//         ): Self::SystemData,
//     ) {
//         for (entity, operation) in (&entities, &mut items).join() {
//             if let Some(tc) = operation.wait_if_ready() {
//                 match events.get_mut(entity) {
//                     Some(event) => {
//                         event.fire(tc);
//                     }
//                     None => {}
//                 }
//             }
//         }
//     }
// }

// /// System that starts engines
// ///
// /// Checks to see if an engine has an outstanding operation, if not starts
// /// the engine if hasn't started already.
// ///
// struct HostStartup;

// impl<'a> System<'a> for HostStartup {
//     type SystemData = (
//         CatalogReader<'a, Operation>,
//         ReadStorage<'a, ThunkContext>,
//         WriteStorage<'a, Event>,
//     );

//     fn run(&mut self, (CatalogReader { entities, items }, contexts, mut events): Self::SystemData) {
//         for (entity, operation, context) in (&entities, items.maybe(), &contexts).join() {
//             if let None = operation {
//                 match events.get_mut(entity) {
//                     Some(ref mut event) if !event.is_running() => {
//                         // The event runtime is responsible for setting the entity
//                         // If the event has an entity, it means that it has already ran at least once
//                         if context.entity.is_none() {
//                             event.fire(context.clone());
//                         }
//                         // TODO: Handle this case?
//                     }
//                     _ => {}
//                 }
//             }
//         }
//     }
// }

// /// Manages driving guest runtimes
// ///
// #[derive(Default)]
// struct HostRuntime {
//     guests: HashMap<Entity, Dispatcher<'static, 'static>>,
// }

// impl<'a> System<'a> for HostRuntime {
//     type SystemData = (Entities<'a>, WriteStorage<'a, GuestRuntime>);

//     fn run(&mut self, (hosts, mut guests): Self::SystemData) {
//         for (host, guest) in (&hosts, &mut guests).join() {
//             if let Some(guest_dispatcher) = self.guests.get_mut(&host) {
//                 guest.run(());
//                 guest_dispatcher.dispatch(guest.world());
//                 guest.world_mut().maintain();
//             }
//         }
//     }
// }

// mod test {
//     use specs::Entity;
//     use std::collections::HashMap;
//     use tracing::{event, Level};

//     use crate::{
//         plugins::{Plugin,  Test, ThunkContext},
//         AttributeIndex, Extension, Runtime,
//     };

//     use super::{Host, HostExitCode};

//     /// Simple test host implementation, for additional coverage
//     ///
//     struct TestHost(
//         /// iterations before exiting
//         usize,
//         /// test guest
//         Option<specs::Dispatcher<'static, 'static>>,

//         HashMap<Entity, Runtime>,
//     );

//     impl Extension for TestHost {}

//     impl Host for TestHost {
//         fn create_runtime(&mut self, project: crate::plugins::Project) -> Runtime {
//             let mut runtime = Runtime::default();
//             runtime.install::<ChangeName>("test");
//             runtime.install::<IsBob>("test");
//             runtime.install::<IsNotBob>("test");
//             runtime
//         }

//         fn get_runtime(&mut self, engine: Entity) -> Runtime {
//             self.2.get(&engine).expect("exists").clone()
//         }

//         fn add_runtime(&mut self, engine: Entity, runtime: Runtime) {
//             self.2.insert(engine, runtime);
//         }

//         /// Tests that guest is added
//         ///
//         fn add_guest(&mut self, _engine: Entity, _dispatcher: specs::Dispatcher<'static, 'static>) {
//             self.1 = Some(_dispatcher);
//         }

//         fn activate_guest(&mut self, _host: Entity) -> Option<specs::Dispatcher<'static, 'static>> {
//             self.1.take()
//         }

//         /// TODO
//         ///
//         fn should_exit(&mut self) -> Option<HostExitCode> {
//             self.0 -= 1;

//             if self.0 == 0 {
//                 Some(HostExitCode::OK)
//             } else {
//                 None
//             }
//         }
//     }

//     #[derive(Default)]
//     struct ChangeName();

//     impl Plugin for ChangeName {
//         fn symbol() -> &'static str {
//             "change_name"
//         }

//         fn call(context: &ThunkContext) -> Option<crate::plugins::AsyncContext> {
//             context.clone().task(|_| {
//                 let mut tc = context.clone();
//                 async move {
//                     event!(Level::DEBUG, "previous name {:?}", tc.find_text("name"));

//                     tc.state().add_text_attr("name", "not-bob");
//                     event!(Level::DEBUG, "changing names");
//                     Some(tc)
//                 }
//             })
//         }
//     }

//     #[derive(Default)]
//     struct IsBob();

//     impl Plugin for IsBob {
//         fn symbol() -> &'static str {
//             "is_bob"
//         }

//         fn call(context: &ThunkContext) -> Option<crate::plugins::AsyncContext> {
//             context.clone().task(|_| {
//                 let tc = context.clone();
//                 async move {
//                     assert_eq!(tc.find_text("name"), Some("bob".to_string()));

//                     event!(Level::TRACE, "checked if this is bob");

//                     tc.dispatch(
//                         r#"
//                     ``` test println
//                     add test .text hi
//                     ```
//                     "#,
//                     )
//                     .await;

//                     None
//                 }
//             })
//         }
//     }

//     #[derive(Default)]
//     struct IsNotBob();

//     impl Plugin for IsNotBob {
//         fn symbol() -> &'static str {
//             "is_not_bob"
//         }

//         fn call(context: &ThunkContext) -> Option<crate::plugins::AsyncContext> {
//             context.clone().task(|_| {
//                 let tc = context.clone();
//                 async move {
//                     assert_eq!(tc.find_text("name"), Some("not-bob".to_string()));

//                     event!(Level::TRACE, "checked if this is not-bob");

//                     None
//                 }
//             })
//         }
//     }

//     #[test]
//     #[tracing_test::traced_test]
//     fn test_host() {
//         use crate::Project;
//         let code = TestHost(100, None, HashMap::default()).start(
//             Project::load_content(
//                 r#"
//             # Project Settings
//             ```
//             - add debug   .enable
//             ```

//             # Test Host Impl 
//             ``` test_host test
//             add name    .text bob

//             add enable_graph_receiver           .enable
//             add enable_operation_receiver       .enable
//             add enable_error_context_receiver   .enable

//             ``` query
//             define name find .search_text
//             ```
//             "#,
//             )
//             .expect("valid .runmd"),
//         );

//         assert_eq!(code, HostExitCode::OK)
//     }
// }
