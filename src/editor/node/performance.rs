use std::sync::Arc;

use imgui::Ui;
use reality::wire::Protocol;

use crate::{
    engine::Performance,
    prelude::{Appendix, HostEditor},
};

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
            for performance in Performance::samples(bucket_ms, percentiles, connection) {
                render_performance(performance, &self.appendix, ui);
                drawn = true;
            }
        }
        drawn
    }
}

impl Profiler for HostEditor {
    fn histogram(&self, ui: &Ui, _: u64, _: &[f64]) -> bool {
        let mut drawn = false;
         
        if self.has_remote() {
            if let Some(appendix) = self.appendix() {
                if let Some(performance_data) = self.performance_data() {
                    for performance in performance_data {
                        render_performance(performance.clone(), &appendix, ui);
                        drawn = true;
                    }
                }
            }
        }

        drawn
    }
}

impl Profiler for Protocol {
    fn histogram(&self, ui: &Ui, _: u64, _: &[f64]) -> bool {
        let appendix = self.as_ref().fetch::<Arc<Appendix>>();
        let mut drawn = false;
        for performance in self.decode::<Performance>() {
            render_performance(performance, &appendix, ui);
            drawn = true;
        }
        drawn
    }
}

fn render_performance(
    Performance {
        bucket_ms,
        buckets,
        percentiles,
        total_samples,
        from,
        to,
    }: Performance,
    appendix: &Appendix,
    ui: &Ui,
) {
    let show_percentile = |percentile, percentile_value| {
        ui.text(format!("~ {:3}% <= {:6} ms", percentile, percentile_value));
        ui.spacing();
    };

    imgui::PlotHistogram::new(
        ui,
        format!(
            "{} -> {}",
            appendix
                .name(&from)
                .unwrap_or(format!("{}", from.id()).as_str()),
            appendix
                .name(&to)
                .unwrap_or(format!("{}", to.id()).as_str()),
        ),
        buckets.as_slice(),
    )
    .overlay_text(format!("Performance buckets ({} ms)", bucket_ms))
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
}
