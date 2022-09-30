use reality::{Interpreter, SpecialAttribute};
use specs::{Component, VecStorage, WorldExt};

/// Engine lifecycle option, will loop the sequence indefinitely
/// 
#[derive(Component, Default)]
#[storage(VecStorage)]
pub struct Loop;

impl SpecialAttribute for Loop {
    fn ident() -> &'static str {
        "loop"
    }

    fn parse(parser: &mut reality::AttributeParser, _: impl AsRef<str>) {
        parser.define("loop", true);
    }
}

impl Interpreter for Loop {
    fn initialize(&self, world: &mut specs::World) {
        world.register::<Loop>();
    }

    fn interpret(&self, world: &specs::World, block: &reality::Block) {
        if block.is_control_block() {
            if let Some(index) = block.index().iter().find(|i| i.root().name() == "engine") {
                if index
                    .properties()
                    .property("loop")
                    .and_then(|p| Some(p.is_enabled()))
                    .unwrap_or_default()
                {
                    let entity = world.entities().entity(block.entity());
                    world
                        .write_component()
                        .insert(entity, Loop::default())
                        .expect("should be able to insert component");
                }
            }
        }
    }
}
