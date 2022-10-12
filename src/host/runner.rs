use atlier::system::Value;
use clap::Args;
use reality::{Interpreter, SpecialAttribute};
use specs::WorldExt;

use crate::{Host, Operation, Runtime};

/// Wrapper struct over a host that can be scheduled to run operations,
///
/// Unlike the engine/event runtime which can be extended to orchestrate complex
/// sequences, the runner has a more narrow scope. It's designed to wait/listen 
/// for operations to execute from a remote controller.
/// 
#[derive(Default, Args)]
pub struct Runner {
    /// The name of the runner's control block,
    /// 
    #[clap(long, default_value_t = String::from("runner"))]
    control_symbol: String,
    /// The name of the runner's receive event name,
    /// 
    #[clap(long, default_value_t = String::from("receive"))]
    receive_name: String,
    /// The name of the runner's execute event name, 
    /// 
    #[clap(long, default_value_t = String::from("execute"))]
    execute_name: String, 
    /// Source host for the runner
    /// 
    #[clap(skip)]
    host: Option<Host>,
}

impl Interpreter for Runner {
    fn initialize(&self, _world: &mut specs::World) {
        // todo
    }

    fn interpret(&self, world: &specs::World, block: &reality::Block) {
        // todo
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
    let runmd = r#"
    ``` execute runner

    + .operation
    : .println 
    : .println


    ```
    "#;
}
