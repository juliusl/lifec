use std::cell::RefCell;

use super::{Plugin, Thunk};
use crate::AttributeGraph;
use imgui::Ui;
use specs::storage::DenseVecStorage;
use specs::{Component, Join, ReadStorage, RunNow, System, World, WriteStorage};
use tokio::task::JoinHandle;

/// For rendering a ui frame that can mutate state
#[derive(Clone, Component)]
#[storage(DenseVecStorage)]
pub struct Edit(pub fn(&mut AttributeGraph, Option<Thunk>, &Ui));

/// For rendering a ui frame that is read-only
#[derive(Clone, Component)]
#[storage(DenseVecStorage)]
pub struct Display(pub fn(&AttributeGraph, Option<Thunk>, &Ui));

#[derive(Clone)]
/// The render system is to interface entities with specs systems
pub struct Render<'ui>(
    RefCell<Option<&'ui Ui<'ui>>>,
    Option<Thunk>,
    Option<Edit>,
    Option<Display>,
);

impl<'ui> Render<'ui>
{
    /// next_frame prepares the the system for the next frame
    pub fn next_frame(ui: &'ui imgui::Ui<'ui>) -> Self {
        Self(RefCell::new(Some(ui)), None, None, None)
    }

    /// since render needs to happen on the ui thread, this method is to call the system
    /// directly since it can't be handled by the specs dispatcher
    pub fn render_now(&mut self, world: &World) {
        self.run_now(world);
    }

    /// handles the render event
    pub fn on_render<Context>(
        &mut self,
        context: &mut Context,
        thunk: Option<Thunk>,
        edit: Option<Edit>,
        display: Option<Display>,
    )
    where
    Context: Component
        + Clone
        + AsRef<AttributeGraph>
        + AsMut<AttributeGraph>
        + From<AttributeGraph>
        + Sync
        + Send,
    {
        self.1 = thunk;
        self.2 = edit;
        self.3 = display;

        if let Render(ui, thunk, Some(Edit(edit)), ..) = self {
            if let Some(ui) = ui.borrow().and_then(|ui| Some(ui)) {
                edit(context.as_mut(), thunk.clone(), ui);
            }
        }

        if let Render(ui, thunk, .., Some(Display(display))) = self {
            if let Some(ui) = ui.borrow().and_then(|ui| Some(ui)) {
                display(context.as_ref(), thunk.clone(), ui);
            }
        }
    }

    pub fn frame(&self, render: impl FnOnce(&Ui)) {
        if let Some(ui) = &self.0.borrow().and_then(|ui| Some(ui)) {
            render(ui);
        }
    }
}

impl<Context> Plugin<Context> for Render<'_>
where
    Context: Component
        + Clone
        + AsRef<AttributeGraph>
        + AsMut<AttributeGraph>
        + From<AttributeGraph>
        + Sync
        + Send,
{
    fn symbol() -> &'static str {
        "render"
    }

    fn call_with_context(_: &mut Context) -> Option<JoinHandle<Context>> {
        None
    }
}

impl<'a> System<'a> for Render<'_> {
    type SystemData = (
        WriteStorage<'a, AttributeGraph>,
        ReadStorage<'a, Thunk>,
        ReadStorage<'a, Edit>,
        ReadStorage<'a, Display>,
    );

    fn run(&mut self, (mut graphs, thunks, edits, displays): Self::SystemData) {
        for (graph, t, e, d) in (
            &mut graphs,
            thunks.maybe(),
            edits.maybe(),
            displays.maybe(),
        )
            .join()
        {
            let thunk = t.and_then(|t| Some(t.clone()));
            let edit = e.and_then(|e| Some(e.clone()));
            let display = d.and_then(|d| Some(d.clone()));
            self.on_render(graph, thunk, edit, display);
        }
    }
}
