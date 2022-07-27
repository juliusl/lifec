use std::path::PathBuf;

use crate::plugins::{ThunkContext, Plugin};

#[derive(Default)]
pub struct Redirect; 

impl Plugin<ThunkContext> for Redirect {
    fn symbol() -> &'static str {
        "redirect"
    }

    fn description() -> &'static str {
        "Redirect stdout to the path specified by `redirect_stdout`, and redirect stderr to the path specified by `redirect_stderr`"
    }

    fn call_with_context(context: &mut ThunkContext) -> Option<crate::plugins::AsyncContext> {
        context.clone().task(|_| {
            let mut tc = context.clone(); 
            async move {
                tc.as_mut().apply("previous");

                for process_block in tc.as_ref().find_blocks("process") {
                    if let Some(mut redirect_stdout) = tc.as_ref().find_text("redirect_stdout") {
                        if let Some(stdout) = process_block.find_binary("stdout") {

                            if let Some(work_dir) = tc.as_ref().find_text("work_dir") {
                                let work_dir = PathBuf::from(work_dir);
                                if work_dir.exists() {
                                    let redirect = work_dir.join(&redirect_stdout);
                                    redirect_stdout = redirect.to_str().unwrap_or_default().to_string();
                                }
                            }

                            match tokio::fs::write(redirect_stdout, stdout).await {
                                Ok(_) => {
                                    
                                },
                                Err(err) => {
                                    eprintln!("error redirecting stdout {err}");
                                },
                            }
                        }
                    }

                    if let Some(mut redirect_stderr) =  tc.as_ref().find_text("redirect_stderr") {
                        if let Some(stderr) = process_block.find_binary("stderr") {
                            
                            if let Some(work_dir) = tc.as_ref().find_text("work_dir") {
                                let work_dir = PathBuf::from(work_dir);
                                if work_dir.exists() {
                                    let redirect = work_dir.join(&redirect_stderr);
                                    redirect_stderr = redirect.to_str().unwrap_or_default().to_string();
                                }
                            }

                            match tokio::fs::write(redirect_stderr, stderr).await {
                                Ok(_) => {
                                    
                                },
                                Err(err) => {
                                    eprintln!("error redirecting stderr {err}");
                                },
                            }
                        }
                    }
                }

                None
            }
        })
    }
}