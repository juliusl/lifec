use std::env::consts::OS;

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

                // Uses `which` crate to check path for binaries
                for (name, check) in tc.as_ref().find_symbol_values("which") {
                    if let Some((_, os)) = name.trim_end_matches("::which").split_once("::") {
                        if os != OS {
                            eprintln!("skipping {name}");
                            continue;
                        }
                    }

                    if let Value::TextBuffer(command) = check {
                        tc.update_status_only(format!("checking {command}")).await;
                        match which(&command) {
                            Ok(path) => {
                                let path = format!("{:?}", path).trim_matches('"').to_string();
                                tc.update_status_only(format!("found {command} at {path}")).await;

                                // Output results to path symbol for current block
                                project = project.with_block("env", "path", |g| {
                                    g.add_text_attr(&command, path);
                                });
                            }
                            Err(err) => {
                                let log = format!("`expect` plugin error on symbol `which` for `{command}`: {err}");
                                eprintln!("{log}");
                                tc.update_status_only(format!("{log}")).await;
                                tc.error(|g| {
                                    g.add_text_attr(&command, "missing");
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
