use specs::World;

use crate::{plugins::{ThunkContext, Plugin}, AttributeGraph, Runtime};

/// Implement to listen for thunk context updates wrt to runtime and world state
pub trait Listen 
where
    Self: Plugin<ThunkContext>
{
    /// The runtime stores all of the receivers, and the world manages the storage
    /// this allows you to use the runtime to receive an update, and the world to get the update
    fn listen(runtime: &mut Runtime, world: &World) -> Option<AttributeGraph>;
}