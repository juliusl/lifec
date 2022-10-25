use tokio::time::Instant;

/// Resource to control tick rate of event processing,
///
/// TODO: Actually allow for tick rate
///
#[derive(Default)]
pub struct TickControl {
    pause: bool,
    last_tick: Option<Instant>,
    freq: u64,
}

impl TickControl {
    /// Sets pause to true,
    ///
    pub fn pause(&mut self) {
        self.pause = true;
    }

    /// Resets to defaults,
    ///
    pub fn reset(&mut self) {
        self.pause = false;
    }

    /// Returns true if the event runtime can tick,
    ///
    pub fn can_tick(&self) -> bool {
        !self.pause
    }
    
    /// Updates tick rate,
    /// 
    pub fn update_tick_rate(&mut self) {
        if let Some(last) = self.last_tick.replace(Instant::now()) {
            let elapsed = last.elapsed().as_secs_f64();
            if elapsed > 0.0 {
                let freq = 1.0 / elapsed; 
                self.freq = freq as u64;
            }
        }
    }

    /// Returns tick rate in units of freq (hz)
    /// 
    pub fn tick_rate(&self) -> u64 {
        self.freq
    }
}
