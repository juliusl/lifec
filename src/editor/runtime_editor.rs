use imgui::{CollapsingHeader, Window};
use specs::{Component, Entities, System};
use std::time::Instant;

use super::App;
use crate::{Runtime, RuntimeState};

#[derive(Clone)]
pub struct RuntimeEditor<S>
where
    S: RuntimeState,
{
    pub runtime: Runtime<S>,
    pub running: (Option<bool>, Option<Instant>, Option<Instant>),
    pub dispatch_snapshot: Option<()>,
    pub dispatch_remove: Option<u32>,
}

impl<S> RuntimeEditor<S>
where
    S: RuntimeState,
{
    pub fn new(runtime: Runtime<S>) -> Self {
        Self {
            runtime,
            running: (None, None, None),
            dispatch_remove: None,
            dispatch_snapshot: None,
        }
    }
}

impl<'a, S> System<'a> for RuntimeEditor<S>
where
    S: RuntimeState + Component,
{
    type SystemData = (Entities<'a>,);

    /// The runtime editor maintains a vector of sections that it displays
    /// This system coordinates updates to those sections, as well as initialization
    fn run(&mut self, _: Self::SystemData) {}
}

impl<S> Default for RuntimeEditor<S>
where
    S: RuntimeState + Component,
{
    fn default() -> Self {
        Self {
            runtime: Default::default(),
            running: (None, None, None),
            dispatch_snapshot: None,
            dispatch_remove: None,
        }
    }
}

impl<S> App for RuntimeEditor<S>
where
    S: RuntimeState + Component,
{
    fn name() -> &'static str {
        "Runtime Editor"
    }

    fn window_size() -> &'static [f32; 2] {
        &[1500.0, 720.0]
    }

    fn show_editor(&mut self, ui: &imgui::Ui) {
        Window::new(Self::name())
            .size(*Self::window_size(), imgui::Condition::Appearing)
            .menu_bar(true)
            .build(ui, || {
                if let Some(state) = &mut self.runtime.state {
                    let graph = state.dispatcher_mut().as_mut();
                    ui.menu_bar(|| {
                        graph.edit_attr_menu(ui);
                    });
                    
                    graph.edit_attr_table(ui);
                }
            });
    }
}

impl<S> RuntimeEditor<S>
where
    S: RuntimeState + Component,
{
    pub fn show_current(&mut self, ui: &imgui::Ui) {
        match self.running {
            (Some(v), elapsed, stopped) => {
                if ui.button("Stop") {
                    self.dispatch_remove = None;
                    self.dispatch_snapshot = None;
                    self.running = (None, None, None);
                }

                match (v, elapsed, stopped) {
                    (true, Some(elapsed), None) => {
                        ui.same_line();
                        if ui.button("Pause") {
                            self.running = (Some(false), Some(elapsed), Some(Instant::now()));
                        }

                        ui.text(format!("Running {:#?}", elapsed.elapsed()));

                        if self.runtime.can_continue() {
                            self.runtime = self.runtime.step();
                        } else {
                            self.running = (None, Some(elapsed), Some(Instant::now()));
                        }
                    }
                    (false, Some(elapsed), Some(stopped)) => {
                        ui.same_line();
                        if ui.button("Continue") {
                            self.running = (Some(true), Some(elapsed), None);
                        }

                        ui.text(format!("Paused {:#?}", stopped.elapsed()));
                    }
                    _ => {}
                }
            }
            (None, Some(elapsed), Some(stopped)) => {
                if ui.button("Clear") {
                    self.dispatch_remove = None;
                    self.dispatch_snapshot = None;
                    self.running = (None, None, None);
                }
                ui.text(format!("Ran for {:#?}", stopped - elapsed));
            }
            _ => {}
        };

        let context = self.runtime.context();
        ui.label_text(format!("Current Event"), format!("{}", context));
        ui.disabled(self.running.0.is_some(), || {
            if ui.button("Setup") {
                self.runtime.parse_event("{ setup;; }");
            }
            ui.same_line();
            if ui.button("Start") {
                self.running = (Some(true), Some(Instant::now()), None);
            }

            ui.same_line();
            if ui.button("Step") {
                self.runtime = self.runtime.step();
            }

            ui.same_line();
            if ui.button("Clear Attributes") {
                self.runtime.attributes.clear_index();
            }
        });
        ui.new_line();
        ui.separator();
        if let Some(state) = self.runtime.current() {
            ui.input_text_multiline(
                format!("Current State"),
                &mut format!("{}", state),
                [0.0, 0.0],
            )
            .read_only(true)
            .build();
            ui.new_line();
        }

        if !self.runtime.attributes.is_index_empty() {
            if CollapsingHeader::new(format!("Runtime Attributes"))
                .leaf(true)
                .build(ui)
            {
                self.runtime.attributes.iter_attributes().for_each(|a| {
                    ui.text(format!("{}", a));
                });
            }
        }
    }
}
