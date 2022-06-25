use super::{Plugin, ThunkContext, BlockContext, Engine};
use crate::{AttributeGraph, RuntimeDispatcher, RuntimeState};
use atlier::system::Value;
use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use specs::{Component, HashMapStorage};
use tokio::{runtime::Handle, task::JoinHandle};
use std::{
    fmt::Display,
    process::{Command, Output}
};

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

impl Process {
    fn command(&self) -> Option<String> {
        if let Some(form) = self.block.get_block("form") {
            form.find_text("command")
        } else {
            None 
        }
    }

    fn debug_out_enabled(&self) -> Option<bool> {
        if let Some(form) = self.block.get_block("form") {
            form.is_enabled("debug_out")
        } else {
            None
        }
    }
}

struct Start;

impl Engine<Process> for Start {
    fn event_name() -> &'static str {
        "start"
    }
}

impl Plugin<ThunkContext> for Process {
    fn symbol() -> &'static str {
        "process"
    }

    fn description() -> &'static str {
        "Executes a new command w/ an OS process."
    }

    fn call_with_context(context: &mut super::ThunkContext, handle: Option<Handle>) -> Option<JoinHandle<()>> {
        if let Some(handle) = handle {
            handle.block_on(async {
                
            });
        }

        if let Some(process) = context
            .as_ref()
            .find_block("", "form")
            .and_then(|a| Some(Self::from(a)))
        {
            if let Some(command) = process.command() {
                match process.interpret_command(&command, Process::handle_output) {
                    Ok(output) => {
                        if let Some(true) = process.debug_out_enabled() {
                            println!("{:?}", &output.stdout.as_ref().and_then(|o| String::from_utf8(o.to_vec()).ok()));
                        }
                        
                        // publish the result
                        context.publish(|publish_block| {
                            publish_block
                                .with_text("command", command)
                                .with_int("code", output.code.unwrap_or_default())
                                .with_binary("stdout", output.stdout.unwrap_or_default())
                                .with_binary("stderr", process.stdout.unwrap_or_default())
                                .with_text("timestamp_local", output.timestamp_local.unwrap_or_default())
                                .with_text("timestamp_utc", output.timestamp_utc.unwrap_or_default())
                                .with_text("elapsed", output.elapsed.unwrap_or_default())
                                .with_bool("called", true);
                        });

                        context.as_mut().find_remove("error");
                    }
                    Err(e) => {
                        if let Some(true) = process.as_ref().is_enabled("debug_out") {
                            eprintln!("{:?}", &e);
                        }
                        context.error(|a| a.add_text_attr("error", format!("Error: {:?}", e)))
                    }
                }
            }
        }
    
        None
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
