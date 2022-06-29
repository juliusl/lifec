use super::{Plugin, ThunkContext};
use chrono::{Local, Utc};
use serde::{Deserialize, Serialize};
use specs::{Component, HashMapStorage};
use tokio::task::JoinHandle;

#[derive(Debug, Clone, Default, Component, Serialize, Deserialize)]
#[storage(HashMapStorage)]
pub struct Process;

impl Plugin<ThunkContext> for Process {
    fn symbol() -> &'static str {
        "process"
    }

    fn description() -> &'static str {
        "Executes a new command w/ an OS process."
    }

    fn call_with_context(context: &mut super::ThunkContext) -> Option<JoinHandle<ThunkContext>> {
        context.clone().task(|| {
            let mut tc = context.clone();
            async move {
                if let Some(command) = tc.as_ref().find_text("command") {
                    // Creating a new tokio command
                    let parts: Vec<&str> = command.split(" ").collect();
                    if let Some(command) = parts.get(0) {
                        tc.update_progress(format!("command: {}", command), 0.10)
                            .await;
                        let mut command_task = tokio::process::Command::new(&command);
                        for arg in parts.iter().skip(1) {
                            command_task.arg(arg);
                            tc.update_progress(format!("arg: {}", arg), 0.10)
                                .await;
                        }
                        tc.update_progress("running", 0.20).await;
                        let start_time = Some(Utc::now());
                        match command_task.output().await {
                            Ok(output) => {
                                // Completed process, publish result
                                tc.update_progress("Finished, recording output", 0.30).await;
                                let timestamp_utc = Some(Utc::now().to_string());
                                let timestamp_local = Some(Local::now().to_string());
                                let elapsed = start_time
                                    .and_then(|s| Some(Utc::now() - s))
                                    .and_then(|d| Some(format!("{} ms", d.num_milliseconds())));
                                tc.as_mut()
                                    .with_int("code", output.status.code().unwrap_or_default())
                                    .with_binary("stdout", output.stdout)
                                    .with_binary("stderr", output.stderr)
                                    .with_text(
                                        "timestamp_local",
                                        timestamp_local.unwrap_or_default(),
                                    )
                                    .with_text("timestamp_utc", timestamp_utc.unwrap_or_default())
                                    .with_text("elapsed", elapsed.unwrap_or_default());
                                tc.update_progress("completed", 1.0).await;
                            }
                            Err(err) => {
                                tc.update_progress(format!("error {}", err), 0.0).await;
                            }
                        }
                    }
                }
                Some(tc)
            }
        })
    }
}
