use specs::{Component, DenseVecStorage, WorldExt};
use reality::{SpecialAttribute, Interpreter};
use atlier::system::Value;
use crate::Event;

/// Wrapper struct over an Event component,
///
/// Will be initiated and executed only once,
///
#[derive(Component)]
#[storage(DenseVecStorage)]
pub struct Once(Option<Event>);

impl SpecialAttribute for Once {
    fn ident() -> &'static str {
        "once"
    }

    fn parse(parser: &mut reality::AttributeParser, content: impl AsRef<str>) {
        Event::parse(parser, content.as_ref());

        parser.define("once", Value::Symbol(content.as_ref().to_string()));
    }
}

impl Interpreter for Once {
    fn initialize(&self, world: &mut specs::World) {
        world.register::<Once>();
    }

    fn interpret(&self, world: &specs::World, block: &reality::Block) {
        if !block.is_control_block() && !block.is_root_block() {
            todo!()
        }
    }
}