use reality::{SpecialAttribute, Interpreter, BlockProperty};
use specs::{Component, DenseVecStorage, WorldExt};

/// Engine lifecycle option, will repeat the current engine,
/// 
/// If a limit is specified, it will decrement the counter, otherwise 
/// will repeat indefinitely
/// 
#[derive(Default, Component)]
#[storage(DenseVecStorage)]
pub struct Repeat(pub Option<usize>);

impl SpecialAttribute for Repeat {
    fn ident() -> &'static str {
        "repeat"
    }

    fn parse(parser: &mut reality::AttributeParser, content: impl AsRef<str>) {
        if let Some(count) = content.as_ref().parse::<usize>().ok() {
            parser.define("repeat", count);
        }
    }
}

impl Interpreter for Repeat {
    fn initialize(&self, world: &mut specs::World) {
        world.register::<Repeat>();
    }

    fn interpret(&self, world: &specs::World, block: &reality::Block) {
        if block.is_control_block() {
            if let Some(index) = block.index().iter().find(|i| i.root().name() == "engine") {
                if let Some(repeat) = index
                    .properties()
                    .property("repeat")
                    .and_then(BlockProperty::int)
                {
                    let entity = world.entities().entity(block.entity());
                    world
                        .write_component()
                        .insert(entity, Repeat(Some(repeat as usize)))
                        .expect("should be able to insert component");
                }
            }
        }
    }
}