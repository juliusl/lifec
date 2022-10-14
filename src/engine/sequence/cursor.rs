use specs::Entity;


/// Enumeration of cursor types for a sequence,
/// 
#[derive(Debug, Clone)]
pub enum Cursor {
    /// Cursor that creates only 1 branch,
    /// 
    Next(Entity),
    /// Cursor that points to many branches, 
    /// 
    Fork(Vec<Entity>),
}