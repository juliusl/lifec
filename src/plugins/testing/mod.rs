use specs::{Component, DefaultVecStorage};

mod chaos;
pub use chaos::Chaos;

pub struct Test;

/// Debug component 
/// 
#[derive(Component, Default)]
#[storage(DefaultVecStorage)]
pub struct Debug();

