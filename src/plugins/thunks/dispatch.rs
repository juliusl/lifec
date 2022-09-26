use crate::{plugins::Plugin, AttributeIndex};

use super::ThunkContext;

#[derive(Default)]
pub struct Dispatch;

impl Plugin for Dispatch {
    fn symbol() -> &'static str {
        "dispatch"
    }

    fn description() -> &'static str {
        "Dispatches the text attribute `content`"
    }

    fn call(context: &ThunkContext) -> Option<crate::plugins::AsyncContext> {
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