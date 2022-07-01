use std::time::Instant;

use specs::storage::DenseVecStorage;
use tokio::task::JoinHandle;
use crate::plugins::*;

#[derive(Default, Component, Clone)]
#[storage(DenseVecStorage)]
pub struct Timer;

impl Plugin<ThunkContext> for Timer {
    fn symbol() -> &'static str {
        "timer"
    }

    fn description() -> &'static str {
        "Create a timer w/ a duration of seconds."
    }

    fn call_with_context(thunk_context: &mut ThunkContext) -> Option<JoinHandle<ThunkContext>> {
        thunk_context.clone().task(|| {
            let mut tc = thunk_context.clone();
            async move {
                let mut duration = 0.0;
                if let Some(d) = tc.as_ref().find_int("duration") {
                    tc.update_status_only("duration found").await;
                    duration += d as f32;
                }

                if let Some(d_ms) = tc.as_ref().find_float("duration_ms") {
                    tc.update_status_only("duration_ms found").await;
                    duration += d_ms/1000.0;
                }

                let start = Instant::now();
                loop {
                    let elapsed = start.elapsed();
                    let progress =
                        elapsed.as_secs_f32() / duration;
                    if progress < 1.0 {
                        tc.update_progress(format!("elapsed {} ms", elapsed.as_millis()), progress)
                            .await;
                    } else {
                        tc.as_mut()
                            .add_text_attr("elapsed", format!("{:?}", elapsed));
                        break;
                    }
                }

                Some(tc)
            }
        })
    }
}