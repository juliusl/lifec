use atlier::system::{App, Value};
use chrono::{DateTime, Local, Utc};
use imgui::{CollapsingHeader, Ui};
use serde::Deserialize;
use serde::Serialize;
use specs::Component;
use specs::HashMapStorage;
use std::{
    fmt::Display,
    process::{Command, Output},
};

use super::thunks::Thunk;
use crate::RuntimeDispatcher;
use crate::{
    editor::{Section, SectionExtension},
    AttributeGraph, RuntimeState,
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
    graph: AttributeGraph,
    #[serde(skip)]
    start_time: Option<DateTime<Utc>>,
}

impl Process {
    fn command(&self) -> Option<String> {
        let command = self.graph.find_attr_value("command");
        let subcommands = self.graph.find_attr_value("subcommands");

        if let (Some(Value::TextBuffer(command)), Some(Value::TextBuffer(subcommands))) =
            (command, subcommands)
        {
            Some(format!("{}::{}", command, subcommands))
        } else {
            None
        }
    }
}

impl Thunk for Process {
    fn symbol() -> &'static str {
        "process"
    }

    fn description() -> &'static str {
        "Executes a new command w/ an OS process."
    }

    fn call_with_context(context: &mut super::ThunkContext) {
        let process = Self::from(context.as_ref().clone());

        if let Some(command) = process.command() {
            match process.dispatch(command) {
                Ok(mut output) => {
                    output.stdout = output.stdout.and_then(|o| {
                        context.write_output("stdout", Value::BinaryVector(o));
                        None
                    });

                    output.stderr = output.stderr.and_then(|o| {
                        context.write_output("stderr", Value::BinaryVector(o));
                        None
                    });

                    if let Some(code) = output.code {
                        context.write_output("code", Value::Int(code));
                    }

                    if let Some(local_ts) = output.timestamp_local {
                        context.write_output("timestamp_local", Value::TextBuffer(local_ts));
                    }

                    if let Some(utc_ts) = output.timestamp_utc {
                        context.write_output("timestamp_utc", Value::TextBuffer(utc_ts));
                    }

                    if let Some(elapsed) = output.elapsed {
                        context.write_output("elapsed", Value::TextBuffer(elapsed));
                    }
                    context.set_return::<Process>(Value::Bool(true));
                    context.as_mut().find_remove("error");
                }
                Err(e) => {
                    context
                        .as_mut()
                        .with("error", Value::TextBuffer(format!("Error: {:?}", e)));
                }
            }
        }
    }
}

impl Process {
    fn interpret_command(
        &self,
        expr: impl AsRef<str>,
        interpret: impl Fn(Self, &mut Command) -> Result<Self, ProcessExecutionError>,
    ) -> Result<Self, ProcessExecutionError> {
        let parts: Vec<String> = expr.as_ref().split("::").map(|p| p.to_string()).collect();

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
                graph: AttributeGraph::default(),
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
        for (name, value) in &self.graph.find_symbol_values("flag") {
            if !name.is_empty() {
                command = command.arg(name);
            }

            if let Value::TextBuffer(value) = value {
                command = command.arg(value);
            }
        }

        let output = command.output().ok();
        if let Some(Output {
            status,
            stdout,
            stderr,
        }) = output
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

    fn edit(&mut self, ui: &Ui) {
        self.as_mut().edit_attr("Enable node editor", "enable_node_editor", ui);

        // Show the default view for this editor
        self.show_editor(ui);

        ui.new_line();
        // Some tools to edit the process
        ui.text("Edit Process:");
        ui.new_line();
        self.as_mut().edit_attr(
            "edit the process command",
            "command",
            ui,
        );
        self.as_mut().edit_attr(
            "edit process subcommands",
            "subcommands",
            ui,
        );

        if ui.button("execute") {
            match (
                self.as_ref().find_attr_value("command"),
                self.as_ref().find_attr_value("subcommands"),
            ) {
                (Some(Value::TextBuffer(command)), Some(Value::TextBuffer(subcommand))) => {
                    if let Some(next) = self
                        .as_ref()
                        .dispatch(&format!("{}::{}", command, subcommand))
                        .ok()
                    {
                        self.graph = next;
                    } else {
                        eprintln!("did not execute `{} {}`", command, subcommand);
                    }
                }
                (Some(Value::TextBuffer(command)), None) => {
                    if let Some(next) = self.as_ref().dispatch(&&format!("{}::", command)).ok() {
                        self.graph = next;
                    } else {
                        eprintln!("did not execute `{}`", command);
                    }
                }
                _ => (),
            }
        }
    }
}

impl Display for Process {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "code: {:?}", self.code)
    }
}

impl App for Process {
    fn name() -> &'static str {
        "Process (Start/Configure OS Processes)"
    }

    fn show_editor(&mut self, ui: &imgui::Ui) {
        if let Some(Value::TextBuffer(command)) = self.graph.find_attr_value("command"){
            ui.label_text("Command", command);
        }

        if let Some(Value::TextBuffer(command)) = self.graph.find_attr_value("subcommands"){
            ui.label_text("Subcommand", command);
        }

        let flags = self.graph.find_symbol_values("flag");

        if !flags.is_empty() {
            if CollapsingHeader::new("Arguments").begin(ui) {
                flags.iter().for_each(|arg_entry| {
                    ui.text(format!("{:?}", arg_entry));
                });
            }
        }
    }
}

#[derive(Debug)]
pub struct ProcessExecutionError {}

impl From<AttributeGraph> for Process {
    fn from(graph: AttributeGraph) -> Self {
        Self { graph, stdout: None, stderr: None, code: None, elapsed: None, timestamp_utc: None, timestamp_local: None, start_time: None }
    }
}

impl AsMut<AttributeGraph> for Process {
    fn as_mut(&mut self) -> &mut AttributeGraph {
        &mut self.graph
    }
}

impl AsRef<AttributeGraph> for Process {
    fn as_ref(&self) -> &AttributeGraph {
        &self.graph
    }
}

impl RuntimeState for Process {
    type Dispatcher = AttributeGraph;

    fn dispatcher(&self) -> &Self::Dispatcher {
        &self.graph
    }

    fn dispatcher_mut(&mut self) -> &mut Self::Dispatcher {
        &mut self.graph
    }
}

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
