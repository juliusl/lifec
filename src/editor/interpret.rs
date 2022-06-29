use specs::{Entity, World, System, ReadStorage, Entities, Join};

use crate::plugins::{Engine, ThunkContext, BlockContext};

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
                   for block in _context.as_ref().iter_blocks() {
                        let block = BlockContext::from(block);
                        if let Some(file) = block.get_block("file") {
                            // TODO do the thing
                            file.write_file_as(".interpret", "content").ok();
                        }
                   }
                }
            }
        }
    }
}