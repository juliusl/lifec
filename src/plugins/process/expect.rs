use atlier::system::Value;
use which::which;

use crate::plugins::{Plugin, ThunkContext};

#[derive(Default)]
pub struct Expect;

impl Plugin<ThunkContext> for Expect {
    fn symbol() -> &'static str {
        "expect"
    }

    fn description() -> &'static str {
        "Check expectations for the current environment."
    }

    fn call_with_context(context: &mut ThunkContext) -> Option<crate::plugins::AsyncContext> {
        context.clone().task(|_| {
            let mut tc = context.clone();
            async move {
                let mut project = tc.clone().project.unwrap_or_default();

                // Uses `which` crate to check program installation
                for (_, check) in tc.as_ref().find_symbol_values("which") {
                    if let Value::TextBuffer(command) = check {
                        match which(&command) {
                            Ok(path) => {
                                let path = format!("{:?}", path).trim_matches('"').to_string();

                                // Output results to path symbol for current block
                                project = project.with_block("env", "path", |g| {
                                    g.add_text_attr(&command, path);
                                });
                            }
                            Err(err) => {
                                eprintln!("`expect` plugin error on symbol `which` for `{command}`: {err}");
                                
                                project = project.with_block("env", "missing", |g| {
                                    g.add_text_attr(&command, "");
                                });
                            }
                        }
                    }
                }

                // dispatch result
                tc.project = Some(project);
                Some(tc)
            }
        })
    }
}
