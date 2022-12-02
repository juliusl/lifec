use std::io::BufRead;

use crate::prelude::*;

/// Plugin that reads a line from stdin,
///
#[derive(Default)]
pub struct Readln;

impl Plugin for Readln {
    fn symbol() -> &'static str {
        "readln"
    }

    fn description() -> &'static str {
        "Reads a line from stdin and stores it in state for the subsequent plugin."
    }

    fn call(context: &mut ThunkContext) -> Option<AsyncContext> {
        context.task(|_| {
            let mut tc = context.clone();
            async {
                for prompt_name in tc.search().find_symbol_values("prompt") {
                    if let Some(prompt) = tc.state().find_symbol(&prompt_name) {
                        // TODO print the prompt on stderr or stdout ?
                        eprint!("{prompt} ");
                    }

                    if let Some(Ok(line)) = std::io::stdin().lock().lines().next() {
                        if prompt_name == "readln-prompt" {
                            if let Some(prop_name) = tc.state().find_symbol("readln") {
                                tc.state_mut().replace_symbol(prop_name, line);
                            }
                        } else {
                            tc.state_mut().replace_symbol(prompt_name, line);
                        }
                    }
                }

                Some(tc)
            }
        })
    }

    fn compile(parser: &mut reality::AttributeParser) {
        if let Some(mut docs) = Self::start_docs(parser) {
            docs.as_mut().add_custom_with("prompt", |p, content| {
                let child_entity = p.last_child_entity().expect("should have a child entity");
                if let Some(prop_name) = p.symbol().cloned() {
                    /*
                    # Example
                    : name .prompt name >
                    : age  .prompt age  >
                    */
                    p.define_child(
                        child_entity,
                        "prompt",
                        Value::Symbol(String::from(&prop_name)),
                    );
                    p.define_child(child_entity, &prop_name, Value::Symbol(content));
                } else {
                    /*
                    # Example
                    : .readln name
                    : .prompt name>
                    */
                    p.define_child(
                        child_entity,
                        "prompt",
                        Value::Symbol(String::from("readln-prompt")),
                    );
                    p.define_child(child_entity, "readln-prompt", Value::Symbol(content));
                }
            })
            .add_doc(&mut docs, "Prints a prompt on stderr. If an identifier was passed to the .readln, then the value read here will be used to replace the property with that identifier.")
            .list()
            .name_optional()
            .symbol("The prompt to print, ex: name>");
        }
    }
}

impl BlockObject for Readln {
    fn query(&self) -> reality::BlockProperties {
        BlockProperties::default()
            .optional("readln")
            .optional("prompt")
    }

    fn parser(&self) -> Option<reality::CustomAttribute> {
        Some(Self::as_custom_attr())
    }
}
