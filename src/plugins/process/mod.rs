use std::env::consts::OS;

use super::{thunks::CancelToken, Plugin, ThunkContext};
use atlier::system::Value;
use chrono::{Local, Utc};
use specs::{Component, HashMapStorage};
use tokio::{select, task::JoinHandle, process::Command};

mod remote;
pub use remote::Remote;

mod expect;
pub use expect::Expect;

mod missing;
pub use missing::Missing;

/// The process component executes a command and records the output
#[derive(Debug, Clone, Default, Component)]
#[storage(HashMapStorage)]
pub struct Process;

impl Process {
    fn resolve_command(tc: &ThunkContext) -> Option<String> {
        let os = OS;
        let os_command = format!("command_{os}");

        if let Some(command) = tc.as_ref().find_text(os_command) {
            Some(command) 
        } else {
            tc.as_ref().find_text("command")
        }
    }

    async fn resolve_args(tc: &ThunkContext, command: &mut Command) {
        for (arg, value) in tc.clone().as_ref().find_symbol_values("arg") {
            let arg = arg.trim_end_matches("::arg");
            match value {
                Value::Bool(true) => {
                    tc.update_status_only(format!("define {arg} arg .enable")).await;
                    if arg.len() == 1 {
                        let arg = format!("-{arg}");
                        command.arg(arg);
                    } else {
                        let arg = format!("--{arg}");
                        command.arg(arg);
                    }
                },
                Value::Symbol(reference) => {
                    if let Some(value) = tc.as_ref().find_text(&reference) {
                        tc.update_status_only(format!("define {arg} arg .symbol {reference}")).await;
                        if arg.len() == 1 {
                            let arg = format!("-{arg}");
                            command.arg(arg).arg(value);
                        } else {
                            let arg = format!("--{arg}");
                            command.arg(arg).arg(value);
                        }
                    }
                },
                _ => continue 
            }
        }
    }

    async fn resolve_env(tc: &ThunkContext, command: &mut Command) {
        for (env, value) in tc.as_ref().find_symbol_values("env") {
            let env = env.trim_end_matches("::env");
            match value {
                Value::Symbol(reference) => {
                    tc.update_status_only(format!("define {env} env .symbol {reference}")).await;
                    if let Some(value) = tc.as_ref().find_text(reference) {
                        command.env(env, value);
                    }
                },
                _ => {
                    continue;
                }
            }
        }
    }
}


impl Plugin<ThunkContext> for Process {
    fn symbol() -> &'static str {
        "process"
    }

    fn description() -> &'static str {
        "Executes a new command w/ an OS process."
    }

    fn call_with_context(
        context: &mut super::ThunkContext,
    ) -> Option<(JoinHandle<ThunkContext>, CancelToken)> {
        context.clone().task(|cancel_source| {
            let mut tc = context.clone();
            async move {
                let command = Self::resolve_command(&tc).unwrap_or("echo missing command".to_string());
                // Creating a new tokio command
                let parts: Vec<&str> = command.split(" ").collect();
                tc.update_progress(format!("``` {} process", tc.block.block_name), 0.10)
                    .await;
                if let Some(program) = parts.get(0) {
                    tc.update_progress(format!("add command .text {}", program), 0.10)
                        .await;
                    let mut command_task = tokio::process::Command::new(&program);
                    for (el, arg) in parts.iter().skip(1).enumerate() {
                        command_task.arg(arg);
                        tc.update_progress(format!("define arg{}    .text {}", el, arg), 0.10)
                            .await;
                    }

                    Self::resolve_args(&tc, &mut command_task).await;
                    Self::resolve_env(&tc, &mut command_task).await;

                    if let Some(current_dir) = tc.as_ref().find_text("current_dir") {
                        tc.update_progress(format!("add current_dir .text {current_dir}"), 0.20).await;
                        command_task.current_dir(current_dir);
                    }

                    tc.update_progress("```", 0.20).await;
                    tc.update_progress("# Running", 0.20).await;
                    let start_time = Some(Utc::now());

                    command_task.kill_on_drop(true);

                    select! {
                       output = command_task.output() => {
                            match output {
                                Ok(output) => {
                                    // Completed process, publish result
                                    tc.update_progress("# Finished, recording output", 0.30).await;
                                    let timestamp_utc = Some(Utc::now().to_string());
                                    let timestamp_local = Some(Local::now().to_string());
                                    let elapsed = start_time
                                        .and_then(|s| Some(Utc::now() - s))
                                        .and_then(|d| Some(format!("{} ms", d.num_milliseconds())));

                                    if let Some(project) = tc.project.as_mut() {
                                        *project = project.with_block(program, "process", |c| {
                                            c.with_int("code", output.status.code().unwrap_or_default())
                                                .with_text("command", &command)
                                                .with_binary("stdout", output.stdout.to_vec())
                                                .with_binary("stderr", output.stderr)
                                                .with_text("timestamp_local", timestamp_local.unwrap_or_default())
                                                .with_text("timestamp_utc", timestamp_utc.unwrap_or_default())
                                            .add_text_attr("elapsed", elapsed.unwrap_or_default());
                                        });
                                    }
                                }
                                Err(err) => {
                                    tc.update_progress(format!("# error {}", err), 0.0).await;
                                }
                            }
                       }
                       _ = cancel_source => {
                            tc.update_progress(format!("# cancelling"), 0.0).await;
                       }
                    }
                }

                Some(tc)
            }
        })
    }
}
