use specs::{Component, HashMapStorage, System};

use super::Profilers;

/// Component to create a terminal point for adhoc events to point to,
///
/// This is also a "stateless" system for sampling activity from connections.
/// 
#[derive(Component, Clone)]
#[storage(HashMapStorage)]
pub struct Profiler {
    /// Size of the bucket to use,
    ///
    pub bucket_ms: u64,
    /// Percentiles to measure
    ///
    pub percentiles: Vec<f64>,
}

impl Default for Profiler {
    fn default() -> Self {
        Self {
            bucket_ms: 100,
            percentiles: vec![50.0, 75.0, 90.0, 99.9],
        }
    }
}

impl<'a> System<'a> for Profiler {
    type SystemData = Profilers<'a>;

    fn run(&mut self, profilers: Self::SystemData) {       
        profilers.profile();
    }
}
