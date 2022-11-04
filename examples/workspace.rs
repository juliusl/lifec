use lifec::prelude::*;
use tracing_subscriber::EnvFilter;

/// Example showing opening an editor for a workspace,
///
fn main() {
    tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive("lifec=debug".parse().expect("should parse"))
                .from_env()
                .expect("should work"),
        )
        .compact()
        .init();
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

    + test .config execute.test-2
    : opname .symbol print-2
    
    + .operation print
    : .println Hello Print Operation a
    : .chaos

    + test .operation print
    : .println Hello Print Operation a 2
    : .chaos

    + .operation print-2
    : .println Hello Print Operation c 3
    : .chaos
    : .test_host

    + .operation start_test_host
    : .test_host
    : .test_host_sender

    + test .operation print-2
    : .println Hello Print Operation c 4
    : .chaos

    + .operation read-performance
    : .monitor test_host
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
        : .chaos
        ```

        ``` receive
        + .runtime
        : .println hello receive a {name}, {description}
        : .fmt name, description
        : .chaos
        ```

        ``` cancel
        + .runtime
        : .println hello cancel a
        : .chaos
        ```

        ``` execute
        + .runtime
        : .println hello execute a
        : .chaos
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
        : .next     test
        ```

        ``` setup
        + .runtime
        : .println hello setup 2
        : .chaos
        ```

        ``` receive
        + .runtime
        : .println hello receive 2
        : .chaos
        ```

        ``` cancel
        + .runtime
        : .println hello cancel 2
        : .chaos
        ```

        ``` execute
        : opname .symbol print

        + .runtime
        : .println hello execute 2
        : .run print-2
        : .chaos
        : .run {opname}
        : .chaos
        ```
        "#,
    );

    let files = vec![test_engine, test_engine2];

    // Manually compile workspace since we don't need settings from the CLI --
    let world = Test::compile_workspace(&workspace, files.iter(), None);

    let mut host = Host::from(world);
    host.enable_listener::<Test>();
    host.link_sequences();
    host.open_runtime_editor::<Test>()
}

#[derive(Default)]
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
        println!("Completed event - {}", e.id());
    }

    fn on_operation(&mut self, _: Operation) {}
    fn on_error_context(&mut self, _: &ErrorContext) {}
}
