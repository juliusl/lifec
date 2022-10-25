use specs::{Entity, Component, VecStorage};

/// Enumeration of cursor types for a sequence,
/// 
#[derive(Component, Debug, Clone, Hash, PartialEq, Eq)]
#[storage(VecStorage)]
pub enum Cursor {
    /// Cursor that points to one other entity,
    /// 
    Next(Entity),
    /// Cursor that points to many entities, 
    /// 
    Fork(Vec<Entity>),
}