use lifec::prelude::*;

/// Example showing opening an editor for a workspace,
/// 
fn main() {
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
    : .println Hello Print Operation b

    + test .operation print
    : .println Hello Print Operation a 2
    : .println Hello Print Operation b 2

    + .operation print-2
    : .println Hello Print Operation c 3
    : .println Hello Print Operation c 3


    + test .operation print-2
    : .println Hello Print Operation c 4
    : .println Hello Print Operation c 4
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
        : opname .symbol print

        + .runtime
        : .println hello execute 2
        : .run print-2
        : .run {opname}
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

    fn on_runmd(&mut self, _: &RunmdFile) {}
    fn on_operation(&mut self, _: Operation) {}
    fn on_error_context(&mut self, _: &ErrorContext) {}
    fn on_start_command(&mut self, _: &Start) {}
}