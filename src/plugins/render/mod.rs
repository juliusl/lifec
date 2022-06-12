
use imgui::Ui;
use specs::storage::DenseVecStorage;
use specs::{
    Component, Join, ReadStorage, RunNow, System, World, WriteStorage,
};

use crate::AttributeGraph;

use super::{Engine, Plugin};

#[derive(Clone, Component)]
#[storage(DenseVecStorage)]
pub struct Edit<Context>(pub fn(&mut Context, &mut AttributeGraph, &Ui)) 
where
    Context: Component;

#[derive(Clone, Component)]
#[storage(DenseVecStorage)]
pub struct Display<Context>(pub fn(&Context, &AttributeGraph, &Ui)) 
where
    Context: Component;

pub struct Render<'a, 'ui, Context>(
    &'a Ui<'ui>,
    Option<Context>,
    Option<Edit<Context>>,
    Option<Display<Context>>,
)
where
    Context: Component;

impl<'a, 'ui, Context> Render<'a, 'ui, Context>
where
    Context: Component + Clone + AsRef<AttributeGraph> + AsMut<AttributeGraph> + From<AttributeGraph>
{
    pub fn new(ui: &'a imgui::Ui<'ui>) -> Self {
        Self(ui, None, None, None)
    }

    pub fn render_now(&mut self, world: &'a World) {
        self.run_now(world);
    }
}

impl<'a, 'ui, Context> Engine for Render<'a, 'ui, Context>
where
    Context: Component + Clone + AsRef<AttributeGraph> + AsMut<AttributeGraph> + From<AttributeGraph>
{
    fn next_mut(&mut self, attributes: &mut AttributeGraph) {
        if let Render(ui, Some(context), Some(Edit(edit)), ..) = self {
            edit(context, attributes, ui);
        }
    }

    fn exit(&mut self, attributes: &AttributeGraph) {
        if let Render(ui, Some(context), .., Some(Display(display))) = self {
            display(context, attributes, ui);
        }
    }
}

impl<'a, 'ui, Context> System<'a> for Render<'a, 'ui, Context>
where
    Context: Component + Clone + AsRef<AttributeGraph> + AsMut<AttributeGraph> + From<AttributeGraph>
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

            let mut engine = Self(self.0, context, edit, display);
            engine.next_mut(g);
            engine.exit(g);
        }
    }
}
