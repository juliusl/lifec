use tracing::{event, Level};

use crate::plugins::Plugin;

use super::ThunkContext;

#[derive(Default)]
pub struct Println;

impl Plugin for Println {
    fn symbol() -> &'static str {
        "println"
    }

    fn call(context: &ThunkContext) -> Option<crate::plugins::AsyncContext> {
        context.clone().task(|_| {
            let mut tc = context.clone();
            async move {
                tc.as_mut().apply("previous");

                if tc.project.as_ref().and_then(|p| p.as_ref().is_enabled("debug")).unwrap_or_default() {
                    event!(Level::DEBUG, "Context -- \n{:#?}", tc.as_ref());
                    event!(Level::DEBUG, "Project -- \n{:#?}", tc.project.as_ref());
                }

                Some(tc) 
            }
        })
    }
}