use specs::SystemData;
use specs::prelude::*;

use crate::prelude::NodeCommand;

/// Runner system data,
/// 
#[derive(SystemData)]
pub struct Runner<'a> {
    /// Entities
    /// 
    entities: Entities<'a>,
    /// Node commands,
    /// 
    commands: WriteStorage<'a, NodeCommand>,
}

impl<'a> Runner<'a> {
    /// Take commands from storage,
    /// 
    pub fn take_commands(&mut self) -> Vec<(Entity, NodeCommand)> {
        (&self.entities, self.commands.drain()).join().collect()
    }
}