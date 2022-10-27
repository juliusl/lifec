use std::collections::HashMap;
use specs::{WriteStorage, SystemData};
use specs::prelude::*;

use super::{Engine, Events};


/// System data for engines,
/// 
#[derive(SystemData)]
pub struct Engines<'a> {
    blocks: Read<'a, HashMap<String, Entity>>,
    events: Events<'a>,
    _entities: Entities<'a>,
    engines: WriteStorage<'a, Engine>,
}

impl<'a> Engines<'a> {
    /// Scans engines for status,
    /// 
    pub fn scan_engines(&'a self) -> Vec<EngineStatus> {
        let Engines { events, engines, .. } = self; 

        let mut statuses = vec![];
        for (entity, sequence, _) in events.join_sequences(engines) {
            let mut _events = sequence.iter_entities().map(|e| events.status(e) );

            if _events.all(|e| match e {
                super::EventStatus::Inactive(_) => true,
                _ => false, 
            }) {
                statuses.push(EngineStatus::Inactive(entity));
            } else {
                statuses.push(EngineStatus::Active(entity));
            }
        }

        statuses
    }

    /// Returns the entity for a block by name, 
    /// 
    pub fn find_block_entity(&self, expression: impl AsRef<str>) -> Option<&Entity> {
        self.blocks.get(expression.as_ref())
    }
}

/// Enumeration of possible engine statuses,
/// 
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum EngineStatus {
    /// All events under this engine are inactive,
    /// 
    Inactive(Entity),
    /// Some events under this engine are active,
    /// 
    Active(Entity),
}