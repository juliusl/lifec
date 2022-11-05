use std::collections::HashMap;
use specs::Entity;
use std::hash::Hash;

mod general;
pub use general::General;

mod state;
pub use state::State;

/// Struct for storing descriptions of entities,
/// 
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Appendix {
    /// Index of General information about an entity,
    /// 
    pub general: HashMap<Entity, General>,
    /// Index of State infomation about an entity,
    /// 
    pub state: HashMap<Entity, State>,
    /// Prefix notes,
    /// 
    pub prefix_notes: HashMap<String, String>,
}

impl Appendix {
    /// Inserts a general description for the entity to the appendix,
    /// 
    pub fn insert_general(&mut self, entity: Entity, general: impl Into<General>) {
        self.general.insert(entity, general.into());
    }

    /// Inserts a state description for the entity to the appendix,
    /// 
    pub fn insert_state(&mut self, entity: Entity, state: impl Into<State>) {
        self.state.insert(entity, state.into());
    }

    /// Returns a state description for the entity,
    /// 
    pub fn state<'a>(&'a self, entity: &'a Entity) -> Option<&'a State> {
        self.state.get(entity)
    }

    /// Returns a general description of the entity,
    /// 
    pub fn general<'a>(&'a self, entity: &'a Entity) -> Option<&'a General> {
        self.general.get(entity)
    }

    /// Returns a name for an entity,
    /// 
    pub fn name<'a>(&'a self, entity: &'a Entity) -> Option<&'a str> {
        self.general(entity).and_then(|g| Some(g.name.as_str()))
    }

    /// Returns a name for an entity,
    /// 
    pub fn control_symbol<'a>(&'a self, entity: &'a Entity) -> Option<String> {
        self.state(entity).and_then(|g| Some(g.control_symbol.to_string()))
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