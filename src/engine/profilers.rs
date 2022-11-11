use specs::prelude::*;
use specs::{Entities, SystemData};

use super::{Performance, Profiler, Connection};

/// System data for profilers/performance related data,
///
#[derive(SystemData)]
pub struct Profilers<'a> {
    /// Lazy updates,
    /// 
    lazy_updates: Read<'a, LazyUpdate>,
    /// Entities
    /// 
    entities: Entities<'a>,
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
                self.lazy_updates
                    .insert(self.entities.create(), sample);
            }
        }
    }
}
