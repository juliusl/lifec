use crate::{prelude::*, project::Listener};
use hyper::{Client, Uri};
use hyper_tls::HttpsConnector;
use specs::{Dispatcher, DispatcherBuilder, Entity, World, WorldExt};
use std::{error::Error, path::PathBuf, str::from_utf8, sync::Arc};

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

pub mod async_ext;

mod handler;
use handler::ListenerSetup;

mod runner;
pub use runner::Runner;

mod host_settings;
pub use host_settings::HostSettings;

/// Struct for initializing and hosting the runtime as well as parsing CLI arguments,
///
/// Used with a type that implements the Project trait.
///
#[derive(Default)]
pub struct Host {
    /// The compiled specs World,
    ///
    world: Option<World>,
    /// Workspace to use that provides environment related values, work_dir, uri, etc..
    ///
    workspace: Workspace,
    /// If set, host will use these settings when it starts up
    ///
    start: Option<Start>,
    /// Will setup a listener for the host
    ///
    listener_setup: Option<ListenerSetup>,
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

    /// Enables a project listener on the host when the dispatcher is prepared,
    ///
    pub fn enable_listener<L>(&mut self)
    where
        L: Listener,
    {
        self.listener_setup = Some(ListenerSetup::new::<L>());
    }

    /// Get an Arc reference to the world,
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

    /// Return the workspace for the host,
    ///
    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    /// Assumes the current directory is a workspace if a .runmd file is present,
    ///
    pub async fn runmd<P>() -> Result<Self, impl Error>
    where
        P: Project,
    {
        let path = PathBuf::from(".runmd")
            .canonicalize()
            .expect("should exist");

        let mut files = vec![];
        for e in path
            .parent()
            .expect("should have a parent dir")
            .read_dir()
            .expect("should be able to read dir")
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".runmd"))
            .filter(|e| e.file_name().to_string_lossy() != ".runmd")
            .filter(|e| e.file_type().and_then(|f| Ok(f.is_file())).ok() == Some(true))        
        {
            let file_name = e.file_name();
            event!(Level::DEBUG, "Found file {:?}", &file_name);

            if let Some(src) = tokio::fs::read_to_string(&file_name).await.ok() {
                let file = RunmdFile {
                    source: Some(src),
                    symbol: file_name.to_str().expect("should be a string").trim_end_matches(".runmd").to_string()
                };
                files.push(file);
                event!(Level::TRACE, "Added {:?}", &file_name);
            } else {
                event!(Level::WARN, "Could not read file {:?}, Skipping", &file_name);
            }
        }

        let mut host = Self {
            workspace: Workspace::default(),
            start: None,
            world: Some(P::compile_workspace(&Workspace::default(), files.iter(), None)),
            listener_setup: None,
        };

        host.link_sequences();
        Ok::<_, std::io::Error>(host)
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
        let mut workspace = Workspace::default();
        workspace.set_root_runmd(content.as_ref());

        let mut host = Self {
            workspace,
            start: None,
            world: Some(P::compile(content, None)),
            listener_setup: None,
        };

        host.link_sequences();
        host
    }

    /// Returns a host with a world compiled from a workspace,
    ///
    pub fn load_workspace<P>(
        root: Option<PathBuf>,
        host: impl AsRef<str>,
        tenant: impl AsRef<str>,
        path: Option<impl AsRef<str>>,
        tag: Option<impl AsRef<str>>,
    ) -> Self
    where
        P: Project,
    {
        let workspace = Workspace::new(host.as_ref(), root);
        let mut workspace = workspace.tenant(tenant.as_ref());

        if let Some(path) = path {
            if let Some(w) = workspace.path(path.as_ref()) {
                workspace = w;
            }
        }

        if let Some(tag) = tag {
            workspace = workspace.use_tag(tag);
        }

        let mut files = vec![];
        match std::fs::read_dir(workspace.work_dir()) {
            Ok(readdir) => {
                for entry in readdir.filter_map(|e| match e {
                    Ok(entry) => match entry.path().extension() {
                        Some(ext) if ext == "runmd" && !entry.file_name().is_empty() => Some(
                            entry
                                .file_name()
                                .to_str()
                                .expect("should be a string")
                                .trim_end_matches(".runmd")
                                .to_string(),
                        ),
                        _ => None,
                    },
                    Err(err) => {
                        event!(Level::ERROR, "Could not get entry {err}");
                        None
                    }
                }) {
                    files.push(RunmdFile {
                        symbol: entry,
                        source: None,
                    });
                }
            }
            Err(err) => {
                event!(Level::ERROR, "Error reading work directory {err}");
            }
        }

        let mut host = Self {
            workspace: workspace.clone(),
            start: None,
            world: Some(P::compile_workspace(&workspace, files.iter(), None)),
            listener_setup: None,
        };

        host.link_sequences();
        host
    }

    /// Returns true if the host should exit,
    ///
    pub fn should_exit(&self) -> bool {
        let events = self.world().system_data::<Events>();

        events.should_exit()
    }

    /// Finds the starting entity from some expression,
    ///
    pub fn find_start(&self, expression: impl AsRef<str>) -> Option<Entity> {
        Engine::find_block(self.world(), expression.as_ref().trim()).and_then(|e| {
            if let Some(engine) = self.world().read_component::<Engine>().get(e) {
                event!(Level::DEBUG, "Found {:#?}", engine);
                engine.start().cloned()
            } else {
                event!(Level::DEBUG, "Couldn't find engine for {}", e.id());
                None
            }
        })
    }

    /// Starts by finding the start event from an engine_name,
    ///
    pub fn start_with<P>(&mut self, engine_name: impl AsRef<str>)
    where
        P: Project,
    {
        let engine_name = engine_name.as_ref();

        self.start = Some(Start {
            id: None,
            engine_name: Some(engine_name.to_string()),
            operation: None,
            thunk_context: None,
        });
        self.start::<P>();
    }

    /// Returns self w/ a start set,
    ///
    pub fn with_start(mut self, start: &Start) -> Self {
        self.start = Some(start.clone());
        self
    }

    /// Creates a new dispatcher builder,
    ///
    pub fn new_dispatcher_builder<'a, 'b, P>(&mut self) -> DispatcherBuilder<'a, 'b>
    where
        P: Project,
    {
        let mut dispatcher = Host::dispatcher_builder();
        P::configure_dispatcher(self.world(), &mut dispatcher);

        if let Some(setup) = self.listener_setup.as_ref() {
            let ListenerSetup(enable) = setup;

            enable(self.world(), &mut dispatcher);
        }
        dispatcher
    }

    /// Prepares the host to start by creating a new dispatcher,
    ///
    pub fn prepare<'a, 'b, P>(&mut self) -> Dispatcher<'a, 'b>
    where
        P: Project,
    {
        let mut dispatcher = self.new_dispatcher_builder::<P>().build();
        dispatcher.setup(self.world_mut());
        dispatcher
    }

    /// Consumes the start directive and starts the host,
    ///
    pub fn start<P>(&mut self)
    where
        P: Project,
    {
        if match self.start.take() {
            Some(start) => match start {
                Start { id: Some(id), .. } => {
                    let id = self.world().entities().entity(id);
                    self.start_event(id);
                    true
                }
                Start {
                    engine_name: Some(engine_name),
                    ..
                } => {
                    if let Some(id) = self.find_start(engine_name) {
                        self.start_event(id);
                        true
                    } else {
                        false
                    }
                }
                _ => {
                    event!(Level::ERROR, "Invalid start settings, {:?}", start);
                    false
                }
            },
            None => false,
        } {
            let mut dispatcher = self.prepare::<P>();

            // Waits for event runtime to exit
            self.wait_for_exit(&mut dispatcher);

            // Exits by shutting down the inner tokio runtime
            self.exit();
        } else {
            panic!( "A start setting was not set for host")
        }
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
    pub fn start_event(&mut self, event: Entity) {
        if let Some(event) = self.world().write_component::<Event>().get_mut(event) {
            event.activate();
        }
    }

    /// Shuts down systems and cancels all thunks,
    ///
    pub fn exit(&mut self) {
        self.world_mut()
            .remove::<tokio::runtime::Runtime>()
            .expect("should be able to remove")
            .shutdown_background();
    }
}

impl Into<World> for Host {
    fn into(self) -> World {
        self.world.unwrap()
    }
}

impl From<World> for Host {
    fn from(world: World) -> Self {
        Host {
            workspace: Workspace::default(),
            start: None,
            world: Some(world),
            listener_setup: None,
        }
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
    #[derive(Default)]
    struct Test;

    impl crate::project::Project for Test {
        fn interpret(_world: &specs::World, _block: &reality::Block) {}
    }

    #[test]
    #[tracing_test::traced_test]
    fn test_host() {
        use crate::prelude::Host;
        let mut host = Host::load_content::<Test>(
            r#"
        ``` repeat
        + .engine 
        : .start print_1
        : .start print_2
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

        host.start_with::<Test>("repeat");
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_example() {
        use crate::prelude::Host;
        use hyper::http::Uri;

        let uri = Uri::from_static("test.example.com");

        eprintln!("{:?}", uri.host().unwrap().split_once("."));

        let mut host = Host::open::<Test>("examples/hello_runmd/.runmd")
            .await
            .expect("should load");
        host.start_with::<Test>("test_block1");
    }
}
