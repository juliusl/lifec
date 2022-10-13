use atlier::system::Value;
use clap::Args;
use reality::{Interpreter, SpecialAttribute};
use specs::WorldExt;

use crate::{Host, Operation, Runtime};

mod menu;
pub use menu::Menu;

mod order;
pub use order::Order;

/// Struct w/ settings for the host's runner platform,
///
/// The generic .runmd host event runtime is focused on defining engine sequences. This type of runtime is enhanced,
/// when accompanied with a "runner" system that can execute operations beneficial to the host runtime. This can range
/// from common tasks such as setup, diagnostics, development, etc. This module aims to provide a generic protocol abstraction
/// supported by .runmd, and enough plumbing that downstream implementations can customize some of the core abstracted methods. These methods
/// will be defined below in the protocol description.
/// 
/// # Runner Protocol Summary
/// 
/// 1) When the main runner host starts up, it will compile operations found in control block.
/// 2) When a runner is created, a .runmd file is created that defines how to send an operation to the runner.
/// 3) The controller will load this file, and execute the `send` event w/ a writable file that the runner can use to acknowledge the request.
/// 4) The runner will acknowledge by writing a .runmd file that defines events to interact with the operation, for example "monitor, cancel".
/// 5) While the runner is operating, the controller can load the file as a host to control the runner while it is active.
/// 
/// The specifics of where files are written and how files are read are up to the specific runner implementations. This module will include a template
/// w/ an end-to-end implementation w/ extention points to define the implementation details later.
/// 
/// # Template implementation - REPL
/// 
/// The template implementation will be a REPL runner that can operate locally, and will be available to all downstream hosts. Since a REPLoop, 
/// is essentially what the runner is doing it should be able to demonstrate the core abstractions nessccary for this type of architecture.
/// 
/// To fit the context more we will change the 'R', Read, to Receive and the 'P' to Complete. Therefore, we will implement a Receive-Execute-Compete loop.
/// 
/// This means that we will need 3 event blocks corresponding to each part of our loop. (These settings can be configured in the CLI)
/// 
/// ## Receive event
/// 1) First this event should write a .runmd file that acts as a menu for the operations available to execute. We can call this the menu.runmd.
/// 2) Next, it should listen for an input file (whose location it configured in the menu.runmd), for the operation to execute. This can be called the order.runmd.
/// 3) When the order.runmd is received, technically the receive event is finished, but further actions could be taken for any type of setup work needed.
/// 
/// ## Execute event
/// 1) Once an order.runmd is received, the execute event needs to generate an task.runmd and return it to the sender,
/// 2) Next, it needs to start the operation and wait for it to complete,
///
#[derive(Default, Args)]
pub struct Runner {
    /// The name of the runner's control block, 
    /// 
    /// This is the control block where all operations should be defined.
    ///
    #[clap(long, short, default_value_t = String::from("runner"))]
    control_symbol: String,
    /// The name of the runner's receive event name,
    ///
    #[clap(long, default_value_t = String::from("receive"))]
    receive_name: String,
    /// The name of the runner's execute event name,
    ///
    #[clap(long, default_value_t = String::from("execute"))]
    execute_name: String,
    /// The name of the runner's complete event name,
    ///
    #[clap(long, default_value_t = String::from("complete"))]
    complete_name: String,
    /// Source host for the runner,
    ///
    #[clap(skip)]
    host: Option<Host>,
}

impl Interpreter for Runner {
    fn initialize(&self, _world: &mut specs::World) {
        // 
    }

    fn interpret(&self, _world: &specs::World, block: &reality::Block) {
        // Find the control block and compile the operations
        if block.is_control_block() && block.symbol() == &self.control_symbol {
            for operation in block
                .index()
                .iter()
                .filter(|i| i.root().name() == "operation")
            {
            }
        }
 
        if !block.is_control_block() && !block.is_root_block() && block.name() == &self.receive_name {

        }

        if !block.is_control_block() && !block.is_root_block() && block.name() == &self.execute_name {
            
        }

        if !block.is_control_block() && !block.is_root_block() && block.name() == &self.complete_name {
            
        }
    }
}

/// This module implements a special attribute for Operation
///
impl SpecialAttribute for Operation {
    fn ident() -> &'static str {
        "operation"
    }

    fn parse(parser: &mut reality::AttributeParser, content: impl AsRef<str>) {
        if let Some(ident) = Self::parse_idents(&content).first() {
            Runtime::parse(parser, "");

            let operation_entity = parser
                .world()
                .expect("should have a world")
                .entities()
                .create();

            parser.define("operation", operation_entity.id() as usize);

            parser.define_child(operation_entity, "name", Value::Symbol(ident.to_string()));
        }
    }
}

#[test]
fn test_runner() {
    use std::path::PathBuf;
    use std::fs;
    
    let test_dir = PathBuf::from(".test").join("test-runner");

    fs::create_dir_all(test_dir).expect("can create test directtory");

    // Define test runmd files
    let runner = r#"
    # This is the main engine loop definition
    ``` runner
    + .engine
    : .event receive
    : .event execute
    : .event complete
    : .loop

    # Operations are defined in this block as well
    # These operations will be available while in the scope of the runner
    
    + .operation print-twice
    : .println hello
    : .println world

    + .operation print-once
    : .println goodbye
    ```
    
    ``` receive runner
    : menu_file .symbol menu.runmd 
    : menu_dir  .symbol .test

    + .runtime
    : .todo Publish the menu.runmd file
    : .todo Listen for a order.runmd file
    ```

    ``` execute runner
    + .runtime
    : .todo Publish a task.runmd file
    : .todo Execute the operation defined in order.runmd
    ```

    ``` complete runner
    + .runtime
    : .todo Complete the loop
    ```
    "#;

    let menu = r#"
    # This is received by the remote
    ``` send menu
    : order_file .symbol order.runmd
    : order_dir  .symbol .test/test-runner

    + .runtime
    : .todo Write to the order.runmd file
    ```
    "#;

    let order = r#"
    # This is received by the runner
    
    ``` accept order
    : task_file .symbol task.runmd
    : task_dir  .symbol  .test

    + .runtime
    : .todo Write a task.runmd file
    : .todo Return this file to the sender
    ``` 
    "#;


    let task = r#"
    ``` monitor task
    + .runtime
    : .todo Check to see if the operation has completed
    : .todo Print logs, various introspection
    ```

    ``` cancel task
    + .runtime
    : .todo Signal the runner to cancel the task
    ```
    "#;
}
