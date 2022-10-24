use std::time::Duration;

use specs::{Dispatcher, WorldExt};
use tokio::time::Instant;

use crate::prelude::*;

/// Async version of host fn's,
///
impl Host {
    /// Waits for the host systems to exit as an async fn,
    ///
    /// Includes a tick rate setting (ex for 60hz use a 16ms tick rate),
    /// as well as an optional start delay
    ///
    pub async fn async_wait_for_exit<'a, 'b>(
        &mut self,
        start: Option<Instant>,
        tick_rate: Duration,
        dispatcher: &mut Dispatcher<'a, 'b>,
    ) {
        let mut interval = if let Some(start) = start {
            tokio::time::interval_at(start, tick_rate)
        } else {
            tokio::time::interval(tick_rate)
        };

        while !self.should_exit() {
            dispatcher.dispatch(self.world());
            self.world_mut().maintain();

            interval.tick().await;
        }
    }
}
