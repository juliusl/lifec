use std::time::Duration;
use std::time::Instant;

use specs::storage::DenseVecStorage;
use tokio::task::JoinHandle;
use crate::plugins::*;
use crate::editor::*;

#[derive(Default, Component, Clone)]
#[storage(DenseVecStorage)]
pub struct Timer;

impl Plugin<ThunkContext> for Timer {
    fn symbol() -> &'static str {
        "timer"
    }

    fn call_with_context(thunk_context: &mut ThunkContext) -> Option<JoinHandle<ThunkContext>> {
        thunk_context.clone().task(|| {
            let mut tc = thunk_context.clone();
            async move {
                let mut duration = 5;
                if let Some(d) = tc.as_ref().find_int("duration") {
                    tc.update_progress("duration found", 0.0).await;
                    duration = d;
                }

                let start = Instant::now();
                let duration = duration as u64;
                loop {
                    let elapsed = start.elapsed();
                    let progress =
                        elapsed.as_secs_f32() / Duration::from_secs(duration).as_secs_f32();
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