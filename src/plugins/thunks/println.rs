use crate::plugins::Plugin;

use super::ThunkContext;
use atlier::prelude::Value;

pub struct Println;

impl Plugin<ThunkContext> for Println {
    fn symbol() -> &'static str {
        "println"
    }

    fn description() -> &'static str {
        "Can be used to debug attributes passed as input to this thunk."
    }

    fn call_with_context(context: &mut ThunkContext) {
        context
            .as_ref()
            .iter_attributes()
            .map(|a| (a.name(), a.value()))
            .for_each(|(name, value)| {
                println!("{}: {}", name, value);
            });

        context.set_return::<Println>("", Value::Bool(true));
    }
}
