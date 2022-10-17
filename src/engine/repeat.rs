use reality::SpecialAttribute;
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
        if let (Some(count), Some(entity)) = (content.as_ref().parse::<usize>().ok(), parser.entity()) {
            let world = parser.world().expect("should be a world");
            world
                .write_component()
                .insert(entity, Repeat(Some(count)))
                .expect("should be able to insert component");
        }
    }
}
