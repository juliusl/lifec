use std::time::Instant;

use crate::plugins::*;
use logos::{Lexer, Logos};
use tokio::task::JoinHandle;

use super::thunks::CancelToken;

/// Timer plugin,
///
#[derive(Default, Clone)]
pub struct Timer;

impl Plugin for Timer {
    fn symbol() -> &'static str {
        "timer"
    }

    fn description() -> &'static str {
        "Create a timer w/ a duration of seconds."
    }

    fn call(thunk_context: &mut ThunkContext) -> Option<(JoinHandle<ThunkContext>, CancelToken)> {
        thunk_context.clone().task(|mut cancel_source| {
            let mut tc = thunk_context.clone();
            async move {
                // Parse the input to timer
                let duration = tc
                    .state()
                    .find_symbol("timer")
                    .and_then(|d| match TimerSettings::lexer(&d).next() {
                        Some(TimerSettings::Duration(duration)) => Some(duration),
                        _ => Some(0.0),
                    })
                    .unwrap_or_default();

                // let duration = Duration::from_secs_f32(duration);
                // tokio::time::sleep(duration).await;

                let start = Instant::now();
                loop {
                    let elapsed = start.elapsed();
                    let progress = elapsed.as_secs_f32() / duration;
                    if progress < 1.0 {
                        if tc.is_enabled("quiet") {
                            tc.progress("", progress).await;
                        } else {
                            tc.progress(
                                format!("elapsed {} ms", elapsed.as_millis()),
                                progress,
                            )
                            .await;
                        }
                    } else {
                        tc.state_mut()
                            .with_symbol("elapsed", format!("{:?}", elapsed));

                        tc.progress("Completed", 1.0).await;
                        break;
                    }

                    if ThunkContext::is_cancelled(&mut cancel_source) {
                        tc.progress("Cancelling", progress).await;
                        break;
                    }
                }

                Some(tc)
            }
        })
    }
}

impl BlockObject for Timer {
    fn query(&self) -> BlockProperties {
        BlockProperties::default().require("timer")
    }

    fn parser(&self) -> Option<CustomAttribute> {
        Some(Timer::as_custom_attr())
    }
}

/// Enumeration of timer settings
///
#[derive(Logos, Debug, PartialEq)]
pub enum TimerSettings {
    /// Duration to wait, The value is always in seconds
    #[regex("[0-9]*", on_duration)]
    Duration(f32),
    #[token("s")]
    #[token("secs")]
    Seconds,
    #[token("ms")]
    Miliseconds,
    #[error]
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

fn on_duration(lexer: &mut Lexer<TimerSettings>) -> Option<f32> {
    let duration = lexer.slice().parse::<f32>();

    match duration {
        Ok(duration) => match lexer.next() {
            Some(token) => match token {
                TimerSettings::Duration(_) | TimerSettings::Seconds | TimerSettings::Error => {
                    Some(duration)
                }
                TimerSettings::Miliseconds => Some(duration / 1000.0),
            },
            None => Some(duration),
        },
        Err(err) => {
            event!(Level::ERROR, "could no parse timer settings {err}");
            None
        }
    }
}

#[test]
fn test_timer_settings() {
    let mut lexer = TimerSettings::lexer("100 ms");
    assert_eq!(lexer.next(), Some(TimerSettings::Duration(100.0 / 1000.0)));

    let mut lexer = TimerSettings::lexer("100ms");
    assert_eq!(lexer.next(), Some(TimerSettings::Duration(100.0 / 1000.0)));
}
