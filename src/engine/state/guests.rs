use specs::prelude::*;
use tracing::Level;
use tracing::event;

use crate::engine::Performance;
use crate::guest::Guest;
use crate::prelude::NodeCommand;

use super::State;

impl<'a> State<'a> {
    /// Take commands from storage,
    /// 
    pub fn take_commands(&mut self) -> Vec<(Entity, NodeCommand)> {
        (&self.entities, self.commands.drain()).join().collect()
    }

    /// Takes performance from world state,
    /// 
    pub fn take_performance(&mut self) -> Vec<Performance> {
        let mut samples = vec![];
        for (entity, sample) in (&self.entities, self.samples.drain()).join() {
            samples.push(sample);
            match self.entities.delete(entity) {
                Ok(_) => {
                    
                },
                Err(err) => {
                    event!(Level::ERROR, "could remove sample entity {}, {err}", entity.id());
                },
            }
        }

        samples
    }

    /// Returns an iterator over guests,
    /// 
    pub fn guests(&self) -> impl Iterator<Item = &Guest> {
        (&self.entities, &self.guests).join().map(|(_, g)| g)
    }
}