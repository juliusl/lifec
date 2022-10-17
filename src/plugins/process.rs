use std::{str::from_utf8, path::PathBuf};

use atlier::system::Value;
use reality::Documentation;
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
        parser.add_custom_with("env", |p, value| {
            let var_name = p.symbol().expect("Requires a var name").to_string();

            let last = p.last_child_entity().expect("should have added an entity for the process");

            p.define_child(last, "env", Value::Symbol(var_name.to_string()));
            p.define_child(last, var_name, Value::Symbol(value));
        });

         // Enable .arg to declare arguments
         parser.add_custom_with("arg", |p, value| {
            let last = p.last_child_entity().expect("should have added an entity for the process");

            p.define_child(last, "arg", Value::Symbol(value.to_string()));
        });

        // Enable .flag to declare arguments
        // This will split by spaces and trim
        parser.add_custom_with("flag", |p, value| {
            let last = p.last_child_entity().expect("should have added an entity for the process"); 

            for arg in value.split(" ") {
                p.define_child(last, "arg", Value::Symbol(arg.trim().to_string()));
            }
        });

        // Enable .inherit, will inherit arg/env from previous state
        parser.add_custom_with("inherit", |p, value| {
            let last = p.last_child_entity().expect("should have added an entity for the process"); 
            
            if value.is_empty() {
                p.define_child(last, "inherit", true);
            }

            // todo, could parse and only take the value as a complex
        });

        // Enable .copy_previous, will copy previous state
        parser.add_custom_with("copy_previous", |p, value| {
            let last = p.last_child_entity().expect("should have added an entity for the process"); 
            
            if value.is_empty() {
                p.define_child(last, "copy_previous", true);
            }
        });

        // Enables .cd, setting the current_directory for the process
        parser.add_custom_with("cd", |p, value| {
            let last = p.last_child_entity().expect("should have added an entity for the process"); 
            
            match PathBuf::from(value).canonicalize() {
                Ok(path) => {
                    event!(Level::DEBUG, "Setting current_directory for process entity {}, {:?}", last.id(), path);

                    p.define_child(last, "current_directory", Value::Symbol(path.to_str().expect("should be a string").to_string()));
                },
                Err(err) => {
                    event!(Level::ERROR, "Could not set current_directory for process entity {}, {err}", last.id());
                },
            }
        });

        parser.add_custom_with("redirect",|p, content| {
            let entity = p.last_child_entity().expect("should have a child entity");
            // TODO: can ensure the file,
            p.define_child(entity, "redirect", Value::Symbol(content));
        })
    }

    fn call(context: &super::ThunkContext) -> Option<(JoinHandle<ThunkContext>, CancelToken)> {
        let clone = context.clone();
        clone.clone().task(|cancel_source| {
            let mut tc = context.clone();
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

                match tc.previous() {
                    // If inherit is enabled, inherit env/arg values from previous state
                    Some(previous) if tc.is_enabled("inherit") => {
                        for (env_name, env_val) in previous
                        .find_symbol_values("env")
                        .iter()
                        .filter_map(|e| tc.state().find_symbol(e).and_then(|s| Some((e, s))))
                    {
                        event!(Level::TRACE, "Setting env var {env_name}");
                        command_task.env(env_name, env_val);
                    }
    
                    // Set up any args
                    for arg in previous
                        .find_symbol_values("arg")
                    {
                        event!(Level::TRACE, "Setting arg {arg}");
                        command_task.arg(arg);
                    }
                    }, 
                    _ => {}
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
                                        // TODO: Write to all redirects
                                        let redirects = tc.search().find_symbol_values("redirect");
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

                if tc.is_enabled("copy_previous") {
                    tc.copy_previous();
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
            .optional("flag")
    }

    fn documentation(&self, _property: impl AsRef<str>) -> Option<reality::Documentation> {
        let documentation = match _property.as_ref() {
            "process" => {
                Documentation::summary("Command to execute on the current operating system")
                    .custom_attr()
                    .required()
                    .input()
                    .symbol("This should be the program name and arguments, ex: echo hello world")
            }
            "current_dir" => {
                Documentation::summary("Setting this will execute the command in the context of a specific directory as the root")
                    .custom_attr()
                    .input()
                    .symbol("This should be a file path to the directory.")
            }
            "env" => {
                Documentation::summary("Can be used to set environment variables for the process. Example usage should be, `: ENV_NAME .env value`")
                    .custom_attr()
                    .list()
                    .input()
                    .symbol("This should be the value of the environment variable")
            }
            "arg" => {
                Documentation::summary("Can be used to set the arguments for the process's program")
                    .custom_attr()
                    .input()    
                    .list()
                    .symbol("This should be a single argument w/ no spaces")
            }
            "flag" => {
                Documentation::summary("Can be used to format a flag/value pair")
                    .custom_attr()
                    .input()    
                    .list()
                    .symbol("This should be the flag followed by a space and then value, `ex. --flag value`")
            }
            _ => Documentation::default()
        };

        Some(documentation)
    }

    fn parser(&self) -> Option<CustomAttribute> {
        Some(Process::as_custom_attr())
    }
}
