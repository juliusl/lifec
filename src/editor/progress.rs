use atlier::system::{App, Extension};
use specs::storage::HashMapStorage;
use specs::{Component, WorldExt};
use std::cmp::min;
use std::fmt::Write;

use crate::plugins::{EventRuntime, StatusUpdate};
use crate::AttributeGraph;

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

impl Into<AttributeGraph> for ProgressStatusBar {
    fn into(self) -> AttributeGraph {
        let Self(progress, status, log_display, log_history) = self;

        AttributeGraph::from(0)
            .with_float("progress", progress)
            .with_text("status", status)
            .with_binary("log_display", log_display)
            .with_binary("history.log", log_history)
            .to_owned()
    }
}

impl From<AttributeGraph> for ProgressStatusBar {
    fn from(graph: AttributeGraph) -> Self {
        Self(
            {
                graph.find_float("progress")
                    .unwrap_or_default()
            }, 
            {
                graph.find_text("status")
                    .unwrap_or_default()
            }, 
            {
                graph.find_binary("log_display")
                    .and_then(|b| String::from_utf8(b).ok())
                    .unwrap_or_default()
            }, 
            {
                graph.find_binary("history.log")
                    .and_then(|b| String::from_utf8(b).ok())
                    .unwrap_or_default()
            })
    }
}

impl App for ProgressStatusBar {
    fn name() -> &'static str {
        "progress_status_bar"
    }

    fn edit_ui(&mut self, _: &imgui::Ui) {}

    fn display_ui(&self, ui: &imgui::Ui) {
        let ProgressStatusBar(progress, status, log_display, log_full) = self;

        if *progress > 0.0 {
            imgui::ProgressBar::new(*progress)
                .overlay_text(format!("{:.4} %", progress * 100.0))
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
}

impl Extension for ProgressStatusBar {
    fn configure_app_world(world: &mut specs::World) {
        world.register::<ProgressStatusBar>();
    }

    fn configure_app_systems(dispatcher: &mut specs::DispatcherBuilder) {
        dispatcher.add(
            EventRuntime::default(),
            "progress_status_bar/event_runtime",
            &[],
        );
    }

    fn on_ui(&'_ mut self, app_world: &specs::World, ui: &'_ imgui::Ui<'_>) {
        self.display_ui(ui);
        self.edit_ui(ui);
        self.on_run(app_world);
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
                writeln!(log_display, "{}", &s).ok();
                writeln!(log, "{}", &s).ok();
                *status = s;
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
