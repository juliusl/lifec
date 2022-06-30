use crate::{
    plugins::{BlockContext, Engine, Plugin, ThunkContext},
    AttributeGraph,
};
use specs::storage::HashMapStorage;
use specs::{Component, Entities, Entity, Join, ReadStorage, System, World};

use super::Call;

pub trait Interpreter {
    fn interpret(&self) -> Option<AttributeGraph>;
}

impl<I> Interpreter for Interpret<I>
where
    I: Plugin<ThunkContext> + Interpreter + Component + Send + Sync,
    <I as specs::Component>::Storage: Default,
{
    fn interpret(&self) -> Option<AttributeGraph> {
        if let Self(interpret, Some(interpreter)) = &self {
           (interpret)(interpreter)
        } else {
            None
        }
    }
}

/// Interpret engine sequences an event, and an interpreter that does something with the result of the event
#[derive(Component)]
#[storage(HashMapStorage)]
pub struct Interpret<I>(
    /// interpet fn
    fn(&I) -> Option<AttributeGraph>, 
    /// Optionally, if a value is provided and interpreter system is running, this will take the value and call the interpet fn
    Option<I>
)
where
    I: Plugin<ThunkContext> + Interpreter + Component + Send + Sync,
    <I as specs::Component>::Storage: Default;


impl<I> Default for Interpret<I>
where
    I: Plugin<ThunkContext> + Interpreter + Component + Send + Sync,
    <I as specs::Component>::Storage: Default,
{
    fn default() -> Self {
        Self(I::interpret, None)
    }
}

impl<I> Engine for Interpret<I>
where
    I: Plugin<ThunkContext> + Interpreter + Component + Send + Sync,
    <I as specs::Component>::Storage: Default,
{
    fn event_name() -> &'static str {
        "interpret"
    }

    fn create_event(entity: Entity, world: &World) {
        // interpret starts with a "call"
        <Call as Engine>::create_event(entity, world);
    }
}

impl<'a, I> System<'a> for Interpret<I>
where
    I: Plugin<ThunkContext> + Interpreter + Component + Send + Sync,
    <I as specs::Component>::Storage: Default,
{
    type SystemData = (Entities<'a>, ReadStorage<'a, ThunkContext>);

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
