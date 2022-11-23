use std::collections::HashSet;

use specs::Entity;
use tokio::time::Instant;

/// Resource to control tick rate of event processing,
///
/// TODO: Actually allow for configuring tick rate
///
#[derive(Default)]
pub struct TickControl {
    pause: bool,
    last_tick: Option<Instant>,
    freq: u64,
    /// Limit the freq of the tick rate,
    /// 
    freq_limit: Option<u64>,
    /// When event status is scanned, this set will be checked to see if an entity is paused
    /// 
    paused: HashSet<Entity>,
}

impl TickControl {
    /// Sets pause to true,
    ///
    pub fn pause(&mut self) {
        self.pause = true;
    }

    /// Resumes any paused entities and runtime,
    ///
    pub fn resume(&mut self) {
        self.pause = false;
        self.paused.clear();
    }

    /// Pause a specific entity, returns true if added to paused set,
    /// 
    pub fn pause_entity(&mut self, entity: Entity) -> bool {
        self.paused.insert(entity)
    }

    /// Resumes a paused entity, returns true if removed from paused set,
    /// 
    pub fn resume_entity(&mut self, entity: Entity) -> bool {
        self.paused.remove(&entity)
    }

    /// Returns true if an entity is paused,
    /// 
    pub fn is_paused(&self, entity: Entity) -> bool {
        self.paused.contains(&entity)
    }

    /// Returns true if the event runtime can tick,
    ///
    pub fn can_tick(&self) -> bool {
        !self.pause && if let Some(rate_limit) = self.freq_limit {
            self.freq < rate_limit
        } else {
            true
        }
    }

    /// If set, returns the rate limit frequency
    /// 
    pub fn rate_limit(&self) -> Option<u64> {
        self.freq_limit
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

    /// Update the freq wrt to the rate limit,
    /// 
    pub fn update_rate_limit(&mut self) {
        if let Some(last) = self.last_tick {
            let elapsed = last.elapsed().as_secs_f64();
            if elapsed > 0.0 {
                let freq = 1.0 / elapsed; 
                self.freq = freq as u64;
            }
        }
    }

    /// Set a rate limit in units of Hz,
    /// 
    pub fn set_rate_limit(&mut self, limit: u64) {
        self.freq_limit = Some(limit);
    }

    /// Removes the rate limit,
    /// 
    pub fn remove_rate_limit(&mut self) {
        self.freq_limit.take();
    }

    /// Returns tick rate in units of freq (hz)
    /// 
    pub fn tick_rate(&self) -> u64 {
        self.freq
    }
}
