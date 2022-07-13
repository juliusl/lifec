use crate::plugins::{Plugin, ThunkContext, combine, Println};

use super::Expect;

#[derive(Default)]
pub struct Check;

impl Plugin<ThunkContext> for Check {
    fn symbol() -> &'static str {
        "check"
    }

    fn description() -> &'static str {
        "Checks expectations, and reads the result"
    }

    fn call_with_context(context: &mut ThunkContext) -> Option<crate::plugins::AsyncContext> {
        combine::<Expect, Println>()(context)
    }
}