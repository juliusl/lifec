use std::cell::RefCell;

use super::{Engine, Plugin};
use crate::AttributeGraph;
use imgui::Ui;
use specs::storage::DenseVecStorage;
use specs::{Component, Join, ReadStorage, RunNow, System, World, WriteStorage};

/// For rendering a ui frame that can mutate state
#[derive(Clone, Component)]
#[storage(DenseVecStorage)]
pub struct Edit<Context>(pub fn(&Context, &mut AttributeGraph, &Ui))
where
    Context: Component;

/// For rendering a ui frame that is read-only
#[derive(Clone, Component)]
#[storage(DenseVecStorage)]
pub struct Display<Context>(pub fn(&Context, &AttributeGraph, &Ui))
where
    Context: Component;

#[derive(Clone)]
/// The render system is to interface entities with specs systems
pub struct Render<'ui, Context>(
    RefCell<Option<&'ui Ui<'ui>>>,
    Option<Edit<Context>>,
    Option<Display<Context>>,
)
where
    Context: Component;

impl<'ui, Context> Render<'ui, Context>
where
    Context: Component
        + Clone
        + AsRef<AttributeGraph>
        + AsMut<AttributeGraph>
        + From<AttributeGraph>
        + Send
        + Sync,
{
    /// next_frame prepares the the system for the next frame
    pub fn next_frame(ui: &'ui imgui::Ui<'ui>) -> Self {
        Self(RefCell::new(Some(ui)), None, None)
    }

    /// since render needs to happen on the ui thread, this method is to call the system
    /// directly since it can't be handled by the specs dispatcher
    pub fn render_now(&mut self, world: &World) {
        self.run_now(world);
    }

    /// starts the render engine with graph
    pub fn render_context(
        &mut self,
        context: &mut Context,
        edit: Option<Edit<Context>>,
        display: Option<Display<Context>>,
    ) where
        Self: Plugin<Context>,
    {
        self.1 = edit;
        self.2 = display;
        self.on_event(context);
    }

    pub fn frame(&self, render: impl FnOnce(&Ui)) {
        if let Some(ui) = &self.0.borrow().and_then(|ui| Some(ui)) {
            render(ui);
        }
    }
}

impl<Context> Engine for Render<'_, Context>
where
    Context:
        Component + Clone + AsRef<AttributeGraph> + AsMut<AttributeGraph> + From<AttributeGraph>,
{
    fn next_mut(&mut self, attributes: &mut AttributeGraph) {
        let context = Context::from(attributes.clone());

        if let Render(ui, Some(Edit(edit)), ..) = self {
            if let Some(ui) = ui.borrow().and_then(|ui| Some(ui)) {
                edit(&context, attributes, ui);
            }
        }
    }

    fn exit(&mut self, attributes: &AttributeGraph) {
        let context = Context::from(attributes.clone());

        if let Render(ui, .., Some(Display(display))) = self {
            if let Some(ui) = ui.borrow().and_then(|ui| Some(ui)) {
                display(&context, attributes, ui);
            }
        }
    }
}

impl<Context> Plugin<Context> for Render<'_, Context>
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

    fn call_with_context(_: &mut Context) {}
}

impl<'a, Context> System<'a> for Render<'_, Context>
where
    Context: Component
        + Clone
        + AsRef<AttributeGraph>
        + AsMut<AttributeGraph>
        + From<AttributeGraph>
        + Send
        + Sync,
{
    type SystemData = (
        WriteStorage<'a, AttributeGraph>,
        ReadStorage<'a, Context>,
        ReadStorage<'a, Edit<Context>>,
        ReadStorage<'a, Display<Context>>,
    );

    fn run(&mut self, (mut graphs, context, edits, displays): Self::SystemData) {
        for (graph, c, e, d) in (
            &mut graphs,
            context.maybe(),
            edits.maybe(),
            displays.maybe(),
        )
            .join()
        {
            let context = c.and_then(|c| Some(c.clone()));
            let edit = e.and_then(|e| Some(e.clone()));
            let display = d.and_then(|d| Some(d.clone()));

            if let Some(mut context) = context {
                self.render_context(&mut context, edit, display);
                graph.merge(context.as_ref());
            }
        }
    }
}
