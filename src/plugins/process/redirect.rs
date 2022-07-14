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
                    if let Some(redirect_stdout) = tc.as_ref().find_text("redirect_stdout") {
                        if let Some(stdout) = process_block.find_binary("stdout") {
                            match tokio::fs::write(redirect_stdout, stdout).await {
                                Ok(_) => {
                                    
                                },
                                Err(err) => {
                                    eprintln!("error redirecting stdout {err}");
                                },
                            }
                        }
                    }

                    if let Some(redirect_stderr) =  tc.as_ref().find_text("redirect_stderr") {
                        if let Some(stderr) = process_block.find_binary("stderr") {
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