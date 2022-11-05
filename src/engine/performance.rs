use specs::{Component, DenseVecStorage, Entity};
use super::Connection;

pub mod wire;

/// Component for a sample of performance related data,
///
#[derive(Component, Debug, Clone)]
#[storage(DenseVecStorage)]
pub struct Performance {
    /// Resolution to use when bucketing the histogram,
    ///
    pub bucket_ms: u64,
    /// Buckets,
    ///
    pub buckets: Vec<f32>,
    /// Percentiles measured,
    /// 
    /// A percentile is the percent of samples that are at or below a given threshold,
    /// 
    pub percentiles: Vec<(u64, u64)>,
    /// Total samples found in the histogram,
    /// 
    pub total_samples: u64,
    /// Performance is measured by transitions between events,
    ///
    /// The entity that scheduled the activity,
    ///
    pub from: Entity,
    /// The entity that measured the performance of the activity,
    ///
    pub to: Entity,
}

impl Performance {
    /// Samples a connection and returns a vector of Performance samples,
    ///
    pub fn samples(bucket_ms: u64, percentiles: &[f64], connection: &Connection) -> Vec<Self> {
        let mut samples = vec![];
        for (incoming, histogram) in connection
            .performance()
            .filter(|(_, h)| !h.is_empty() && h.len() > 1)
        {
            let from = incoming;
            let to = connection.entity();

            let get_buckets = |b| {
                histogram
                    .iter_linear(b)
                    .map(|h| h.percentile() as f32)
                    .collect::<Vec<_>>()
            };

            let get_percentile = |p| {
                let percentile_value = histogram.value_at_percentile(p);
                let percentile = histogram.percentile_below(percentile_value) as u64;
                (percentile, percentile_value)
            };

            let buckets = get_buckets(bucket_ms);

            samples.push(Self {
                bucket_ms,
                buckets,
                total_samples: histogram.len(),
                percentiles: {
                    percentiles.iter().map(|p| get_percentile(*p)).collect()
                },
                from: *from,
                to: to,
            });
        }

        samples
    }
}
