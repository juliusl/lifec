use crate::prelude::*;
use std::path::PathBuf;

mod config;
pub use config::Config as WorkspaceConfig;

mod operation;
pub use operation::Operation as WorkspaceOperation;

/// Struct for managing a complex runmd project,
///
/// For small projects consisting of a single runmd file, the default Host is good enough.
/// For a more complex project with several parts, it is nice to have a few more utilities
/// to help manage the chaos. In addition, the utilities provided by this struct should also
/// be usable by simple files as well.
///
/// # Utilities
/// - Organizing the work directory for this workspace,
///     * The root is always .world/
///     * A workspace has one host, .world/{host}
///     * Tenant can be added to the host, .world/{host}/{tenant}
///     * A tenant can have several paths, .world/{host}/{tenant}/{path}
/// - Authenticating entities
///     * By default when setting up a workspace host, a keypair is used to sign subsequent tenant dirs
///     * Each tenant dir will have a keypair to sign path dirs
///     * Each path dir will have it's own key to validate information it provides
///     * When a plugin runs, it will need to
/// - Orchestrating multiple hosts,
///     * Within a host directory, each tenant will have a seperate Host instance
///     * Within the context of the workspace, each dir with a .runmd file is considered a host
///     * A Host is the owner of the path hosts within it's directory
///     * To tell that a path was added by a tenant, the tenant will sign a file that authenticates each path host
///
#[derive(Clone, Debug, Component)]
#[storage(VecStorage)]
pub struct Workspace {
    /// Work directory for this workspace context,
    work_dir: PathBuf,
    /// Root directory
    root: Option<PathBuf>,
    /// Content of the root runmd file of the workspace,
    root_runmd: Option<String>,
    /// Use this list of tags when generating an engine
    use_tags: Option<Vec<String>>,
    /// Name of the host,
    host: String,
    /// Name of the tenant,
    tenant: Option<String>,
    /// Name of the path,
    path: Option<String>,
}

impl Workspace {
    /// Returns a new workspace for host,
    ///
    pub fn new(host: impl AsRef<str>, root: Option<PathBuf>) -> Self {
        let work_dir = root
            .clone()
            .unwrap_or(PathBuf::from(""))
            .join(".world")
            .join(host.as_ref());

        Self {
            work_dir: work_dir.to_path_buf(),
            root,
            root_runmd: None,
            use_tags: None,
            host: host.as_ref().to_string(),
            tenant: None,
            path: None,
        }
    }

    /// Sets the root runmd content for this workspace,
    ///
    pub fn set_root_runmd(&mut self, runmd: impl AsRef<str>) {
        self.root_runmd = Some(runmd.as_ref().to_string());
    }

    /// Returns the root runmd to use for this workspace,
    ///
    pub fn root_runmd(&self) -> Option<String> {
        self.root_runmd.clone()
    }

    /// Returns a clone with tags,
    ///
    pub fn use_tags(&self, tags: Vec<impl AsRef<str>>) -> Self {
        let mut clone = self.clone();
        clone.use_tags = Some(tags.iter().map(|t| t.as_ref().to_string()).collect());
        clone
    }

    /// Returns the identity uri for the current workspace context for a block,
    ///
    pub fn identity_uri(&self, block: &Block) -> Option<String> {
        match (self.host.as_str(), self.tenant.as_ref(), self.path.as_ref()) {
            (host, Some(tenant), None) if !block.name().is_empty() => Some(format!(
                "{}.{}.{tenant}.{host}",
                block.name(),
                block.symbol()
            )),
            (host, Some(tenant), None) if block.name().is_empty() => {
                Some(format!("{}.{tenant}.{host}", block.symbol()))
            }
            (host, Some(tenant), Some(path)) if !block.name().is_empty() => Some(format!(
                "{}.{}.{tenant}.{host}/{path}",
                block.name(),
                block.symbol()
            )),

            (host, Some(tenant), Some(path)) if block.name().is_empty() => {
                Some(format!("{}.{tenant}.{host}/{path}", block.symbol()))
            }
            _ => None,
        }
    }

    /// Get a tenant from the workspace,
    ///
    pub fn tenant(&self, tenant: impl AsRef<str>) -> Self {
        let work_dir = self
            .root
            .clone()
            .unwrap_or(PathBuf::from(""))
            .join(".world")
            .join(self.host.as_str())
            .join(tenant.as_ref());

        Self {
            work_dir,
            root: self.root.clone(),
            root_runmd: None,
            use_tags: None,
            host: self.host.to_string(),
            tenant: Some(tenant.as_ref().to_string()),
            path: None,
        }
    }

    /// Get a path from the workspace,
    ///
    pub fn path(&self, path: impl AsRef<str>) -> Option<Self> {
        if let Some(tenant) = self.tenant.as_ref() {
            let work_dir = self
                .root
                .clone()
                .unwrap_or(PathBuf::from(""))
                .join(".world")
                .join(self.host.as_str())
                .join(tenant.as_str())
                .join(path.as_ref());

            Some(Self {
                work_dir,
                root: self.root.clone(),
                root_runmd: None,
                use_tags: None,
                host: self.host.to_string(),
                tenant: Some(tenant.to_string()),
                path: Some(path.as_ref().to_string()),
            })
        } else {
            event!(Level::ERROR, "Trying to create a path without a tenant");
            None
        }
    }

    /// Returns an iterator over tags,
    ///
    pub fn iter_tags(&self) -> impl Iterator<Item = &String> {
        if let Some(tags) = self.use_tags.as_ref() {
            tags.iter()
        } else {
            [].iter()
        }
    }

    /// Returns a path buf to the work dir,
    ///
    pub fn work_dir(&self) -> &PathBuf {
        &self.work_dir
    }
}

#[test]
fn test_workspace_paths() {
    use reality::Parser;

    let mut parser = Parser::new().parse(
        r#"
    ``` workspace
    ```

    ``` try workspace
    ```
    "#,
    );

    let workspace = Workspace::new("lifec.io", None);

    assert_eq!(&PathBuf::from(".world/lifec.io"), workspace.work_dir());

    let tenant = workspace.tenant("test");
    assert_eq!(&PathBuf::from(".world/lifec.io/test"), tenant.work_dir());
    assert_eq!(
        Some("workspace.test.lifec.io".to_string()),
        tenant.identity_uri(parser.get_block("", "workspace"))
    );

    let path = tenant
        .path("tester")
        .expect("should be able to create a path");
    assert_eq!(
        &PathBuf::from(".world/lifec.io/test/tester"),
        path.work_dir()
    );
    assert_eq!(
        Some("try.workspace.test.lifec.io/tester".to_string()),
        path.identity_uri(parser.get_block("try", "workspace"))
    );
}

mod tests {
    use crate::{prelude::*, project::Listener};
    struct Test;

    impl Project for Test {
        fn interpret(_: &specs::World, _: &reality::Block) {
            // no-op
        }
    }

    impl Listener for Test {
        fn create(_: &World) -> Self {
            Test {}
        }

        fn on_status_update(&mut self, status_update: &StatusUpdate) {
            event!(Level::TRACE, "Received status_update {:?}", status_update);
        }

        fn on_completed_event(&mut self, e: &Entity) {
            event!(Level::TRACE, "Completed event - {}", e.id());
        }

        fn on_runmd(&mut self, _: &RunmdFile) {}
        fn on_operation(&mut self, _: &Operation) {}
        fn on_error_context(&mut self, _: &ErrorContext) {}
        fn on_start_command(&mut self, _: &Start) {}
    }

    #[test]
    #[tracing_test::traced_test]
    fn test_compile_workspace() {
        use atlier::system::{Attribute, Value};
        use reality::Block;
        use reality::BlockProperty;
        use specs::WorldExt;
        use tracing::Level;

        let mut workspace = Workspace::new("test.io", None);
        workspace.set_root_runmd(
            r#"
        ```
        # Test that the default name is Test
        + .config receive.test
        : name .symbol Test

        # Test that the tagged version is Test2
        + test .config receive.test
        : name .symbol Test2
        
        + .operation print
        : .println Hello Print Operation a
        : .println Hello Print Operation b

        + test .operation print
        : .println Hello Print Operation a 2
        : .println Hello Print Operation b 2
        ```
        "#,
        );

        let test_engine = RunmdFile::new_src(
            "test",
            r#"
            ```
            + .engine
            : .once     setup
            : .start    receive, cancel
            : .select   execute
            : .next     test-2
            ```

            ``` setup
            + .runtime
            : .println hello setup a
            : .println hello setup b
            : .println hello setup c
            ```

            ``` receive
            + .runtime
            : .println hello receive a {name}
            : .fmt name
            : .println hello receive b
            : .println hello receive c
            ```

            ``` cancel
            + .runtime
            : .println hello cancel a
            : .println hello cancel b
            : .println hello cancel c
            ```

            ``` execute
            + .runtime
            : .println hello execute a
            : .println hello execute b
            : .println hello execute c
            ```
            "#,
        );

        let test_engine2 = RunmdFile::new_src(
            "test-2",
            r#"
            ```
            + .engine
            : .once     setup
            : .start    receive, cancel
            : .select   execute
            : .repeat   1
            ```

            ``` setup
            + .runtime
            : .println hello setup 2
            ```

            ``` receive
            + .runtime
            : .println hello receive 2
            ```

            ``` cancel
            + .runtime
            : .println hello cancel 2
            ```

            ``` execute
            + .runtime
            : .println hello execute 2
            ```
            "#,
        );

        let files = vec![test_engine, test_engine2];

        // Test with no tags
        let world = Test::compile_workspace(&workspace, files.iter());
        {
            let root_ent = world.entities().entity(0);
            let root = world.read_component::<Block>();
            let root = root.get(root_ent).expect("should have a root block");

            let indexes = root.index();

            let default = indexes.get(0).expect("should have index");
            assert_eq!(
                default.root(),
                &Attribute::new(0, "config", Value::Symbol("receive.test".to_string()))
            );
            assert_eq!(
                default.find_property("name"),
                Some(BlockProperty::Single(Value::Symbol("Test".to_string())))
            );

            let default = indexes.get(1).expect("should have index");
            assert_eq!(
                default.root(),
                &Attribute::new(0, "test.config", Value::Symbol("receive.test".to_string()))
            );
            assert_eq!(
                default.find_property("name"),
                Some(BlockProperty::Single(Value::Symbol("Test2".to_string())))
            );

            let default = indexes.get(2).expect("should have index");
            assert_eq!(
                default.root(),
                &Attribute::new(29, "operation", Value::Empty)
            );
            assert_eq!(
                default.find_property("name"),
                Some(BlockProperty::Single(Value::Symbol("print".to_string())))
            );
        }

        let mut host = Host::from(world);
        host.enable_listener::<Test>();
        host.link_sequences();

        let mut dispatcher = host.prepare::<Test>();
        {
            let mut events = host.world().system_data::<Events>();
            // Test that initially everything is idle
            assert!(events.scan().is_empty());

            // Test that activating an event gets picked up by .scan()
            let event = host.world().entities().entity(2);
            events.activate(event);

            // TODO - add assertions
            let event_state = events.scan();
            assert_eq!(event_state.get(0), Some(&EventStatus::New(event)));
            events.handle(event_state);

            for i in 0..9 {
                tracing::event!(Level::DEBUG, "Tick {i}");
                events.serialized_tick();
            }
        }

        // Test project listener
        {
            let broker = host.world().system_data::<PluginBroker>();

            broker
                .try_send_status_update((
                    host.world().entities().create(),
                    0.0,
                    String::from("test"),
                ))
                .ok();
        }

        dispatcher.dispatch(host.world());
        dispatcher.dispatch(host.world());

        // Test with tags
        let world = Test::compile_workspace(&workspace.use_tags(vec!["test"]), files.iter());
        let mut host = Host::from(world);
        host.enable_listener::<Test>();
        host.link_sequences();

        let mut dispatcher = host.prepare::<Test>();
        {
            let mut events = host.world().system_data::<Events>();
            // Test that initially everything is idle
            assert!(events.scan().is_empty());

            // Test that activating an event gets picked up by .scan()
            let event = host.world().entities().entity(2);
            events.activate(event);

            // TODO - add assertions
            let event_state = events.scan();
            assert_eq!(event_state.get(0), Some(&EventStatus::New(event)));
            events.handle(event_state);

            for i in 0..9 {
                tracing::event!(Level::DEBUG, "Tick {i}");
                events.serialized_tick();
            }
        }

        // Test project listener
        {
            let broker = host.world().system_data::<PluginBroker>();

            broker
                .try_send_status_update((
                    host.world().entities().create(),
                    0.0,
                    String::from("test"),
                ))
                .ok();
        }

        dispatcher.dispatch(host.world());
        dispatcher.dispatch(host.world());

        let mut operation_data = host.world().system_data::<WorkspaceOperation>();
        let operation = operation_data.execute_operation("print", None, None);
        operation.expect("should have an operation").wait();

        let operation = operation_data.execute_operation("print", Some("test".to_string()), None);
        operation.expect("should have an operation").wait();
    }
}
