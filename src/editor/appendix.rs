use std::collections::HashMap;
use specs::Entity;
use std::hash::Hash;


/// Struct for storing descriptions of entities,
/// 
#[derive(Default, PartialEq, Eq)]
pub struct Appendix {
    pub general: HashMap<Entity, GeneralDescription>,
}

impl Hash for Appendix {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for (e, g) in self.general.iter() {
            e.hash(state);
            g.hash(state);
        }
    }
}

/// General descritpion, name, summary, etc
/// 
#[derive(Default, Hash, PartialEq, Eq)]
pub struct GeneralDescription {
    pub name: String,
}

impl Appendix {
    /// Returns a name for an entity,
    /// 
    pub fn name<'a>(&'a self, entity: &'a Entity) -> Option<&'a str> {
        self.general.get(entity).and_then(|g| Some(g.name.as_str()))
    }
}