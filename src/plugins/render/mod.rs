
use imgui::Ui;
use specs::storage::DenseVecStorage;
use specs::{
    Component, Join, ReadStorage, RunNow, System, World, WriteStorage,
};

use crate::AttributeGraph;

use super::{Engine, Plugin};

#[derive(Clone, Component)]
#[storage(DenseVecStorage)]
pub struct Edit(pub fn(&mut AttributeGraph, &Ui));

#[derive(Clone, Component)]
#[storage(DenseVecStorage)]
pub struct Display(pub fn(&AttributeGraph, &Ui));

pub struct Render<'a, 'ui, Context, P>(
    &'a Ui<'ui>,
    Option<Context>,
    Option<Edit>,
    Option<Display>,
    Option<P>,
)
where
    Context: Component
        + Clone
        + Default
        + AsRef<AttributeGraph>
        + AsMut<AttributeGraph>
        + From<AttributeGraph>,
    P: Plugin<Context> + Component + Clone;

impl<'a, 'ui, Context, P> Render<'a, 'ui, Context, P>
where
    Context: Component
        + Clone
        + Default
        + AsRef<AttributeGraph>
        + AsMut<AttributeGraph>
        + From<AttributeGraph>,
    P: Plugin<Context> + Component + Clone,
{
    pub fn new(ui: &'a imgui::Ui<'ui>) -> Self {
        Self(ui, None, None, None, None)
    }

    pub fn render_now(&mut self, world: &'a World) {
        self.run_now(world);
    }
}

impl<'a, 'ui, Context, P> Engine for Render<'a, 'ui, Context, P>
where
    Context: Component
        + Clone
        + Default
        + AsRef<AttributeGraph>
        + AsMut<AttributeGraph>
        + From<AttributeGraph>,
    P: Plugin<Context> + Component + Clone,
{
    fn next_mut(&mut self, attributes: &mut AttributeGraph) {
        if let Render(ui, Some(_), Some(Edit(edit)), .., Some(_)) = self {
            if ui.button(format!("Call {} {}", P::symbol(), attributes.entity())) {
                P::call(attributes);
            }

            edit(attributes, ui);
        }
    }

    fn exit(&mut self, attributes: &AttributeGraph) {
        if let Render(ui, Some(_), .., Some(Display(display)), Some(_)) = self {
            display(attributes, ui);
        }
    }
}

impl<'a, 'ui, Context, P> System<'a> for Render<'a, 'ui, Context, P>
where
    Context: Component
        + Clone
        + Default
        + AsRef<AttributeGraph>
        + AsMut<AttributeGraph>
        + From<AttributeGraph>,
    P: Plugin<Context> + Component + Clone,
{
    type SystemData = (
        WriteStorage<'a, AttributeGraph>,
        ReadStorage<'a, Context>,
        ReadStorage<'a, P>,
        ReadStorage<'a, Edit>,
        ReadStorage<'a, Display>,
    );

    fn run(&mut self, (mut graphs, context, plugin, edits, displays): Self::SystemData) {
        for (g, c, p, e, d) in (
            &mut graphs,
            context.maybe(),
            plugin.maybe(),
            edits.maybe(),
            displays.maybe(),
        )
            .join()
        {
            let context = c.and_then(|c| Some(c.clone()));
            let edit = e.and_then(|e| Some(e.clone()));
            let display = d.and_then(|d| Some(d.clone()));
            let plugin = p.and_then(|p| Some(p.clone()));

            let engine = Self(self.0, context, edit, display, plugin);
            P::on_update(g, engine);
        }
    }
}
