use std::cell::RefCell;

use super::Engine;
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
    Option<Context>,
    Option<Edit<Context>>,
    Option<Display<Context>>,
)
where
    Context: Component;

impl<'ui, Context> Render<'ui, Context>
where
    Context:
        Component + Clone + AsRef<AttributeGraph> + AsMut<AttributeGraph> + From<AttributeGraph>,
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

    /// starts the render engine with graph
    pub fn render_graph(
        &mut self,
        graph: &mut AttributeGraph,
        context: Context,
        edit: Option<Edit<Context>>,
        display: Option<Display<Context>>,
    ) {
        self.1 = Some(context);
        self.2 = edit;
        self.3 = display;

        self.next_mut(graph);
        self.exit(graph);
    }
}

impl<Context> Engine for Render<'_, Context>
where
    Context:
        Component + Clone + AsRef<AttributeGraph> + AsMut<AttributeGraph> + From<AttributeGraph>,
{
    fn next_mut(&mut self, attributes: &mut AttributeGraph) {
        if let Render(ui, Some(context), Some(Edit(edit)), ..) = self {
            if let Some(ui) = ui.borrow().and_then(|ui| Some(ui)) { 
                edit(context, attributes, ui);
            }
        }
    }

    fn exit(&mut self, attributes: &AttributeGraph) {
        if let Render(ui, Some(context), .., Some(Display(display))) = self {
            if let Some(ui) = ui.borrow().and_then(|ui| Some(ui)) { 
                display(context, attributes, ui);            
            }
        }
    }
}

impl<'a, Context> System<'a> for Render<'_, Context>
where
    Context:
        Component + Clone + AsRef<AttributeGraph> + AsMut<AttributeGraph> + From<AttributeGraph>,
{
    type SystemData = (
        WriteStorage<'a, AttributeGraph>,
        ReadStorage<'a, Context>,
        ReadStorage<'a, Edit<Context>>,
        ReadStorage<'a, Display<Context>>,
    );

    fn run(&mut self, (mut graphs, context, edits, displays): Self::SystemData) {
        for (g, c, e, d) in (
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

            if let Some(context) = context {
                self.render_graph(g, context, edit, display);
            }
        }
    }
}

