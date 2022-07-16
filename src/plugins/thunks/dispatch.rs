use crate::plugins::Plugin;

use super::ThunkContext;

#[derive(Default)]
pub struct Dispatch;

impl Plugin<ThunkContext> for Dispatch {
    fn symbol() -> &'static str {
        "dispatch"
    }

    fn description() -> &'static str {
        "Dispatches the text attribute `content`"
    }

    fn call_with_context(context: &mut ThunkContext) -> Option<crate::plugins::AsyncContext> {
        context.clone().task(|_| {
            let tc = context.clone();
            async move {
                if let Some(message) = tc.as_ref().find_text("content") {
                    tc.dispatch(message).await;
                }

                None 
            }
        })
    }
}