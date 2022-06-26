use super::{Plugin, ThunkContext};
use atlier::system::Extension;
use chrono::{Local, Utc};
use serde::{Deserialize, Serialize};
use specs::{Component, HashMapStorage};
use tokio::task::JoinHandle;

#[derive(Debug, Clone, Default, Component, Serialize, Deserialize)]
#[storage(HashMapStorage)]
pub struct Process {
    pub stdout: Option<Vec<u8>>,
    pub stderr: Option<Vec<u8>>,
    pub code: Option<i32>,
    pub elapsed: Option<String>,
    pub timestamp_utc: Option<String>,
    pub timestamp_local: Option<String>,
}

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
                    let mut command_task = tokio::process::Command::new(&command);                    
                    
                    // TODO: Handle args, and env

                    let start_time = Some(Utc::now());
                    if let Some(output) = command_task.output().await.ok() {
                        
                        // Completed process, publish result
                        tc.publish(|publish_block| {
                            let timestamp_utc = Some(Utc::now().to_string());
                            let timestamp_local = Some(Local::now().to_string());
                            let elapsed = start_time
                                .and_then(|s| Some(Utc::now() - s))
                                .and_then(|d| Some(format!("{} ms", d.num_milliseconds())));
                            publish_block
                                .with_text("command", &command)
                                .with_int("code", output.status.code().unwrap_or_default())
                                .with_binary("stdout", output.stdout)
                                .with_binary("stderr", output.stderr)
                                .with_text("timestamp_local", timestamp_local.unwrap_or_default())
                                .with_text("timestamp_utc", timestamp_utc.unwrap_or_default())
                                .with_text("elapsed", elapsed.unwrap_or_default())
                                .with_bool("called", true);
                        });
                    }
                }
                Some(tc)
            }
        })
    }
}


impl Extension for Process {
    fn configure_app_world(_: &mut specs::World) {
        todo!()
    }

    fn configure_app_systems(_: &mut specs::DispatcherBuilder) {
        todo!()
    }

    fn on_ui(&'_ mut self, _: &specs::World, _: &'_ imgui::Ui<'_>) {
        todo!()
    }

    fn on_window_event(&'_ mut self, _: &specs::World, _: &'_ atlier::system::WindowEvent<'_>) {
        todo!()
    }

    fn on_run(&'_ mut self, _: &specs::World) {
        todo!()
    }
}
