use specs::Component;
use specs::storage::DenseVecStorage;

use crate::plugins::Engine;

use super::Call;

#[derive(Component, Default)]
#[storage(DenseVecStorage)]
pub struct Fix; 

impl Engine for Fix {
    fn event_symbol() -> &'static str {
        "fix"
    }

    fn create_event(entity: specs::Entity, world: &specs::World) {
        Call::create_event(entity, world)
    }

    fn init_event(entity: specs::EntityBuilder, event: crate::plugins::Event) -> specs::EntityBuilder {
        Call::init_event(entity, event)
    }
}
