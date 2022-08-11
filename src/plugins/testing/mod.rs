use specs::{Builder, Component, DefaultVecStorage, WorldExt};

use super::Engine;

pub struct Test;

impl Engine for Test {
    fn event_name() -> &'static str {
        "test"
    }

    fn create_event(engine: specs::Entity, world: &specs::World) {
        // TODO
        world.write_component().insert(engine, Debug()).expect("Can add debug component");
    }

    fn init_event(entity: specs::EntityBuilder, event: super::Event) -> specs::EntityBuilder {
        // TODO 
        entity
            .with(event)
            .with(Debug())
    }
}

/// Debug component 
/// 
#[derive(Component, Default)]
#[storage(DefaultVecStorage)]
pub struct Debug();

