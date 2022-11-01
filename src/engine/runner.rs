use specs::SystemData;
use specs::prelude::*;

use crate::guest::Guest;
use crate::prelude::NodeCommand;

/// Runner system data,
/// 
#[derive(SystemData)]
pub struct Runner<'a> {
    /// Entities
    /// 
    pub entities: Entities<'a>,
    /// Guests,
    /// 
    pub guests: WriteStorage<'a, Guest>,
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

    /// Returns an iterator over guests,
    /// 
    pub fn guests(&self) -> impl Iterator<Item = &Guest> {
        (&self.entities, &self.guests).join().map(|(_, g)| g)
    }
}