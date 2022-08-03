use tracing::{event, Level};

use crate::plugins::{Plugin, ThunkContext, Println, combine, Timer};

use super::Remote;

#[derive(Default)]
pub struct Missing; 

impl Plugin<ThunkContext> for Missing {
    fn symbol() -> &'static str {
        "missing"
    }

    fn description() -> &'static str {
        "Calls a command for a missing problem"
    }

    fn caveats() -> &'static str {
        "Dispatched by engines w/ `fix` event"
    }

    fn call_with_context(context: &mut ThunkContext) -> Option<crate::plugins::AsyncContext> {
        // TODO can probably move this to process
        if let Some(required_os) = context.as_ref().find_text("required_os") {
            let current_os = std::env::consts::OS;
            if required_os != current_os { 
                let log = format!("Cannot use fix, required OS {required_os}, current OS {current_os}");
                event!(Level::ERROR, "{log}");

                 return context.clone().task(|_| {
                    let tc = context.clone();
                    async move {
                        tc.update_status_only(log).await;
                        None
                    }
                });
            }
        }

        combine::<Timer, (Remote, Println)>()(context)
    }
}