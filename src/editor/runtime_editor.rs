use imgui::{ChildWindow, CollapsingHeader, Window};
use knot::store::Store;
use specs::{
    storage::DefaultVecStorage, Component, Entities, Join, Read,
    ReadStorage, System, Write, WriteStorage,
};
use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
    time::Instant,
};

use super::{
     unique_title, App, Value,
};
use crate::{Action, Runtime, RuntimeState, AttributeGraph};

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
    S: RuntimeState {
        pub fn new(runtime: Runtime<S>) -> Self {
            Self { runtime, running: (None, None, None), dispatch_remove: None, dispatch_snapshot: None }
        }
    }

impl<'a, S> System<'a> for RuntimeEditor<S>
where
    S: RuntimeState + Component,
{
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Loader>,
        Write<'a, RuntimeEditor<S>>,
        Write<'a, Dispatch>,
    );

    /// The runtime editor maintains a vector of sections that it displays
    /// This system coordinates updates to those sections, as well as initialization
    fn run(
        &mut self,
        (entities,  mut loader, mut runtime_editor, mut dispatcher): Self::SystemData,
    ) {


      
        let runtime_editor = runtime_editor.deref_mut();
        *runtime_editor = self.clone();
    }
}

#[derive(Default)]
pub struct RuntimeDispatcher<S>
where
    S: RuntimeState + Component,
{
    runtime: Option<RuntimeEditor<S>>,
}

impl<S> From<RuntimeEditor<S>> for RuntimeDispatcher<S>
where
    S: RuntimeState + Component,
{
    fn from(runtime: RuntimeEditor<S>) -> Self {
        Self {
            runtime: Some(runtime),
        }
    }
}

pub enum Dispatch {
    Empty,
    RemoveSnapshot(u32),
}

#[derive(Component)]
#[storage(DefaultVecStorage)]
pub enum Loader {
    Empty,
    LoadSection(AttributeGraph),
}

impl Default for Loader {
    fn default() -> Self {
        Loader::Empty
    }
}

impl Default for Dispatch {
    fn default() -> Self {
        Self::Empty
    }
}

impl<'a, S> System<'a> for RuntimeDispatcher<S>
where
    S: RuntimeState + Component,
{
    type SystemData = (
        Entities<'a>,
        Read<'a, RuntimeEditor<S>>,
        Write<'a, Dispatch>,
        WriteStorage<'a, Loader>,
        WriteStorage<'a, AttributeGraph>,
    );

    fn run(
        &mut self,
        (
            entities,
            runtime_editor,
            mut msg,
            mut loader,
            mut section_attributes,
        ): Self::SystemData,
    ) {
        if let Dispatch::RemoveSnapshot(id) = msg.deref() {
            let to_remove = entities.entity(*id);
            match entities.delete(to_remove) {
                Ok(_) => {
                    let msg = msg.deref_mut();
                    *msg = Dispatch::Empty;
                    section_attributes.remove(to_remove);
                    return;
                }
                Err(err) => eprintln!("RuntimeDispatcher Error {}", err),
            }
        }

        for entity in entities.join() {
            if let Some(Loader::LoadSection(attributes)) = loader.get(entity) {
                println!("Load section for {:?}", entity);

                if let Some(v) = loader.get_mut(entity) {
                    *v = Loader::Empty;
                }
                return;
            }
        }

     
    }
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
            .build(ui, || {
                ChildWindow::new("Sections").always_use_window_padding(true).size([1000.0, 0.0]).build(ui, || {
                    if CollapsingHeader::new("Snapshots").leaf(true).begin(ui) {
                        if ui.button("Take Snapshot of Runtime") {
                            self.dispatch_snapshot = Some(());
                            return;
                        }
                        ui.new_line();
                    }
                });
                ui.same_line();
                ChildWindow::new("Runtime").always_use_window_padding(true).size([0.0, 0.0]).build(ui, || {
                    if CollapsingHeader::new(format!("Current Runtime"))
                        .leaf(true)
                        .begin(ui)
                    {
                        ui.separator();
                        self.show_current(ui);
                    }
                    ui.new_line();
                });
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
