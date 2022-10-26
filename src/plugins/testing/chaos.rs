use reality::{BlockObject, BlockProperties};

use crate::{
    prelude::{Plugin, Timer},
    state::AttributeIndex,
};

#[derive(Default)]
pub struct Chaos;

impl Plugin for Chaos {
    fn symbol() -> &'static str {
        "chaos"
    }

    fn description() -> &'static str {
        "Generates chaotic performance"
    }

    fn call(context: &crate::prelude::ThunkContext) -> Option<crate::prelude::AsyncContext> {
        let mut tc = context.clone();
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let delay = rng.gen::<u64>() % 1000;

        tc.state_mut().with_symbol("timer", format!("{delay} ms"));

        Timer::call(&tc)
    }
}

impl BlockObject for Chaos {
    fn query(&self) -> reality::BlockProperties {
        BlockProperties::default()
    }

    fn parser(&self) -> Option<reality::CustomAttribute> {
        Some(Self::as_custom_attr())
    }
}