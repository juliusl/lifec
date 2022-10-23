use specs::{Component, DenseVecStorage};

/// Component to indicate a limit,
/// 
#[derive(Component)]
#[storage(DenseVecStorage)]
pub struct Limit(pub usize);