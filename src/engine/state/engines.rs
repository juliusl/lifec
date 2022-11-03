use specs::{Entity, Join};
use crate::engine::EngineStatus;
use super::State;

impl<'a> State<'a> {
    /// Scans engines for status,
    /// 
    pub fn scan_engine_status(&'a self) -> Vec<EngineStatus> {
        let mut statuses = vec![];
        for (entity, sequence, _) in (&self.entities, &self.sequences, &self.engines).join() {
            let mut _events = sequence.iter_entities().map(|e| self.status(e) );

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
        self.entity_map.get(expression.as_ref())
    }
}