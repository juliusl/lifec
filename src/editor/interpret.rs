use specs::{Entity, World, System, ReadStorage, Entities, Join};

use crate::plugins::{Engine, ThunkContext};

use super::Call;

#[derive(Default)]
pub struct Interpret;

impl Engine for Interpret {
    fn event_name() -> &'static str {
        "interpret"
    }

    fn create_event(entity: Entity, world: &World) {
        // interpret starts with a "call"
        <Call as Engine>::create_event(entity, world);
    }
}

impl<'a> System<'a> for Interpret {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, ThunkContext>
    );

    fn run(&mut self, (entities, contexts): Self::SystemData) {
        for (_entity, context) in (&entities, contexts.maybe()).join() {
            if let Some(_context) = context {
                if let Some(true) = _context.as_ref().is_enabled("interpret") {
                   
                }
            }
        }
    }
}