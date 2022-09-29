use reality::{BlockProperty, Interpreter, SpecialAttribute};
use specs::{WorldExt, Component, DenseVecStorage, Entity};
use tracing::{event, Level};
use atlier::system::Value;

use crate::Engine;

/// Engine lifecycle option, will start one engine
///
#[derive(Default, Component)]
#[storage(DenseVecStorage)]
pub struct Next(pub Option<Entity>);

impl SpecialAttribute for Next {
    fn ident() -> &'static str {
        "next"
    }

    fn parse(parser: &mut reality::AttributeParser, content: impl AsRef<str>) {
        let idents = Next::parse_idents(content.as_ref());

        match (idents.get(0), idents.get(1)) {
            (Some(name), Some(symbol)) => {
                parser.define("next", Value::Symbol(format!("{name} {symbol}")));
            }
            (Some(symbol), None) => {
                parser.define("next", Value::Symbol(format!(" {symbol}")));
            }
            _ => {
                event!(Level::ERROR, "Invalid format idents state");
            }
        }
    }
}

impl Interpreter for Next {
    fn initialize(&self, world: &mut specs::World) {
        world.register::<Next>();
    }

    fn interpret(&self, world: &specs::World, block: &reality::Block) {
        if block.is_control_block() {
            if let Some(index) = block.index().iter().find(|i| i.root().name() == "engine") {
                if let Some(next) = index
                    .properties()
                    .property("next")
                    .and_then(BlockProperty::symbol)
                    .and_then(|p| Engine::find_block(world, p))
                {
                    let entity = world.entities().entity(block.entity());
                    world
                        .write_component()
                        .insert(entity, Next(Some(next)))
                        .expect("should be able to insert component");
                }
            }
        }
    }
}
