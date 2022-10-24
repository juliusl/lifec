use crate::prelude::*;

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
                if let Some(mut message) = tc.state().find_symbol("println") {
                    for fmt in tc.state().find_symbol_values("fmt") {
                        // Search the previous state
                        if let Some(value) = tc.search().find_symbol(&fmt) {
                            message = message.replace(&format!("{{{fmt}}}"), &format!("{}", value));
                        }
                    }

                    println!("{}", message);
                }
                None
            }
        })
    }

    fn compile(parser: &mut reality::AttributeParser) {
        parser.add_custom_with("fmt", |p, content| {
            let entity = p.last_child_entity().expect("should have a child entity");

            for ident in Self::parse_idents(content) { 
                p.define_child(entity, "fmt", Value::Symbol(ident));
            }
        });
    }
}

impl SpecialAttribute for Println {
    fn ident() -> &'static str {
        unimplemented!()
    }

    fn parse(_: &mut reality::AttributeParser, _: impl AsRef<str>) {
        unimplemented!()
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
