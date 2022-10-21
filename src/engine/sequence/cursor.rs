use specs::{Entity, Component, DenseVecStorage};

/// Enumeration of cursor types for a sequence,
/// 
#[derive(Component, Debug, Clone)]
#[storage(DenseVecStorage)]
pub enum Cursor {
    /// Cursor that points to one other entity,
    /// 
    Next(Entity),
    /// Cursor that points to many entities, 
    /// 
    Fork(Vec<Entity>),
}