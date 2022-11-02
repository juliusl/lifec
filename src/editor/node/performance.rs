use imgui::Ui;

use crate::engine::Performance;

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
            for Performance {
                buckets,
                percentiles,
                total_samples,
                from,
                to,
                ..
            } in Performance::samples(bucket_ms, percentiles, connection) {
                let show_percentile = |percentile, percentile_value| {
                    ui.text(format!("~ {:3}% <= {:6} ms", percentile, percentile_value));
                    ui.spacing();
                };

                imgui::PlotHistogram::new(
                    ui,
                    format!(
                        "{} -> {}",
                        self.appendix
                            .name(&from)
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
                for (p, pv) in percentiles {
                    show_percentile(p, pv);
                }
                group.end();
                // Adds more spacing between groups,
                ui.same_line();
                ui.spacing();
                ui.same_line();
                let group = ui.begin_group();
                ui.text(format!("total samples: {:10}", total_samples));
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
