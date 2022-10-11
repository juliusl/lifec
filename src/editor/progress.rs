use atlier::system::{App, Extension};
use specs::storage::HashMapStorage;
use specs::{Component, WorldExt};
use std::cmp::min;
use std::fmt::Write;

use crate::plugins::{EventRuntime, StatusUpdate};
use crate::{AttributeGraph, AttributeIndex};

#[derive(Component, Clone, Default)]
#[storage(HashMapStorage)]
pub struct ProgressStatusBar(
    /// progress
    pub f32, 
    /// status
    pub String,
    /// log_display
    pub String, 
    /// history.log
    /// TODO: change to binary vec
    pub String
);

impl Extension for ProgressStatusBar {
    fn configure_app_world(world: &mut specs::World) {
        world.register::<ProgressStatusBar>();
    }

    fn on_ui(&'_ mut self, _: &specs::World, ui: &'_ imgui::Ui<'_>) {
        let ProgressStatusBar(progress, status, log_display, log_full) = self;

        if *progress > 0.0 {
            imgui::ProgressBar::new(*progress)
                .overlay_text(format!("{:.4} %", *progress * 100.0))
                .build(ui);
        }

        if !status.is_empty() {
            ui.text(format!("{}", status));

            let log_tool = || {
                if !log_display.is_empty() {
                    ui.text("Output log (Right+Click to see more)");
                    let width = (log_display
                        .split_once("\n")
                        .and_then(|(s, _)| Some(s.len()))
                        .unwrap_or_default() as f64
                        * 16.0)
                        .min(1360.0);
                    ui.input_text_multiline(
                        "output_log",
                        &mut log_display.clone(),
                        [width as f32, 160.0],
                    )
                    .read_only(true)
                    .build();
                }
            };

            if ui.is_item_hovered() {
                ui.tooltip(log_tool);
            }

            ui.popup(&log_full, || {
                ui.text("Log history");
                if ui.button("dump to console out") {
                    println!("{}", &log_full);
                }
                ui.input_text_multiline("output_log", &mut log_full.clone(), [1360.0, 35.0 * 16.0])
                    .read_only(true)
                    .build();
            });

            if ui.is_item_clicked_with_button(imgui::MouseButton::Right) {
                ui.open_popup(&log_full);
            }
        }
    }

    fn on_run(&'_ mut self, app_world: &specs::World) {
        let mut rx = app_world.write_resource::<tokio::sync::mpsc::Receiver<StatusUpdate>>();
        let mut progress = app_world.write_storage::<ProgressStatusBar>();

        if let Some((entity, p, s)) = rx.try_recv().ok() {
            if let Some(ProgressStatusBar(progress, status, log_display, log)) =
                progress.get_mut(entity)
            {
                *progress = p;
                // The idea here is to show at max 10 lines with 85 chars on each line
                // this gets dynamically sized so that small log messages get more that 10 lines automatically
                let limit = 10 * min(85, status.len());
                if log_display.len() > limit {
                    if let Some((_, remaining)) = log_display.split_once("\n") {
                        *log_display = remaining.to_string();
                    }
                }
                if !s.is_empty() {
                    writeln!(log_display, "{}", &s).ok();

                    // TODO need to have caps on log-size, or compressor
                    writeln!(log, "{}", &s).ok();
                    *status = s;
                }
            } else {
                match progress.insert(
                    entity,
                    ProgressStatusBar(p, s, String::default(), String::default()),
                ) {
                    Ok(_) => {}
                    Err(_) => {}
                }
            }
        }
    }
}
