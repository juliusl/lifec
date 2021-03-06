use std::time::Instant;

use crate::plugins::*;
use specs::storage::DenseVecStorage;
use tokio::task::JoinHandle;

use super::CancelToken;

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

    fn call_with_context(
        thunk_context: &mut ThunkContext,
    ) -> Option<(JoinHandle<ThunkContext>, CancelToken)> {
        thunk_context.clone().task(|mut cancel_source| {
            let mut tc = thunk_context.clone();
            async move {
                let mut duration = 0.0;
                if let Some(d) = tc.as_ref().find_int("duration") {
                    tc.update_status_only("duration found").await;
                    duration += d as f32;
                }

                if let Some(d_ms) = tc.as_ref().find_float("duration_ms") {
                    tc.update_status_only("duration_ms found").await;
                    duration += d_ms / 1000.0;
                }

                let start = Instant::now();

                loop {
                    let elapsed = start.elapsed();
                    let progress = elapsed.as_secs_f32() / duration;
                    if progress < 1.0 {
                        
                        if tc.as_ref().is_enabled("quiet").unwrap_or_default() {
                            tc.update_progress("", progress).await;
                        } else {
                            tc.update_progress(format!("elapsed {} ms", elapsed.as_millis()), progress).await;
                        }
                    } else {
                        tc.as_mut()
                            .add_text_attr("elapsed", format!("{:?}", elapsed));
                        break;
                    }

                    if ThunkContext::is_cancelled(&mut cancel_source) {
                        break;
                    }
                }

                Some(tc)
            }
        })
    }
}
