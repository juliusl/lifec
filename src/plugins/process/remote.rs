use crate::plugins::{Plugin, ThunkContext};
use chrono::{Local, Utc};
use specs::storage::DenseVecStorage;
use specs::Component;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[derive(Component, Default)]
#[storage(DenseVecStorage)]
pub struct Remote;

impl Plugin<ThunkContext> for Remote {
    fn symbol() -> &'static str {
        "remote"
    }

    fn call_with_context(
        context: &mut ThunkContext,
    ) -> Option<tokio::task::JoinHandle<ThunkContext>> {
        context.clone().task(|| {
            let log = context.clone();
            let mut tc = context.clone();
            let child_handle = context.handle().clone();
            async move {
                let cmd = tc.as_ref()
                    .find_text("command")
                    .unwrap_or("echo missing command".to_string());
                let parts: Vec<&str> = cmd.split(" ").collect();
                tc.update_progress(format!("``` {} process", tc.block.block_name), 0.10)
                            .await;
                if let Some(command) = parts.get(0) {
                    tc.update_progress(format!("add command .text {}", command), 0.10).await;
                    let mut command_task = Command::new(&command);
                    for (el, arg) in parts.iter().skip(1).enumerate() {
                            command_task.arg(arg);
                            tc.update_progress(format!("add arg{}    .text {}", el, arg), 0.10)
                                .await;
                    }
                    tc.update_progress("```", 0.10).await;
                    command_task.stdout(Stdio::piped());

                    if let Some(mut child) = command_task.spawn().ok() {
                        if let Some(stdout) = child.stdout.take() {
                            let mut reader = BufReader::new(stdout).lines();

                            if let Some(handle) = child_handle {
                                let _child_task = handle.spawn(async move {
                                tc.update_progress("# child process started, stdout/stdin are being piped to console", 0.50).await;
                                let start_time = Some(Utc::now());
                                match child.wait_with_output().await {
                                    Ok(output) => {
                                        // Completed process, publish result
                                        tc.update_progress("# child is exiting, recording output", 0.80).await;
                                        let timestamp_utc = Some(Utc::now().to_string());
                                        let timestamp_local = Some(Local::now().to_string());
                                        let elapsed = start_time
                                            .and_then(|s| Some(Utc::now() - s))
                                            .and_then(|d| Some(format!("{} ms", d.num_milliseconds())));
                                        tc.as_mut()
                                            .with_int("code", output.status.code().unwrap_or_default())
                                            .with_binary("stderr", output.stderr)
                                            .with_text(
                                                "timestamp_local",
                                                timestamp_local.unwrap_or_default(),
                                            )
                                            .with_text("timestamp_utc", timestamp_utc.unwrap_or_default())
                                            .with_text("elapsed", elapsed.unwrap_or_default());
                                        println!("exit code {}", output.status.code().unwrap_or_default());
                                    }
                                    Err(err) => {
                                        tc.update_progress(format!("# error {}", err), 0.0).await;
                                    }
                                }
                                tc
                            });

                            log.update_status_only("# remote session started").await;
                            loop {
                                match reader.next_line().await {
                                    Ok(Some(line)) => {
                                        eprintln!("{}", line);
                                        log.update_status_only(line).await;
                                    }
                                    Err(err) => {
                                        eprintln!("err: {}", err);
                                        break;
                                    }
                                    _ => {
                                        break;
                                    }
                                }
                            }
                                return _child_task.await.ok()
                            }
                        }
                    }
                }

                log.update_status_only("Could not spawn child process").await;
                None
            }
        })
    }
}
