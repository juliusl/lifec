use specs::Join;

use super::{Performance, State};

impl<'a> State<'a> {
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
                self.samples
                    .insert(self.entities.create(), sample)
                    .expect("should be able to insert sample");
            }
        }
    }
}