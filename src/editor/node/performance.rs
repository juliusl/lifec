use imgui::Ui;

use super::Node;

/// Extension of node to display performance information,
///
pub trait Profiler {
    /// Display a histogram of performance collected by the node,
    ///
    /// Returns true if a histogram was drawn
    ///
    fn histogram(&self, ui: &Ui, bucket_ms: u64, percentiles: &[f64]) -> bool;
}

impl Profiler for Node {
    fn histogram(&self, ui: &Ui, bucket_ms: u64, percentiles: &[f64]) -> bool {
        let mut drawn = false;
        if let Some(connection) = self.connection.as_ref() {
            for (incoming, histogram) in connection
                .performance()
                .filter(|(_, h)| !h.is_empty() && h.len() > 1)
            {
                // TODO: Add view-options

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

                let show_percentile = |p| {
                    let (percentile, percentile_value) = get_percentile(p);
                    ui.text(format!("~ {:3}% <= {:6} ms", percentile, percentile_value));
                    ui.spacing();
                };

                let buckets = get_buckets(bucket_ms);
                imgui::PlotHistogram::new(
                    ui,
                    format!(
                        "{} -> {}",
                        self.appendix
                            .name(from)
                            .unwrap_or(format!("{}", from.id()).as_str()),
                        self.appendix
                            .name(&to)
                            .unwrap_or(format!("{}", to.id()).as_str()),
                    ),
                    buckets.as_slice(),
                )
                .overlay_text("Performance buckets (100 ms)")
                .graph_size([0.0, 75.0])
                .build();

                ui.spacing();
                let group = ui.begin_group();
                for p in percentiles {
                    show_percentile(*p);
                }
                group.end();
                // Adds more spacing between groups, 
                ui.same_line();
                ui.spacing();
                ui.same_line();
                let group = ui.begin_group();
                ui.text(format!("total samples: {:10}", histogram.len()));
                ui.text(format!("# of buckets:  {:10}", buckets.len()));
                group.end();
                ui.new_line();
                ui.separator();
                drawn = true;
            }
        }
        drawn
    }
}
