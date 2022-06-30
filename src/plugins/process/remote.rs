use crate::plugins::{Plugin, ThunkContext};
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
            let tc = context.clone();
            let child_handle = context.handle().clone();
            async move {
                // TODO: put actual command here
                let mut cmd = Command::new("zsh");
                cmd.stdout(Stdio::piped());

                if let Some(mut child) = cmd.spawn().ok() {
                    if let Some(stdout) = child.stdout.take() {
                        let mut reader = BufReader::new(stdout).lines();

                        if let Some(handle) = child_handle {
                            let _child_task = handle.spawn(async move {
                                tc.update_status_only("# child process started").await;
                                if let Some(output) = child.wait_with_output().await.ok() {
                                    eprintln!("exiting child process {:?}", output.status.code());
                                }
                                // TODO: record output
                                tc
                            });

                            log.update_status_only("# remote session started").await;
                            loop {
                                match reader.next_line().await {
                                    Ok(Some(line)) => {
                                        log.update_status_only(format!("{}", line)).await;
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
                log.update_status_only("Could not spawn child process").await;
                None
            }
        })
    }
}
