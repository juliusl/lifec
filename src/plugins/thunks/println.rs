use crate::plugins::Plugin;

use super::ThunkContext;

#[derive(Default)]
pub struct Println;

impl Plugin<ThunkContext> for Println {
    fn symbol() -> &'static str {
        "println"
    }

    fn call_with_context(context: &mut ThunkContext) -> Option<crate::plugins::AsyncContext> {
        context.clone().task(|_| {
            let mut tc = context.clone();
            async move {
                tc.as_mut().apply("previous");
                Some(tc) 
            }
        })
    }
}