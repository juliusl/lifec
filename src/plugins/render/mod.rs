
use imgui::Ui;
use specs::storage::DenseVecStorage;
use specs::{Component, System, Entities, WriteStorage, Join, RunNow};

use crate::AttributeGraph;

pub type RenderFn = fn(&mut AttributeGraph, &Ui);

#[derive(Clone, Component)]
#[storage(DenseVecStorage)]
pub struct RenderComponent(pub RenderFn);


pub struct Render<'a, 'ui>(pub &'a Ui<'ui>);

impl<'a, 'ui> System<'a> for Render<'a, 'ui> 
{
    type SystemData = (
        Entities<'a>, 
        WriteStorage<'a, AttributeGraph>, 
        WriteStorage<'a, RenderComponent>
    );

    fn run(&mut self, (entities, mut graphs, mut render_components): Self::SystemData) {
        for e in entities.join() {
            if let (Some(graph), Some(RenderComponent(render))) = (graphs.get_mut(e), render_components.get_mut(e)) {
                 render(graph, self.0);
            }
        }
    }
}


impl<'a, 'ui> Render<'a, 'ui> 
{
    pub fn run(&mut self, app_world: &'a specs::World) {
        self.run_now(app_world);
     }
}
