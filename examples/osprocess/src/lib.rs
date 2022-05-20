use imgui::{CollapsingHeader, Ui};
use lifec::editor::*;
use lifec::*;
use specs::*;
use std::{
    collections::BTreeMap,
    fmt::Display,
    process::{Command, Output},
};

#[derive(Debug, Clone, Default, Component)]
#[storage(HashMapStorage)]
pub struct Process {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub code: Option<i32>,
    pub command: String,
    pub subcommand: String,
    pub flags: BTreeMap<String, String>,
}

impl Process {
    pub fn edit(section: &mut Section<Process>, ui: &Ui) {
        // Show the default view for this editor
        Process::show_editor(&mut section.state, ui);

        // Retrieve a value from section state
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
            if let (Some(Value::TextBuffer(command)), Some(Value::TextBuffer(subcommand))) = (
                section.get_attr_value("command"),
                section.get_attr_value("subcommand"),
            ) {
                if let Some(next) = section
                    .state
                    .process(&format!("{} {}", command, subcommand))
                    .ok()
                {
                    section.state = next;
                }
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
        ui.label_text("subcommand", &self.subcommand);

        if CollapsingHeader::new("Arguments").begin(ui) {
            self.flags.iter().for_each(|arg_entry| {
                ui.text(format!("{:?}", arg_entry));
            });
        }

        ui.label_text("status code", format!("{:?}", self.code));

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
                Ok(process)
            } else {
                Err(ProcessExecutionError {})
            }
        } else {
            Err(ProcessExecutionError {})
        }
    }

    fn process_with_args<S: AsRef<str> + ?Sized>(
        state: lifec::WithArgs<Self>,
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
                Ok(process)
            } else {
                Err(ProcessExecutionError {})
            }
        } else {
            Err(ProcessExecutionError {})
        }
    }
}
