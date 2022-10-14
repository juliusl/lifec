use clap::Args;
use hyper::{Client, Uri};
use hyper_tls::HttpsConnector;
use specs::{Dispatcher, DispatcherBuilder, Entity, Join, World, WorldExt, WriteStorage};
use std::{error::Error, fmt::Debug, path::PathBuf, str::from_utf8, sync::Arc};
use tracing::{event, Level};

use crate::{
    plugins::{CancelThunk, EventRuntime},
    project::{
        CompletedPluginListener, ErrorContextListener, OperationListener, RunmdListener,
        StartCommandListener,
    },
    Engine, Event, LifecycleOptions, Project, ThunkContext,
};

mod traverse;
pub use traverse::Traverse;

mod inspector;
pub use inspector::Inspector;

mod start;
pub use start::Start;

mod commands;
pub use commands::Commands;

mod sequencer;
pub use sequencer::Sequencer;

mod executor;
pub use executor::Executor;

mod editor;
pub use editor::Editor;

mod runner;
pub use runner::Runner;

/// Struct for initializing and hosting the runtime as well as parsing CLI arguments,
///
/// Used with a type that implements the Project trait.
///
#[derive(Default, Args)]
#[clap(arg_required_else_help = true)]
pub struct Host {
    /// URL to .runmd file used to configure this host,
    ///
    #[clap(long)]
    pub url: Option<String>,
    /// Path to runmd file used to configure this host,
    /// Defaults to .runmd,
    ///
    #[clap(long)]
    pub runmd_path: Option<String>,
    /// The command to execute w/ this host,
    ///
    #[clap(subcommand)]
    pub command: Option<Commands>,
    /// The compiled specs World,
    ///
    #[clap(skip)]
    pub world: Option<World>,
}

/// CLI functions
///
impl Host {
    /// Handles the current command
    ///
    pub fn handle_start<P>(&mut self)
    where
        P: Project,
    {
        match self.command() {
            Some(Commands::Start(Start { id: Some(id), .. })) => {
                event!(Level::DEBUG, "Starting engine by id {id}");
                self.start::<P>(*id, None);
            }
            Some(Commands::Start(Start {
                engine_name: Some(engine_name),
                ..
            })) => {
                event!(Level::DEBUG, "Starting engine by name {engine_name}");
                self.start_with::<P>(engine_name.clone());
            }
            _ => {
                unreachable!("A command should exist by this point")
            }
        }
    }

    /// Returns the current command,
    ///
    pub fn command(&self) -> Option<&Commands> {
        self.command.as_ref()
    }

    /// Sets the command argument,
    ///
    pub fn set_command(&mut self, command: Commands) {
        self.command = Some(command);
    }

    /// Sets the runmd path argument, if None defaults to ./.runmd
    ///
    pub fn set_path(&mut self, path: impl AsRef<str>) {
        self.runmd_path = Some(path.as_ref().to_string());
    }

    /// Sets the runmd url argument,
    ///
    pub fn set_url(&mut self, url: impl AsRef<str>) {
        self.url = Some(url.as_ref().to_string());
    }

    /// Creates a new lifec host,
    ///
    /// Will parse runmd from either a url, local file path, or current directory
    ///
    pub async fn create_host<P>(&self) -> Option<Host>
    where
        P: Project,
    {
        let command = self.command().cloned();
        match self {
            Self { url: Some(url), .. } => match Host::get::<P>(url).await {
                Ok(mut host) => {
                    host.command = command;
                    return Some(host);
                }
                Err(err) => {
                    event!(Level::ERROR, "Could not get runmd from url {url}, {err}");
                    return None;
                }
            },
            Self {
                runmd_path: Some(runmd_path),
                ..
            } => {
                let mut runmd_path = PathBuf::from(runmd_path);
                if runmd_path.is_dir() {
                    runmd_path = runmd_path.join(".runmd");
                }

                match Host::open::<P>(runmd_path).await {
                    Ok(mut host) => {
                        host.command = command;
                        Some(host)
                    }
                    Err(err) => {
                        event!(Level::ERROR, "Could not load runmd from path {err}");
                        None
                    }
                }
            }
            _ => match Host::runmd::<P>().await {
                Ok(mut host) => {
                    host.command = command;
                    Some(host)
                }
                Err(err) => {
                    event!(
                        Level::ERROR,
                        "Could not load `.runmd` from current directory {err}"
                    );
                    None
                }
            },
        }
    }
}

/// ECS configuration,
///
impl Host {
    /// Returns a new dispatcher builder with core
    /// systems included.
    ///
    /// When adding additional systems, the below systems can be used as dependencies
    ///
    /// # Systems Included:
    /// * event_runtime - System that manages running engines.
    ///
    pub fn dispatcher_builder<'a, 'b>() -> DispatcherBuilder<'a, 'b> {
        let dispatcher_builder = DispatcherBuilder::new();

        dispatcher_builder.with(EventRuntime::default(), "event_runtime", &[])
    }

    /// Add's a runmd listener to a dispatcher,
    ///
    /// The name of the system will be "runmd_listener"
    ///
    pub fn add_runmd_listener<'a, 'b, P>(
        thunk_context: ThunkContext,
        dispatcher_builder: &mut DispatcherBuilder<'a, 'b>,
    ) where
        P: Project + From<ThunkContext> + Send + 'a,
    {
        dispatcher_builder.add(
            RunmdListener::<P>::from(thunk_context),
            "runmd_listener",
            &["event_runtime"],
        );
    }

    /// Add's an operation listener to a dispatcher,
    ///
    /// The name of the system will be "operation_listener"
    ///
    pub fn add_operation_listener<'a, 'b, P>(
        thunk_context: ThunkContext,
        dispatcher_builder: &mut DispatcherBuilder<'a, 'b>,
    ) where
        P: Project + From<ThunkContext> + Send + 'a,
    {
        dispatcher_builder.add(
            OperationListener::<P>::from(thunk_context),
            "operation_listener",
            &["event_runtime"],
        );
    }

    /// Add's an error context listener to a dispatcher
    ///
    /// The name of the system will be "error_context_listener"
    ///
    pub fn add_error_context_listener<'a, 'b, P>(
        thunk_context: ThunkContext,
        dispatcher_builder: &mut DispatcherBuilder<'a, 'b>,
    ) where
        P: Project + From<ThunkContext> + Send + 'a,
    {
        dispatcher_builder.add(
            ErrorContextListener::<P>::from(thunk_context),
            "error_context_listener",
            &["event_runtime"],
        );
    }

    /// Add's an error context listener to a dispatcher,
    ///
    /// The name of the system will be "completed_plugin_listener"
    ///
    pub fn add_completed_plugin_listener<'a, 'b, P>(
        thunk_context: ThunkContext,
        dispatcher_builder: &mut DispatcherBuilder<'a, 'b>,
    ) where
        P: Project + From<ThunkContext> + Send + 'a,
    {
        dispatcher_builder.add(
            CompletedPluginListener::<P>::from(thunk_context),
            "completed_plugin_listener",
            &["event_runtime"],
        );
    }

    /// Add's a status update listener to a dispatcher,
    ///
    /// The name of the system will be "status_update_listener"
    ///
    pub fn add_status_update_listener<'a, 'b, P>(
        thunk_context: ThunkContext,
        dispatcher_builder: &mut DispatcherBuilder<'a, 'b>,
    ) where
        P: Project + From<ThunkContext> + Send + 'a,
    {
        dispatcher_builder.add(
            CompletedPluginListener::<P>::from(thunk_context),
            "status_update_listener",
            &["event_runtime"],
        );
    }

    /// Add's a status update listener to a dispatcher,
    ///
    /// The name of the system will be "status_update_listener"
    ///
    pub fn add_start_command_listener<'a, 'b, P>(
        thunk_context: ThunkContext,
        dispatcher_builder: &mut DispatcherBuilder<'a, 'b>,
    ) where
        P: Project + From<ThunkContext> + Send + 'a,
    {
        dispatcher_builder.add(
            StartCommandListener::<P>::from(thunk_context),
            "start_command_listener",
            &["event_runtime"],
        );
    }

    /// Get a reference to the world,
    ///
    pub fn world_ref(&self) -> Arc<&World> {
        Arc::new(self.world.as_ref().expect("should exist"))
    }

    /// Returns a immutable reference to the world,
    ///
    pub fn world(&self) -> &World {
        self.world.as_ref().expect("World should exist")
    }

    /// Returns a mutable reference to the world,
    ///
    pub fn world_mut(&mut self) -> &mut World {
        self.world.as_mut().expect("World should exist")
    }

    /// Opens the .runmd file in the current directory,
    ///
    pub async fn runmd<P>() -> Result<Self, impl Error>
    where
        P: Project,
    {
        Self::open::<P>(".runmd").await
    }

    /// Opens a file, compiles, and returns a host,
    ///
    pub async fn open<P>(path: impl Into<PathBuf>) -> Result<Self, impl Error>
    where
        P: Project,
    {
        let path = path.into();
        match tokio::fs::read_to_string(&path).await {
            Ok(runmd) => Ok(Host::load_content::<P>(runmd)),
            Err(err) => {
                event!(Level::ERROR, "Could not open file {:?}, {err}", path);
                Err(err)
            }
        }
    }

    /// Opens a uri via GET, compiles the body, and returns a host,
    ///
    pub async fn get<P>(uri: impl AsRef<str>) -> Result<Self, impl Error>
    where
        P: Project,
    {
        let https = HttpsConnector::new();
        let client = Client::builder().build::<_, hyper::Body>(https);

        match uri.as_ref().parse::<Uri>() {
            Ok(uri) => match client.get(uri).await {
                Ok(mut response) => match hyper::body::to_bytes(response.body_mut()).await {
                    Ok(bytes) => {
                        let bytes = bytes.to_vec();
                        let content =
                            from_utf8(&bytes).expect("should be able to read into a string");
                        Ok(Self::load_content::<P>(&content))
                    }
                    Err(err) => {
                        panic!("Could not read bytes {err}")
                    }
                },
                Err(err) => {
                    event!(Level::ERROR, "Could not get content {err}");
                    Err(err)
                }
            },
            Err(err) => {
                panic!("Could not parse uri, {}, {err}", uri.as_ref());
            }
        }
    }

    /// Compiles runmd content into a Host,
    ///
    pub fn load_content<P>(content: impl AsRef<str>) -> Self
    where
        P: Project,
    {
        let mut host = Self {
            runmd_path: None,
            url: None,
            command: None,
            world: Some(P::compile(content)),
        };

        host.link_sequences();
        host
    }

    /// Returns true if the host should exit,
    ///
    pub fn should_exit(&self) -> bool {
        let entities = self.world().entities();
        let lifecycle_options = self.world().read_component::<LifecycleOptions>();
        let events = self.world().read_component::<Event>();
        if (&entities, &events, &lifecycle_options).join().all(
            |(entity, event, lifecycle_option)| match (event, lifecycle_option) {
                (Event(.., None), LifecycleOptions::Exit(None)) => {
                    event!(Level::TRACE, "{:?} has exited", entity);
                    true
                }
                _ => {
                    false
                }
            },
        ) {
            true
        } else {
            false
        }
    }

    /// Finds the starting entity from some expression,
    ///
    pub fn find_start(&self, expression: impl AsRef<str>) -> Option<Entity> {
        Engine::find_block(self.world(), expression.as_ref().trim()).and_then(|e| {
            self.world()
                .read_component::<Engine>()
                .get(e)
                .and_then(|e| e.start())
        })
    }

    /// Starts by finding the start event from an engine_name,
    ///
    pub fn start_with<P>(&mut self, engine_name: impl AsRef<str>)
    where
        P: Project,
    {
        let engine_name = engine_name.as_ref();

        if let Some(start) = self.find_start(engine_name) {
            // If the starting entity has a thunk context, this will be passed to the configure_dispatcher method
            // on the project. The project can use that context to initialize listeners
            //
            let tc = self
                .world()
                .read_component::<ThunkContext>()
                .get(start)
                .cloned();

            self.start::<P>(start.clone().id(), tc);
        } else {
            panic!("Did not start {engine_name}");
        }
    }

    /// Prepares the host to start by creating a new dispatcher,
    /// 
    pub fn prepare<'a, 'b, P>(&mut self, context: Option<ThunkContext>) -> Dispatcher<'a, 'b> 
    where
        P: Project,
    {
        let mut dispatcher = {
            let mut dispatcher = Host::dispatcher_builder();
            P::configure_dispatcher(&mut dispatcher, context);
            dispatcher.build()
        };
        dispatcher.setup(self.world_mut());
        dispatcher
    }

    /// Starts an event entity,
    ///
    pub fn start<P>(&mut self, event_entity: u32, thunk_context: Option<ThunkContext>)
    where
        P: Project,
    {
        let mut dispatcher = self.prepare::<P>(thunk_context);

        // Starts an event
        let event = self.world().entities().entity(event_entity);
        self.start_event(event, ThunkContext::default());

        // Waits for event runtime to exit
        self.wait_for_exit(&mut dispatcher);

        // Exits by shutting down the inner tokio runtime
        self.exit();
    }

    /// Waits for the host systems to exit,
    ///
    pub fn wait_for_exit<'a, 'b>(&mut self, dispatcher: &mut Dispatcher<'a, 'b>) {
        // Waits for the event runtime to complete
        while !self.should_exit() {
            dispatcher.dispatch(self.world());
            self.world_mut().maintain();
        }
    }

    /// Starts an event,
    ///
    pub fn start_event(&mut self, event: Entity, thunk_context: ThunkContext) {
        if let Some(event) = self.world().write_component::<Event>().get_mut(event) {
            event.fire(thunk_context);
        }
        self.world_mut().maintain();
    }

    /// Shuts down systems and cancels all thunks,
    ///
    pub fn exit(&mut self) {
        self.world_mut()
            .exec(|mut cancel_tokens: WriteStorage<CancelThunk>| {
                for token in cancel_tokens.drain().join() {
                    token.0.send(()).ok();
                }
            });

        self.world_mut()
            .remove::<tokio::runtime::Runtime>()
            .expect("should be able to remove")
            .shutdown_background();
    }
}

impl Debug for Host {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Host")
            .field("url", &self.url)
            .field("runmd_path", &self.runmd_path)
            .finish()
    }
}

impl Into<World> for Host {
    fn into(self) -> World {
        self.world.unwrap()
    }
}

impl AsRef<World> for Host {
    fn as_ref(&self) -> &World {
        self.world()
    }
}

impl AsMut<World> for Host {
    fn as_mut(&mut self) -> &mut World {
        self.world_mut()
    }
}

mod test {
    struct Test;

    impl crate::Project for Test {
        fn interpret(_world: &specs::World, _block: &reality::Block) {}
    }

    #[test]
    #[tracing_test::traced_test]
    fn test_host() {
        use crate::{Commands, Host};
        let mut host = Host::load_content::<Test>(
            r#"
        ``` repeat
        + .engine 
        : .event print_1
        : .event print_2
        : .repeat 5
        ```

        ``` print_1 repeat
        + .runtime
        : .println hello
        ```

        ``` print_2 repeat
        + .runtime
        : .println world
        ```
        "#,
        );

        host.set_command(Commands::start_engine("repeat"));
        host.handle_start::<Test>();
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_example() {
        use crate::{Commands, Host};
        let mut host = Host::open::<Test>("examples/hello_runmd/.runmd")
            .await
            .expect("should load");
        host.set_command(Commands::start_engine("test_block1"));
        host.handle_start::<Test>();

        // Make sure everything exited successfully
        assert!(logs_contain(
            "lifec::host: Entity(6, Generation(1)) has exited"
        ));
        assert!(logs_contain(
            "lifec::host: Entity(9, Generation(1)) has exited"
        ));
        assert!(logs_contain(
            "lifec::host: Entity(13, Generation(1)) has exited"
        ));
        assert!(logs_contain(
            "lifec::host: Entity(17, Generation(1)) has exited"
        ));
    }
}
