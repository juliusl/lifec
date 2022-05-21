use atlier::system::{App, Value};
use chrono::{Local, Utc, DateTime};
use imgui::{CollapsingHeader, Ui};
use specs::Component;
use specs::HashMapStorage;
use std::{
    collections::BTreeMap,
    fmt::Display,
    process::{Command, Output}
};

use crate::{RuntimeState, editor::{Section, SectionExtension}, WithArgs, parse_flags};

#[derive(Debug, Clone, Default, Component)]
#[storage(HashMapStorage)]
pub struct Process {
    pub command: String,
    pub subcommand: String,
    pub flags: BTreeMap<String, String>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub code: Option<i32>,
    pub start_time: Option<DateTime<Utc>>,
    pub elapsed: Option<String>,
    pub last_timestamp_utc: Option<String>,
    pub last_timestamp_local: Option<String>,
}

impl SectionExtension<Process> for Process
{
    fn show_extension(section: &mut Section<Process>, ui: &imgui::Ui) {
        Process::edit(section, ui);
    }
}

impl Process {
    fn edit(section: &mut Section<Process>, ui: &Ui) {
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
            "edit the process subcommand",
            "subcommand",
            |s| Some(&mut s.subcommand),
            ui,
        );

        if ui.button("execute") {
            match ( section.get_attr_value("command"),
                    section.get_attr_value("subcommand")
                  ) {
                (Some(Value::TextBuffer(command)), Some(Value::TextBuffer(subcommand))) => {
                    if let Some(next) = section
                        .state
                        .process(&format!("{} {}", command, subcommand))
                        .ok()
                    {
                        section.state = next;
                    }
                }
                (Some(Value::TextBuffer(command)), None) => {
                    if let Some(next) = section
                        .state
                        .process(&command)
                        .ok()
                    {
                        section.state = next;
                    }
                }
                _ => (),
            }
        }

        section.state.flags.clear();
        section
            .attributes
            .clone()
            .iter_mut()
            .filter(|f| f.name().starts_with("arg::"))
            .for_each(|arg| {
                if let Value::TextBuffer(value) = arg.value() {
                    let arg_name = &arg.name()[5..];
                    section.edit_state_string(
                        format!("edit flag {}", arg_name),
                        arg.name(),
                        |s| {
                            if let None = s.flags.get(arg_name) {
                                s.flags.insert(arg_name.to_string(), value.to_string());

                            }
                            s.flags.get_mut(arg_name)
                        },
                        ui,
                    );
                }
            });
    }
}

impl Display for Process {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "os process")?;

        Ok(())
    }
}

impl App for Process {
    fn name() -> &'static str {
        "Process (Start/Configure OS Processes)"
    }

    fn show_editor(&mut self, ui: &imgui::Ui) {
        ui.label_text("command", &self.command);

        if !self.subcommand.is_empty() {
            ui.label_text("subcommand", &self.subcommand);
        }

        if !self.flags.is_empty() {
            if CollapsingHeader::new("Arguments").begin(ui) {
                self.flags.iter().for_each(|arg_entry| {
                    ui.text(format!("{:?}", arg_entry));
                });
            }
        }

        if (!self.stdout.is_empty() || !self.stderr.is_empty()) && self.code.is_some() {
            if CollapsingHeader::new(format!("Standard I/O, Exit Code: {:?}, Local Timestamp: {:?}, UTC Timestamp: {:?}, Elapsed: {:?}", self.code, self.last_timestamp_local, self.last_timestamp_utc, self.elapsed)).leaf(true).begin(ui) {
                if let Some(mut output) = String::from_utf8(self.stdout.to_vec()).ok() {
                    ui.input_text_multiline("stdout", &mut output, [0.0, 0.0])
                        .read_only(true)
                        .build();
                }
        
                if let Some(mut output) = String::from_utf8(self.stderr.to_vec()).ok() {
                    ui.input_text_multiline("stderr", &mut output, [0.0, 0.0])
                        .read_only(true)
                        .build();
                }
            }
        }
    }
}

pub struct ProcessExecutionError {}

impl RuntimeState for Process {
    type Error = ProcessExecutionError;

    fn load<'a, S: AsRef<str> + ?Sized>(&self, _: &'a S) -> Self
    where
        Self: Sized,
    {
        todo!()
    }

    fn process<'a, S: AsRef<str> + ?Sized>(&self, msg: &'a S) -> Result<Self, Self::Error> {
        let parts = msg.as_ref().split(" ");
        let command = parts.clone().take(1);
        let command: Vec<&str> = command.collect();
        if let Some(command) = command.get(0) {
            let subcommand: Vec<&str> = parts.skip(1).collect();

            let mut process = Process {
                stdout: vec![],
                stderr: vec![],
                code: None,
                command: command.to_string(),
                subcommand: subcommand.join(" "),
                flags: self.flags.clone(),
                start_time: Some(Utc::now()),
                elapsed: None,
                last_timestamp_local: None,
                last_timestamp_utc: None
            };

            let mut command = Command::new(&process.command);
            let mut command = &mut command;

            if !&process.subcommand.is_empty() {
                command = command.arg(&process.subcommand);
            }

            let output = command.output().ok();
            if let Some(Output {
                status,
                stdout,
                stderr,
            }) = output
            {
                process.stdout = stdout;
                process.stderr = stderr;
                process.code = status.code();
                process.last_timestamp_utc = Some(Utc::now().to_string());
                process.last_timestamp_local = Some(Local::now().to_string());
                process.elapsed = process.start_time.and_then(|s| Some(Utc::now()-s)).and_then(|d| Some(format!("{} ms", d.num_milliseconds())));
                Ok(process)
            } else {
                Err(ProcessExecutionError {})
            }
        } else {
            Err(ProcessExecutionError {})
        }
    }

    fn process_with_args<S: AsRef<str> + ?Sized>(
        state: WithArgs<Self>,
        msg: &S,
    ) -> Result<Self, Self::Error>
    where
        Self: Clone + Default + RuntimeState,
    {
        let parts = msg.as_ref().split(" ");
        let command = parts.clone().take(1);
        let command: Vec<&str> = command.collect();
        if let Some(command) = command.get(0) {
            let subcommand: Vec<&str> = parts.skip(1).collect();

            let mut process = Process {
                stdout: vec![],
                stderr: vec![],
                code: None,
                command: command.to_string(),
                subcommand: subcommand.join(" "),
                flags: parse_flags(state.get_args().to_vec()),
                start_time: Some(Utc::now()),
                elapsed: None,
                last_timestamp_utc: None,
                last_timestamp_local: None,
            };

            let mut command = Command::new(&process.command);
            let mut command = &mut command;

            if !&process.subcommand.is_empty() {
                command = command.arg(&process.subcommand);
            }

            let output = command.args(state.get_args()).output().ok();
            if let Some(Output {
                status,
                stdout,
                stderr,
            }) = output
            {
                process.stdout = stdout;
                process.stderr = stderr;
                process.code = status.code();
                process.last_timestamp_local = Some(Local::now().to_string());
                process.last_timestamp_utc = Some(Utc::now().to_string());
                process.elapsed = process.start_time.and_then(|s| Some(Utc::now()-s)).and_then(|d| Some(format!("{} ms", d.num_milliseconds())));
                Ok(process)
            } else {
                Err(ProcessExecutionError {})
            }
        } else {
            Err(ProcessExecutionError {})
        }
    }
}
