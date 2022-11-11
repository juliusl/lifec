use std::{path::PathBuf, process::Stdio};
use reality::Documentation;
use tokio::{io::{BufReader, AsyncBufReadExt}, select};

use crate::prelude::*;

/// The process component executes a command and records the output
/// 
#[derive(Debug, Clone, Default)]
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
            let last = p.last_child_entity().expect("should have added an entity for the process");

            if let Some(var_name) = p.symbol() {
                let var_name = var_name.to_string();
                p.define_child(last, "env", Value::Symbol(var_name.to_string()));
                if !value.is_empty() {
                    p.define_child(last, var_name, Value::Symbol(value));
                }
            } else {
                p.define_child(last, "env", Value::Symbol(value.to_string()));
            }
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

        // Enables redirecting stdout to a file, 
        parser.add_custom_with("redirect",|p, content| {
            let entity = p.last_child_entity().expect("should have a child entity");
            // TODO: can ensure the file,
            p.define_child(entity, "redirect", Value::Symbol(content));
        });

        // Cache output from process
        parser.add_custom_with("cache_output",|p, _| {
            let entity = p.last_child_entity().expect("should have a child entity");
            // TODO: can ensure the file,
            p.define_child(entity, "cache_output", true);
        });

                // Silent stdout/stderr from stream
        parser.add_custom_with("silent",|p, _| {
            let entity = p.last_child_entity().expect("should have a child entity");
            // TODO: can ensure the file,
            p.define_child(entity, "silent", true);
        });
        
    }

    fn call(context: &mut ThunkContext) -> Option<AsyncContext> {
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
                    .search()
                    .find_symbol_values("env")
                    .iter()
                    .filter_map(|e| tc.search().find_symbol(e).and_then(|s| Some((e, s))))
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
                        .filter_map(|e| previous.find_symbol(e).and_then(|s| Some((e, s))))
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

                // Set current directory if current_dir is set
                if let Some(current_dir) = tc.state().find_symbol("current_dir") {
                    let path = PathBuf::from(&current_dir);
                    match path.canonicalize() {
                        Ok(current_dir) => {
                            command_task.current_dir(current_dir);
                        },
                        Err(err) => {
                            panic!("Could not canonicalize path {current_dir}, {err}");
                        },
                    }
                }

                command_task
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());

                let mut child = command_task.spawn().expect("should be able to spawn process");

                let stdout = child.stdout.take().expect("should be able to take stdout");
                let stderr = child.stderr.take().expect("should be able to take stderr");

                let mut reader = BufReader::new(stdout).lines();
                let mut stderr_reader = BufReader::new(stderr).lines();

                let reader_context = tc.clone();
                let reader_task = tc.handle().unwrap().spawn(async move {
                    event!(Level::DEBUG, "starting to listen to stdout");
                    
                    let mut stdout = String::new();

                    while let Ok(line) = reader.next_line().await {
                        match line {
                            Some(line) => {
                                use std::fmt::Write;
                                
                                if !reader_context.is_enabled("silent") {
                                    println!("{}", line);
                                }
                                writeln!(&mut stdout, "{}", line).expect("should be able to write");
                                reader_context.status(format!("0: {}", line)).await;
                            },
                            None => {
                                break;
                            },
                        }
                    }

                    for redirect in reader_context.search().find_symbol_values("redirect") {
                        match tokio::fs::write(&redirect, &stdout).await {
                            Ok(_) => {
                                event!(Level::DEBUG, "Redirected output to {redirect}");
                            },
                            Err(err) => {
                                event!(Level::ERROR, "Could not write to {redirect}, {err}");
                            },
                        }
                    }

                    stdout
                });

                let err_reader_context = tc.clone();
                let stderr_reader_task = tc.handle().unwrap().spawn(async move {
                    event!(Level::DEBUG, "starting to listen to stderr");
                    while let Ok(line) = stderr_reader.next_line().await {
                        match line {
                            Some(line) => {
                                if !err_reader_context.is_enabled("silent") {
                                    eprintln!("{}", line);
                                }

                                err_reader_context.status(format!("1: {}", line)).await;
                            },
                            None => {
                                break;
                            },
                        }
                    }
                });

                select! {
                   output = child.wait_with_output() => {
                        match output {
                            Ok(_) => {
                                event!(Level::DEBUG, "Completed process");
                            }
                            Err(err) => {
                                event!(Level::ERROR, "Error waiting for process {err}");
                            }
                        }
                   }
                   _ = cancel_source => {
                        event!(Level::TRACE, "Task is being canclled");
                   }
                }

                if tc.is_enabled("copy_previous") {
                    tc.copy_previous();
                }

                if tc.is_enabled("cache_output") { 
                    match reader_task.await {
                        Ok(output) => {
                            tc.with_text("output", output);
                        },
                        Err(err) => {
                            event!(Level::ERROR, "Error getting output, {err}");
                        },
                    }
                } else {
                    reader_task.abort();
                }

                stderr_reader_task.abort();
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
            .optional("redirect")
    }

    fn documentation(&self, property: impl AsRef<str>) -> Option<reality::Documentation> {
        let documentation = match property.as_ref() {
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
