use crate::{plugins::Plugin, AttributeIndex, BlockObject, BlockProperties, CustomAttribute};
use super::ThunkContext;

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

    fn call(context: &ThunkContext) -> Option<crate::plugins::AsyncContext> {
        context.clone().task(|_| {
            let tc = context.clone();
            async move {
                if let Some(message) = tc.state().find_symbol("println") {
                    println!("{}", message);
                }
                None
            }
        })
    }
}

impl BlockObject for Println {
    fn query(&self) -> BlockProperties {
        BlockProperties::default().require("println")
    }

    fn parser(&self) -> Option<CustomAttribute> {
        Some(Println::as_custom_attr())
    }
}
