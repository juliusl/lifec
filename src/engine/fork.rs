use reality::{BlockProperty, Interpreter, SpecialAttribute};
use specs::{Component, DenseVecStorage, Entity, WorldExt};
use atlier::system::Value;

use crate::Engine;

/// Engine lifecycle option, will start a list of engines at once
///
#[derive(Default, Component)]
#[storage(DenseVecStorage)]
pub struct Fork(pub Vec<Entity>);

impl SpecialAttribute for Fork {
    fn ident() -> &'static str {
        "fork"
    }

    fn parse(parser: &mut reality::AttributeParser, content: impl AsRef<str>) {
        for control_block in Fork::parse_idents(content.as_ref()) {
            parser.define("fork", Value::Symbol(format!(" {control_block}")));
        }
    }
}

impl Interpreter for Fork {
    fn initialize(&self, world: &mut specs::World) {
        world.register::<Fork>();
    }

    fn interpret(&self, world: &specs::World, block: &reality::Block) {
        if block.is_control_block() {
            if let Some(index) = block.index().iter().find(|i| i.root().name() == "engine") {
                if let Some(forks) = index
                    .properties()
                    .property("fork")
                    .and_then(BlockProperty::symbol_vec)
                {
                    let forks = forks.iter().filter_map(|f| Engine::find_block(world, f));
                    let fork = Fork(forks.collect());
                    let entity = world.entities().entity(block.entity());

                    world
                        .write_component()
                        .insert(entity, fork)
                        .expect("should be able to insert component");
                }
            }
        }
    }
}
