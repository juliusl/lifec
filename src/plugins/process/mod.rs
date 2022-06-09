use atlier::system::{App, Value};
use chrono::{DateTime, Local, Utc};
use imgui::{CollapsingHeader, Ui};
use serde::Deserialize;
use serde::Serialize;
use specs::Component;
use specs::HashMapStorage;
use std::{
    collections::BTreeMap,
    fmt::Display,
    process::{Command, Output},
};

use super::thunks::Thunk;
use crate::RuntimeDispatcher;
use crate::{
    editor::{Section, SectionExtension}, AttributeGraph, RuntimeState, Runtime
};

#[derive(Debug, Clone, Default, Component, Serialize, Deserialize)]
#[storage(HashMapStorage)]
pub struct Process {
    pub command: String,
    pub subcommands: String,
    pub flags: BTreeMap<String, String>,
    pub vars: BTreeMap<String, String>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub code: Option<i32>,
    pub elapsed: Option<String>,
    pub timestamp_utc: Option<String>,
    pub timestamp_local: Option<String>,
    #[serde(skip)]
    start_time: Option<DateTime<Utc>>,
}

impl Thunk for Process {
    fn symbol() -> &'static str {
        "process"
    }

    fn call_with_context(context: &mut super::ThunkContext) {
        let _ = Self::from(context.state().clone());

        // match process.dispatch(&process.command) {
        //     Ok(output) => {
        //         context.set_output("stdout", Value::BinaryVector(output.stdout));
        //         context.set_returns(Value::Bool(true));
        //         context.state_mut().as_mut().find_remove("error");
        //     }
        //     Err(e) => {
        //         context.state_mut().as_mut().with(
        //             "error".to_string(),
        //             Value::TextBuffer(format!("Error: {:?}", e)),
        //         );
        //     }
        // }
    }
}

impl SectionExtension<Process> for Process {
    fn show_extension(section: &mut Section<Process>, ui: &imgui::Ui) {
        Process::edit(section, ui);
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
                stdout: vec![],
                stderr: vec![],
                code: None,
                command: command.to_string(),
                subcommands: subcommands.join("::"),
                flags: self.flags.clone(),
                vars: self.vars.clone(),
                start_time: Some(Utc::now()),
                elapsed: None,
                timestamp_local: None,
                timestamp_utc: None,
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
        if !self.flags.is_empty() {
            for (name, value) in &self.flags {
                if !name.is_empty() {
                    command = command.arg(name);
                }

                if !value.is_empty() {
                    command = command.arg(value);
                }
            }
        }

        let output = command.output().ok();
        if let Some(Output {
            status,
            stdout,
            stderr,
        }) = output
        {
            self.stdout = stdout;
            self.stderr = stderr;
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

    fn edit(section: &mut Section<Process>, ui: &Ui) {
        section.edit_attr("Enable node editor", "enable node editor", ui);

        // Show the default view for this editor
        Process::show_editor(&mut section.state, ui);

        ui.new_line();
        // Some tools to edit the process
        ui.text("Edit Process:");
        ui.new_line();
        section.edit_state_string(
            "edit the process command",
            "command",
            |s| Some(&mut s.command),
            ui,
        );
        section.edit_state_string(
            "edit process subcommands",
            "subcommands",
            |s| Some(&mut s.subcommands),
            ui,
        );

        if ui.button("execute") {
            match (
                section.attributes.find_attr_value("command"),
                section.attributes.find_attr_value("subcommands"),
            ) {
                (Some(Value::TextBuffer(command)), Some(Value::TextBuffer(subcommand))) => {
                    if let Some(next) = section
                        .state
                        .dispatcher()
                        .dispatch(&format!("{}::{}", command, subcommand))
                        .ok()
                    {
                        section.state = Process::from(next);
                    }
                }
                (Some(Value::TextBuffer(command)), None) => {
                    if let Some(next) = section
                        .state
                        .dispatcher()
                        .dispatch(&&format!("{}::", command))
                        .ok()
                    {
                        section.state = Process::from(next);
                    }
                }
                _ => (),
            }
        }

        section.state.flags.clear();
        // section
        //     .attributes
        //     .clone()
        //     .iter_mut()
        //     .filter(|(_, f)| f.name().starts_with("arg::"))
        //     .for_each(|(_, arg)| {
        //         let arg_name = &arg.name()[5..];
        //         if arg_name.is_empty() {
        //             return;
        //         }

        //         if let Value::TextBuffer(value) = arg.value() {
        //             section.edit_state_string(
        //                 format!("edit flag {}", arg_name),
        //                 arg.name(),
        //                 |s| {
        //                     if let None = s.flags.get(arg_name) {
        //                         s.flags.insert(arg_name.to_string(), value.to_string());
        //                     }
        //                     s.flags.get_mut(arg_name)
        //                 },
        //                 ui,
        //             );
        //         }
        //     });
    }
}

impl Display for Process {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "code: {:?}", self.code)?;
        writeln!(f, "stdout: {:?}", String::from_utf8(self.stdout.to_vec()))?;
        writeln!(f, "stderr: {:?}", String::from_utf8(self.stderr.to_vec()))
    }
}

impl App for Process {
    fn name() -> &'static str {
        "Process (Start/Configure OS Processes)"
    }

    fn show_editor(&mut self, ui: &imgui::Ui) {
        if !self.command.is_empty() {
            ui.label_text("Command", &self.command);
        }

        if !self.subcommands.is_empty() {
            ui.label_text("Subcommand", &self.subcommands);
        }

        if !self.flags.is_empty() {
            if CollapsingHeader::new("Arguments").begin(ui) {
                self.flags.iter().for_each(|arg_entry| {
                    ui.text(format!("{:?}", arg_entry));
                });
            }
        }

        if (!self.stdout.is_empty() || !self.stderr.is_empty()) && self.code.is_some() {
            ui.separator();
            ui.label_text("Exit Code", format!("{:?}", self.code));
            ui.label_text(
                "Local Timestamp",
                self.timestamp_local.as_deref().unwrap_or_default(),
            );
            ui.label_text(
                "UTC Timestamp",
                self.timestamp_utc.as_deref().unwrap_or_default(),
            );
            ui.label_text("Elapsed", self.elapsed.as_deref().unwrap_or_default());

            ui.separator();
            if let Some(mut output) = String::from_utf8(self.stdout.to_vec()).ok() {
                ui.input_text_multiline("Stdout", &mut output, [0.0, 0.0])
                    .read_only(true)
                    .build();
            }

            if let Some(mut output) = String::from_utf8(self.stderr.to_vec()).ok() {
                ui.input_text_multiline("Stderr", &mut output, [0.0, 0.0])
                    .read_only(true)
                    .build();
            }
        }
    }
}

#[derive(Debug)]
pub struct ProcessExecutionError {}

impl From<AttributeGraph> for Process {
    fn from(_: AttributeGraph) -> Self {
        todo!();
    }
}

impl AsMut<AttributeGraph> for Process {
    fn as_mut(&mut self) -> &mut AttributeGraph {
        todo!()
    }
}

impl AsRef<AttributeGraph> for Process {
    fn as_ref(&self) -> &AttributeGraph {
        todo!()
    }
}

impl RuntimeDispatcher for Process {
    type Error = ProcessExecutionError;

    fn dispatch_mut(&mut self, msg: impl AsRef<str>) -> Result<(), Self::Error> {
        match self.interpret_command(msg, Self::handle_output) {
            Ok(updated) => {
                *self = updated;
                Ok(())
            },
            Err(err) => Err(err),
        }
    }
}

impl RuntimeState for Process {
    type Dispatcher = Self;

    fn dispatcher(&self) -> &Self::Dispatcher {
        self
    }

    fn dispatcher_mut(&mut self) -> &mut Self::Dispatcher {
        self
    }
}

#[derive(Clone, Default)]
struct Process2(AttributeGraph); 

impl From<AttributeGraph> for Process2 {
    fn from(g: AttributeGraph) -> Self {
        Self(g)
    }
}

impl Display for Process2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl RuntimeState for Process2 {
    type Dispatcher = AttributeGraph;

    fn dispatcher(&self) -> &Self::Dispatcher {
        &self.0
    }

    fn dispatcher_mut(&mut self) -> &mut Self::Dispatcher {
        &mut self.0
    }

    fn setup_runtime(&mut self, runtime: &mut Runtime::<Self>) {
        runtime.with_call_mut("interpret_command", |s, e| {
            
            "{ exit;; }".to_string()
        });
    }
}