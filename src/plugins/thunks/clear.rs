use crate::plugins::Plugin;

use super::ThunkContext;

#[derive(Default)]
pub struct Clear;

impl Plugin<ThunkContext> for Clear {
    fn symbol() -> &'static str {
        "clear"
    }

    fn description() -> &'static str {
        "Clears any previous messages."
    }

    fn call_with_context(context: &mut ThunkContext) -> Option<crate::plugins::AsyncContext> {
        context.clone().task(|_| {
            async {
                None 
            }
        })
    }
}