use specs::prelude::*;
use specs::SystemData;

use super::{Connection, Performance, Profiler};

/// System data for profilers/performance related data,
///
#[derive(SystemData)]
pub struct Profilers<'a> {
    /// Lazy updates,
    ///
    lazy_updates: Read<'a, LazyUpdate>,
    /// Connections
    ///
    connections: ReadStorage<'a, Connection>,
    /// Profilers
    ///
    profilers: ReadStorage<'a, Profiler>,
}

impl<'a> Profilers<'a> {
    /// Collect profiling data, results are stored as entities
    ///
    pub fn profile(&self) {
        for connection in self.connections.join() {
            let profiler = self
                .profilers
                .get(connection.entity())
                .cloned()
                .unwrap_or_default();

            let samples = Performance::samples(
                profiler.bucket_ms,
                profiler.percentiles.as_slice(),
                connection,
            );

            for sample in samples {
                self.lazy_updates.insert(sample.from, sample);
            }
        }
    }
}
