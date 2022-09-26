// use std::env::consts::OS;

// use atlier::system::Value;
// use tracing::{event, Level};
// use which::which;

// use crate::{plugins::{Plugin, ThunkContext}, AttributeIndex};

// #[derive(Default)]
// pub struct Expect;

// impl Expect {
//     pub fn should_expect(name: impl AsRef<str>, symbol: impl AsRef<str>) -> bool {
//         let symbol = symbol.as_ref();
//         if let Some((_, os)) = name.as_ref().trim_end_matches(&format!("::{symbol}")).split_once("::") {
//             return os == OS;
//         }

//         true
//     }
// }

// impl Plugin for Expect {
//     fn symbol() -> &'static str {
//         "expect"
//     }

//     fn description() -> &'static str {
//         "Check expectations for the current environment."
//     }

//     fn call(context: &ThunkContext) -> Option<crate::plugins::AsyncContext> {
//         context.clone().task(|_| {
//             let mut tc = context.clone();
//             async move {
//                 let mut project = tc.clone().project.unwrap_or_default();

//                 // Uses `which` crate to check path for binaries
//                 for (name, check) in tc.as_ref().find_symbol_values("which") {
//                     if !Self::should_expect(&name, "which") {
//                         eprintln!("skipping {name}");
//                         continue;
//                     }

//                     if let Value::TextBuffer(command) = check {
//                         tc.update_status_only(format!("checking {command}")).await;
//                         match which(&command) {
//                             Ok(path) => {
//                                 let path = format!("{:?}", path).trim_matches('"').to_string();
//                                 tc.update_status_only(format!("found {command} at {path}")).await;

//                                 // Output results to path symbol for current block
//                                 project = project.with_block("env", "path", |g| {
//                                     g.add_text_attr(&command, path);
//                                 });
//                             }
//                             Err(err) => {
//                                 let log = format!("`expect` plugin error on symbol `which` for `{command}`: {err}");
//                                 event!(Level::ERROR, "{log}");
//                                 tc.update_status_only(format!("{log}")).await;
//                                 tc.error(|g| {
//                                     g.add_text_attr(&command, "missing");
//                                 });
//                             }
//                         }
//                     }
//                 }

//                 // dispatch result
//                 tc.project = Some(project);
//                 Some(tc)
//             }
//         })
//     }
// }
