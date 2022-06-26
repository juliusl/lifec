use super::{BlockContext, Plugin, ThunkContext};
use crate::{AttributeGraph, RuntimeDispatcher, RuntimeState};
use atlier::system::{Value, Extension};
use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use specs::{Component, HashMapStorage};
use std::{
    fmt::Display,
    process::{Command, Output},
};
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
    block: BlockContext,
    #[serde(skip)]
    start_time: Option<DateTime<Utc>>,
}

impl Plugin<ThunkContext> for Process {
    fn symbol() -> &'static str {
        "process"
    }

    fn description() -> &'static str {
        "Executes a new command w/ an OS process."
    }

    fn call_with_context(context: &mut super::ThunkContext) -> Option<JoinHandle<ThunkContext>> {
        context.clone().task(|_| {
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

impl Process {
    fn interpret_command(
        &self,
        expr: impl AsRef<str>,
        interpret: impl Fn(Self, &mut Command) -> Result<Self, ProcessExecutionError>,
    ) -> Result<Self, ProcessExecutionError> {
        let parts: Vec<String> = expr.as_ref().split(" ").map(|p| p.to_string()).collect();

        if let Some(command) = parts.get(0) {
            let subcommands = &parts[1..];
            let process = Process {
                stdout: None,
                stderr: None,
                code: None,
                elapsed: None,
                timestamp_local: None,
                timestamp_utc: None,
                start_time: Some(Utc::now()),
                block: BlockContext::default(),
            };

            let mut command = Command::new(&command);
            let mut command = &mut command;
            for s in subcommands {
                if !s.is_empty() {
                    command = command.arg(s);
                }
            }

            interpret(process, command)
        } else {
            Err(ProcessExecutionError {})
        }
    }

    fn handle_output(mut self, mut command: &mut Command) -> Result<Self, ProcessExecutionError> {
        for (name, value) in &self.block.as_ref().find_symbol_values("flag") {
            if !name.is_empty() {
                command = command.arg(name);
            }
            if let Value::TextBuffer(value) = value {
                command = command.arg(value);
            }
        }

        if let Some(Output {
            status,
            stdout,
            stderr,
        }) = command.output().ok()
        {
            self.stdout = Some(stdout);
            self.stderr = Some(stderr);
            self.code = status.code();
            self.timestamp_utc = Some(Utc::now().to_string());
            self.timestamp_local = Some(Local::now().to_string());
            self.elapsed = self
                .start_time
                .and_then(|s| Some(Utc::now() - s))
                .and_then(|d| Some(format!("{} ms", d.num_milliseconds())));
            Ok(self)
        } else {
            Err(ProcessExecutionError {})
        }
    }
}

impl Display for Process {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "code: {:?}", self.code)
    }
}

impl From<AttributeGraph> for Process {
    fn from(graph: AttributeGraph) -> Self {
        Self {
            block: BlockContext::from(graph),
            ..Default::default()
        }
    }
}

impl AsMut<AttributeGraph> for Process {
    fn as_mut(&mut self) -> &mut AttributeGraph {
        self.block.as_mut()
    }
}

impl AsRef<AttributeGraph> for Process {
    fn as_ref(&self) -> &AttributeGraph {
        &self.block.as_ref()
    }
}

impl RuntimeState for Process {
    type Dispatcher = AttributeGraph;

    fn dispatcher(&self) -> &Self::Dispatcher {
        &self.block.as_ref()
    }

    fn dispatcher_mut(&mut self) -> &mut Self::Dispatcher {
        self.block.as_mut()
    }
}

#[derive(Debug)]
pub struct ProcessExecutionError {}

impl RuntimeDispatcher for Process {
    type Error = ProcessExecutionError;

    fn dispatch_mut(&mut self, msg: impl AsRef<str>) -> Result<(), Self::Error> {
        match self.interpret_command(msg, Self::handle_output) {
            Ok(updated) => {
                *self = updated;
                Ok(())
            }
            Err(err) => Err(err),
        }
    }
}
