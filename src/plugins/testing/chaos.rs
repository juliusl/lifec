use reality::{BlockObject, BlockProperties};

use crate::{
    prelude::{Plugin, Timer},
    state::AttributeIndex,
};

/// Plugin to simulate chaotic behavior,
/// 
#[derive(Default)]
pub struct Chaos;

impl Plugin for Chaos {
    fn symbol() -> &'static str {
        "chaos"
    }

    fn description() -> &'static str {
        "Generates chaotic performance"
    }

    fn call(context: &mut crate::prelude::ThunkContext) -> Option<crate::prelude::AsyncContext> {
        let mut tc = context.clone();
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let delay = rng.gen::<u64>() % 1000;

        tc.state_mut().with_symbol("timer", format!("{delay} ms"));

        Timer::call(&mut tc)
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