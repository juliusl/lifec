use std::collections::HashMap;
use specs::Entity;
use std::hash::Hash;

mod general;
pub use general::General;

/// Struct for storing descriptions of entities,
/// 
#[derive(Default, PartialEq, Eq)]
pub struct Appendix {
    pub general: HashMap<Entity, General>,
}

impl Appendix {
    /// Inserts a general description for the entity to the appendix,
    /// 
    pub fn insert_general(&mut self, entity: Entity, general: impl Into<General>) {
        self.general.insert(entity, general.into());
    }

    /// Returns a general description of the entity
    /// 
    pub fn general<'a>(&'a self, entity: &'a Entity) -> Option<&'a General> {
        self.general.get(entity)
    }

    /// Returns a name for an entity,
    /// 
    pub fn name<'a>(&'a self, entity: &'a Entity) -> Option<&'a str> {
        self.general(entity).and_then(|g| Some(g.name.as_str()))
    }
}

impl Hash for Appendix {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for (e, g) in self.general.iter() {
            e.hash(state);
            g.hash(state);
        }
    }
}