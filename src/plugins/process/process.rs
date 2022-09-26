use atlier::system::Value;
use reality::{BlockObject, BlockProperties, CustomAttribute};
use specs::{Component, DenseVecStorage};
use tokio::{select, task::JoinHandle};

use crate::plugins::{thunks::CancelToken, Plugin, ThunkContext};

/// The process component executes a command and records the output
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

    fn customize(parser: &mut reality::AttributeParser) {
        parser.add_custom(CustomAttribute::new_with("env", |p, value| {
            let var_name = p.symbol().expect("Requires a var name").to_string();
            p.define("env", Value::Symbol(var_name.to_string()));
            p.define(var_name, Value::Symbol(value));
        }));
    }

    fn call(context: &super::ThunkContext) -> Option<(JoinHandle<ThunkContext>, CancelToken)> {
        let clone = context.clone();
        clone.clone().query::<Process>().task(|cancel_source| {
            let tc = context.clone();
            async move {
                let properties = tc.clone().block.properties.expect("properties must exist");

                let command = properties
                    .property("process").expect("process property required")
                    .symbol().unwrap();

                let mut command_task = tokio::process::Command::new(command);
                command_task.kill_on_drop(true);

                if let Some(env) = properties.property("env").and_then(|p| p.symbol_vec()) {
                    for (env_name, env_val) in env.iter().filter_map(|e| {
                        properties.property(e).and_then(|p| p.symbol()).and_then(|s| Some((e, s)))
                    }) {
                        command_task.env(env_name, env_val);
                    }
                }

                if let Some(work_dir) = properties.property("work_dir").and_then(|p| p.symbol()) {
                    command_task.current_dir(work_dir);
                }

                select! {
                   output = command_task.output() => {
                        match output {
                            Ok(output) => {
                                for b in output.stdout.clone() {
                                    tc.send_char(b).await;
                                }
                                for b in output.stderr.clone() {
                                    tc.send_char(b).await;
                                }
                                // Completed process, publish result
                                tc.update_progress("# Finished, recording output", 0.30).await;
                                // Self::resolve_output(&mut tc, command, start_time, output);
                                
                            }
                            Err(err) => {
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
    fn query(&self) -> reality::BlockProperties {
        BlockProperties::default()
            .require("process")
            .optional("work_dir")
            .optional("env")
    }

    fn parser(&self) -> Option<reality::CustomAttribute> {
        Some(Process::as_custom_attr())
    }
}
