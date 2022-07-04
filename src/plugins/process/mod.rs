use super::{thunks::CancelToken, Plugin, ThunkContext};
use atlier::system::Value;
use chrono::{Local, Utc};
use specs::{Component, HashMapStorage};
use tokio::{select, task::JoinHandle};

mod remote;
pub use remote::Remote;

/// The process component executes a command and records the output
#[derive(Debug, Clone, Default, Component)]
#[storage(HashMapStorage)]
pub struct Process;


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
                let command = tc
                    .as_ref()
                    .find_text("command")
                    .unwrap_or("echo missing command".to_string());
                // Creating a new tokio command
                let parts: Vec<&str> = command.split(" ").collect();
                tc.update_progress(format!("``` {} process", tc.block.block_name), 0.10)
                    .await;
                if let Some(command) = parts.get(0) {
                    tc.update_progress(format!("add command .text {}", command), 0.10)
                        .await;
                    let mut command_task = tokio::process::Command::new(&command);
                    for (el, arg) in parts.iter().skip(1).enumerate() {
                        command_task.arg(arg);
                        tc.update_progress(format!("define arg{}    .text {}", el, arg), 0.10)
                            .await;
                    }

                    for (_, arg) in tc.as_ref().find_symbol_values("arg") {
                        if let Value::TextBuffer(arg) = arg {
                            let parts: Vec<&str> = arg.split(" ").collect();

                            for (e, arg) in parts.iter().enumerate() {
                                command_task.arg(arg);
                                tc.update_progress(format!("add arg{0}{0}    .text {1}", e, arg), 0.20)
                                .await;
                            }
                        }
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
                                tc.as_mut()
                                    .with_int("code", output.status.code().unwrap_or_default())
                                    .with_binary("stdout", output.stdout)
                                    .with_binary("stderr", output.stderr)
                                    .with_text("timestamp_local", timestamp_local.unwrap_or_default())
                                    .with_text("timestamp_utc", timestamp_utc.unwrap_or_default())
                                    .add_text_attr("elapsed", elapsed.unwrap_or_default());
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
