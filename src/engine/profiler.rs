use specs::{Component, HashMapStorage};

/// Component to create a terminal point for adhoc events to point to,
/// 
#[derive(Component, Default)]
#[storage(HashMapStorage)]
pub struct Profiler;