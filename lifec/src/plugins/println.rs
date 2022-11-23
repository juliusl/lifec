use crate::prelude::*;

use super::attributes::Fmt;

/// Prints a message to stdout
///
#[derive(Default)]
pub struct Println;

impl Plugin for Println {
    fn symbol() -> &'static str {
        "println"
    }

    fn description() -> &'static str {
        "Prints a message to stdout, ex. .println <message>"
    }

    fn call(context: &mut ThunkContext) -> Option<crate::plugins::AsyncContext> {
        context.clone().task(|_| {
            let tc = context.clone();
            async move {
                if let Some(message) = tc.state().find_symbol("println") {
                    let message = tc.format(message);

                    println!("{}", message);
                }
                None
            }
        })
    }

    fn compile(parser: &mut reality::AttributeParser) {
        parser.with_custom::<Fmt>();
    }
}

impl BlockObject for Println {
    fn query(&self) -> BlockProperties {
        BlockProperties::default()
            .require("println")
            .optional("fmt")
    }

    fn parser(&self) -> Option<CustomAttribute> {
        Some(Println::as_custom_attr())
    }
}
