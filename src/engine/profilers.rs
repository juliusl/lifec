use specs::prelude::*;
use specs::{Entities, SystemData, WriteStorage};

use super::{Performance, Profiler, Connection};

/// System data for profilers/performance related data,
///
#[derive(SystemData)]
pub struct Profilers<'a> {
    /// Entities
    /// 
    entities: Entities<'a>,
    /// Connections
    /// 
    connections: ReadStorage<'a, Connection>,
    /// Profilers,
    /// 
    profilers: WriteStorage<'a, Profiler>,
    /// Performance storage,
    /// 
    performance: WriteStorage<'a, Performance>,
}

impl<'a> Profilers<'a> {
    /// Collect profiling data, results are stored as entities
    /// 
    pub fn profile(&mut self) {
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
                self.performance
                    .insert(self.entities.create(), sample)
                    .expect("should be able to insert sample");
            }
        }
    }
}
