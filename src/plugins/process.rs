use std::{str::from_utf8, path::PathBuf};

use atlier::system::Value;
use crate::{AttributeParser, BlockObject, BlockProperties, CustomAttribute};
use specs::{Component, DenseVecStorage};
use tokio::{select, task::JoinHandle};

use crate::{
    plugins::{thunks::CancelToken, Plugin, ThunkContext},
    AttributeIndex,
};

use tracing::event;
use tracing::Level;

/// The process component executes a command and records the output
/// 
#[derive(Debug, Clone, Default, Component)]
#[storage(DenseVecStorage)]
pub struct Process;

impl Plugin for Process {
    fn symbol() -> &'static str {
        "process"
    }

    fn description() -> &'static str {
        "Executes a new command w/ an OS process."
    }

    fn compile(parser: &mut AttributeParser) {
        // Enable .env to declare environment variables
        parser.add_custom(CustomAttribute::new_with("env", |p, value| {
            let var_name = p.symbol().expect("Requires a var name").to_string();

            let last = p.last_child_entity().expect("should have added an entity for the process");

            p.define_child(last, "env", Value::Symbol(var_name.to_string()));
            p.define_child(last, var_name, Value::Symbol(value));
        }));

         // Enable .arg to declare environment variables
         parser.add_custom(CustomAttribute::new_with("arg", |p, value| {
            let last = p.last_child_entity().expect("should have added an entity for the process");

            p.define_child(last, "arg", Value::Symbol(value.to_string()));
        }));
    }

    fn call(context: &super::ThunkContext) -> Option<(JoinHandle<ThunkContext>, CancelToken)> {
        let clone = context.clone();
        clone.clone().task(|cancel_source| {
            let tc = context.clone();
            async move {
                let command = tc
                    .state()
                    .find_symbol("process")
                    .expect("missing process property");
                
                event!(Level::TRACE, "Creating command for {command}");

                let mut args = command.split(" ");

                let command = args.next().expect("should have at least one argument that is the program");
                let mut command_task = tokio::process::Command::new(command);
                command_task.args(args);

                command_task.kill_on_drop(true);

                // Set up any env variables
                for (env_name, env_val) in tc
                    .state()
                    .find_symbol_values("env")
                    .iter()
                    .filter_map(|e| tc.state().find_symbol(e).and_then(|s| Some((e, s))))
                {
                    event!(Level::TRACE, "Setting env var {env_name}");
                    command_task.env(env_name, env_val);
                }

                // Set up any args
                for arg in tc
                    .state()
                    .find_symbol_values("arg")
                {
                    event!(Level::TRACE, "Setting arg {arg}");
                    command_task.arg(arg);
                }

                // Set current directory if work_dir is set
                if let Some(work_dir) = tc.state().find_symbol("current_dir") {
                    let path = PathBuf::from(&work_dir);
                    match path.canonicalize() {
                        Ok(work_dir) => {
                            command_task.current_dir(work_dir);
                        },
                        Err(err) => {
                            panic!("Could not canonicalize path {work_dir}, {err}");
                        },
                    }
                }

                select! {
                   output = command_task.output() => {
                        match output {
                            Ok(output) => {
                                
                                // TODO
                                // for b in output.stdout.clone() {
                                //     tc.send_char(b).await;
                                // }
                                // for b in output.stderr.clone() {
                                //     tc.send_char(b).await;
                                // }

                                let stdout = output.stdout.clone();
                                match from_utf8(stdout.as_slice()) {
                                    Ok(stdout) => {
                                        for line in stdout.lines() {
                                            println!("{}", line);
                                        }
                                    },
                                    Err(err) => {
                                        event!(Level::ERROR, "Could not read stdout {err}")
                                    },
                                }

                                let stderr = output.stderr.clone();
                                match from_utf8(stderr.as_slice()) {
                                    Ok(stderr) => {
                                        for line in stderr.lines() {
                                            eprintln!("{}", line);
                                        }
                                    },
                                    Err(err) => {
                                        event!(Level::ERROR, "Could not read stdout {err}")
                                    },
                                }
                                // Completed process, publish result
                                tc.update_progress("# Finished, recording output", 0.30).await;
                                // Self::resolve_output(&mut tc, command, start_time, output);
                            }
                            Err(err) => {
                                let path = std::env::current_dir().expect("should be able to get current dir");
                                event!(Level::TRACE, "The current directory is {}", path.display());
                                tc.update_progress(format!("# error {}", err), 0.0).await;
                            }
                        }
                   }
                   _ = cancel_source => {
                        tc.update_progress(format!("# cancelling"), 0.0).await;
                   }
                }

                Some(tc)
            }
        })
    }
}

impl BlockObject for Process {
    fn query(&self) -> BlockProperties {
        BlockProperties::new("runtime")
            .require("process")
            .optional("current_dir")
            .optional("env")
            .optional("arg")
    }

    fn parser(&self) -> Option<CustomAttribute> {
        Some(Process::as_custom_attr())
    }
}
