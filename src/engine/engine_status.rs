use specs::prelude::*;


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