
// TODO: Incorporate this as a custom attribute added by .process 

// use crate::AttributeIndex;
// use crate::plugins::thunks::CancelToken;
// use crate::plugins::{Plugin, ThunkContext};
// use chrono::Utc;
// use specs::storage::DenseVecStorage;
// use specs::Component;
// use tokio::sync::oneshot;
// use tracing::{event, Level};
// use std::process::Stdio;
// use tokio::io::{AsyncBufReadExt, BufReader, AsyncWriteExt};
// use tokio::process::Command;
// use tokio::select;

// use super::Process;

// #[derive(Component, Default)]
// #[storage(DenseVecStorage)]
// pub struct Remote;

// impl Plugin for Remote {
//     fn symbol() -> &'static str {
//         "remote"
//     }

//     fn description() -> &'static str {
//         "Starts a process and pipes stdin and stdout to the current console. Useful for ssh, etc."
//     }

//     fn call(
//         context: &ThunkContext,
//     ) -> Option<(tokio::task::JoinHandle<ThunkContext>, CancelToken)> {
//         context.clone().task(|mut cancel_source| {
//             let mut tc = context.clone();
//             let log = context.clone();
//             let child_handle = context.handle().clone();
//             async move {
//                 let cmd = Process::resolve_command(&tc).unwrap_or("echo missing command".to_string());
//                 let parts: Vec<&str> = cmd.split(" ").collect();
//                 tc.update_progress(format!("``` {} process", tc.block.block_name), 0.10)
//                             .await;
//                 if let Some(command) = parts.get(0) {
//                     tc.update_progress(format!("add command .text {}", command), 0.10).await;
//                     let mut command_task = Command::new(&command);
//                     for (el, arg) in parts.iter().skip(1).enumerate() {
//                             command_task.arg(arg);
//                             tc.update_progress(format!("add arg{}    .text {}", el, arg), 0.10)
//                                 .await;
//                     }
//                     tc.update_progress("```", 0.10).await;
                    
//                     let enable_stdin_shell = tc.as_ref().is_enabled("enable_listener").unwrap_or_default();
//                     if enable_stdin_shell {
//                         // Normally stdin would be from the terminal window, 
//                         // enable this attribute to use the built in shell
//                         command_task.stdin(Stdio::piped());
//                     }

//                     command_task.stdout(Stdio::piped());
//                     command_task.stderr(Stdio::piped());

//                     if let Some(current_dir) = tc.as_ref().find_text("current_dir") {
//                         tc.update_progress(format!("add current_dir .text {current_dir}"), 0.20).await;
//                         command_task.current_dir(current_dir);
//                     }

//                     Process::resolve_args(&tc, &mut command_task).await;
//                     Process::resolve_env(&tc, &mut command_task).await;

//                     if let Some(mut child) = command_task.spawn().ok() {
//                         if let (Some(stdout), Some(stderr)) = (child.stdout.take(), child.stderr.take()) {
//                             let mut reader = BufReader::new(stdout).lines();
//                             let mut stderr_reader = BufReader::new(stderr).lines();
//                             let (child_cancel_tx, child_cancel_rx) = tokio::sync::oneshot::channel::<()>();

//                             let (stdin_task, mut input_cancel) = if enable_stdin_shell {
//                                 let mut stdin = child.stdin.take().expect("If stdin can't be taken, then the child would hang");
//                                 // Starts a listener and dispatches the address to the underlying runtime
//                                 // under the listener block at address ``` {block_name} listener
//                                 let mut reader = tc
//                                     .enable_listener(&mut cancel_source)
//                                     .await
//                                     .expect("expected a reader");
    
//                                 let (input_cancel, mut input_cancel_source) = oneshot::channel::<()>();
//                                 (Some(tc.handle().expect("needs to exist").spawn(async move {
//                                     loop {
//                                         match reader.next_line().await {
//                                             Ok(Some(line)) => {
//                                                 let line = format!("{line}\n").replace('\r', "");
//                                                 event!(Level::TRACE, "{line}");
//                                                 match stdin.write_all(line.as_bytes()).await {
//                                                     Ok(_) => {
//                                                         event!(Level::TRACE, "Wrote to child stdin OK");
//                                                     },
//                                                     Err(err) => {
//                                                         event!(Level::ERROR, "Could not write to child_task's stdin {err}");
//                                                         break;
//                                                     },
//                                                 }
//                                             },
//                                             Err(err) => {
//                                                 event!(Level::ERROR, "Could not read from listener {err}");
//                                                 break;
//                                             },
//                                             _ => {
//                                                 event!(Level::TRACE, "Didn't read anything");
//                                                 break;
//                                             }
//                                         }

//                                         if ThunkContext::is_cancelled(&mut input_cancel_source) {
//                                             break;
//                                         }

//                                         if let Some(err) = stdin.flush().await.err() {
//                                             event!(Level::ERROR, "error flushing stdin {err}");
//                                         }
//                                     }
//                                 })), Some(input_cancel))
//                             } else {
//                                 (None, None)
//                             };

//                             if let Some(handle) = child_handle {
//                                 let _child_task = handle.clone().spawn(async move {
//                                 tc.update_progress("# child process started, stdout/stdin are being piped to console", 0.50).await;
//                                 let start_time = Some(Utc::now());

//                                 select! {
//                                     output = child.wait_with_output() => {
//                                          match output {
//                                              Ok(output) => {
//                                                 // Completed process, publish result
//                                                 tc.update_progress("# Finished, recording output", 0.30).await;
//                                                 Process::resolve_output(&mut tc, cmd, start_time, output);
//                                              }
//                                              Err(err) => {
//                                                  tc.update_progress(format!("# error {}", err), 0.0).await;
//                                              }
//                                          }
//                                     }
//                                     _ = child_cancel_rx => {
//                                          tc.update_progress(format!("# child cancel received"), 0.0).await;
//                                     }
//                                  }

//                                 tc
//                             });

//                             let log = log.clone();
//                             let log_stderr = log.clone();

//                             // Reads child's stdout, so that stdin can continue to work
//                             let reader_task = log.handle().unwrap().spawn(async move {
//                                 event!(Level::DEBUG, "starting to listen to stdout");
//                                 while let Ok(line) = reader.next_line().await {
//                                     match line {
//                                         Some(line) => {
//                                             for byte in line.as_bytes() {
//                                                 log.send_char(*byte).await;
//                                             }
//                                             log.send_char(b'\r').await;
//                                             log.update_status_only(line).await;
//                                         },
//                                         None => {
//                                             break;
//                                         },
//                                     }
//                                 }
//                             });
//                             let stderr_reader_task = log_stderr.handle().unwrap().spawn(async move {
//                                 event!(Level::DEBUG, "starting to listen to stderr");
//                                 while let Ok(line) = stderr_reader.next_line().await {
//                                     match line {
//                                         Some(line) => {      
//                                             for byte in line.as_bytes() {
//                                                 log_stderr.send_char(*byte).await;
//                                             }
//                                             log_stderr.send_char(b'\r').await;
                                        
//                                             eprintln!("{}", line);
//                                             log_stderr.update_status_only(line).await;
//                                         },
//                                         None => {
//                                             event!(Level::WARN, "Didn't read anything from stderr");
//                                             break;
//                                         },
//                                     }
//                                 }
//                             });
                         

//                             // Wait for child to exit
//                             // OR, cancellation
//                             let output = select! {
//                                 tc = _child_task => {
//                                     eprintln!("child task completed");
//                                     return match tc {
//                                         Ok(tc) => {
//                                              Some(tc)
//                                         },
//                                         _ =>  None
//                                     }
//                                 }
//                                 _ = cancel_source => {
//                                     child_cancel_tx.send(()).ok();

//                                     if let Some(input_cancel) = input_cancel.take() {
//                                         input_cancel.send(()).ok();
//                                     }
//                                     None
//                                 }
//                             };

//                             if let Some(task) = stdin_task {
//                                 task.abort();
//                             }
//                             reader_task.abort();
//                             stderr_reader_task.abort();
//                             event!(Level::INFO, "remote canceled");
//                             return output;
//                         }
//                     }
//                 }

//                 log.update_status_only("Could not spawn child process").await;
//                 None
//             } else {
//                 None
//             }
//             }
//         })
//     }
// }
