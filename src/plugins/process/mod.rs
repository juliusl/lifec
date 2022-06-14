use atlier::system::{Value, Extension};
use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use specs::{Component, HashMapStorage, WorldExt, Builder};
use std::{
    process::{Command, Output}, fmt::Display,
};
use crate::{RuntimeDispatcher, AttributeGraph, RuntimeState};
use super::{Plugin, ThunkContext, Edit};

mod echo;
pub use echo::Echo;

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

        if let Some(Value::TextBuffer(command)) = command {
            Some(command.to_string())
        } else {
            None
        }
    }
}

impl Extension for Process {
    fn configure_app_world(world: &mut specs::World) {
        world.register::<ThunkContext>();
        world.register::<Edit<ThunkContext>>();
        world.register::<Process>();
        world.register::<Edit<Process>>();
    
        if let Some(graph) = AttributeGraph::load_from_file("process.runmd") {
            for process in graph.find_blocks("process") {
                world.create_entity()
                    .with(Process::from(process))
                    .maybe_with(Some(Edit::<ThunkContext>(|_, _, _| {

                    })))
                    .maybe_with(Some(Edit::<Process>(|_, _, _| {
                        
                    })))
                    .build();
            }
        }
    }

    fn configure_app_systems(_: &mut specs::DispatcherBuilder) {
    }

    fn on_ui(&mut self, _app_world: &specs::World, _ui: &imgui::Ui<'_>) {
    }
}

impl Plugin<ThunkContext> for Process {
    fn symbol() -> &'static str {
        "process"
    }

    fn description() -> &'static str {
        "Executes a new command w/ an OS process."
    }

    fn call_with_context(context: &mut super::ThunkContext) {
        let process = Self::from(context.as_ref().clone());
        if let Some(command) = process.command() {
            match process.interpret_command(command, Process::handle_output) {
                Ok(mut output) => {
                    output.stdout = output.stdout.and_then(|o| {
                        context.write_output(
                            "stdout", 
                            Value::BinaryVector(o)
                        );
                        None
                    });

                    output.stderr = output.stderr.and_then(|o| {
                        context.write_output(
                            "stderr", 
                            Value::BinaryVector(o)
                        );
                        None
                    });

                    if let Some(code) = output.code {
                        context.write_output("code", Value::Int(code));
                    }

                    if let Some(local_ts) = output.timestamp_local {
                        context.write_output(
                            "timestamp_local", 
                            Value::TextBuffer(local_ts));
                    }

                    if let Some(utc_ts) = output.timestamp_utc {
                        context.write_output(
                            "timestamp_utc", 
                            Value::TextBuffer(utc_ts)
                        );
                    }

                    if let Some(elapsed) = output.elapsed {
                        context.write_output(
                            "elapsed", 
                            Value::TextBuffer(elapsed)
                        );
                    }

                    context.set_return::<Process>(
                        "called", 
                        Value::Bool(true)
                    );

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
        Self { graph, 
            ..Default::default()
        }
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
