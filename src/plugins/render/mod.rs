use std::collections::{BTreeMap, HashMap};

use specs::{Component, System, Entities, WriteStorage, ReadStorage, Join};
use specs::storage::DenseVecStorage;

use crate::AttributeGraph;

mod imgui {
    pub use imgui::Ui;
    use specs::Component;
    use specs::storage::DenseVecStorage;

    use crate::AttributeGraph;

    pub type RenderFn = fn(&mut AttributeGraph, &Ui);

    #[derive(Clone, Component)]
    #[storage(DenseVecStorage)]
    pub struct RenderComponent(pub RenderFn);
}
pub use self::imgui::RenderFn;
pub use self::imgui::RenderComponent;

/// Render is a struct wrapper over a render function for imgui
#[derive(Clone, Component)]
#[storage(DenseVecStorage)]
pub enum Render{
        Empty,
        Func(RenderFn),
        Output(Option<AttributeGraph>),
}

impl Default for Render
{
    fn default() -> Self {
        Self::Empty
    }
}

pub struct RenderContext(AttributeGraph);

impl AsRef<AttributeGraph> for RenderContext {
    fn as_ref(&self) -> &AttributeGraph {
        &self.0
    }
}

impl AsMut<AttributeGraph> for RenderContext {
    fn as_mut(&mut self) -> &mut AttributeGraph {
        &mut self.0
    }
}

impl From<AttributeGraph> for RenderContext {
    fn from(graph: AttributeGraph) -> Self {
        Self(graph)
    }
}

pub struct RenderSystem 
{    
    funcs: BTreeMap<u32, Render>,
    output: HashMap<u32, AttributeGraph>
}

impl<'a> System<'a> for RenderSystem 
{
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, RenderComponent>,
        WriteStorage<'a, Render>,
    );

    fn run(&mut self, (entities, render_component, mut render_output): Self::SystemData) {
        for (e, RenderComponent(render)) in (&entities, &render_component).join() {
            if let None = self.funcs.get(&e.id()) {
                self.funcs.insert(e.id(), Render::Func(render.clone()));
            }
        }

        for (key, next) in self.output.iter() {
            let entity = entities.entity(*key); 
            if entities.is_alive(entity) {
                match render_output.get(entity) {
                    None | Some(Render::Empty) | Some(Render::Output(None))  => {
                        match render_output.insert(entity, Render::Output(Some(next.clone()))) {
                            Ok(_) => todo!(),
                            Err(_) => todo!(),
                        }
                    }
                    Some(Render::Output(Some(current))) => {
                        if next != current {
                            render_output.remove(entity);
                        }
                    }
                    _ => (),
                }
            } else {
                let to_remove = &entity.id();
                self.funcs.remove(to_remove);
            }
        }

        for (key, _) in self.output.iter_mut() {
            let entity = entities.entity(*key);
            if !entities.is_alive(entity) {
               
            }
        }

        todo!()
    }
}