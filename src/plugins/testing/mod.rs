use specs::{Component, DefaultVecStorage};

pub struct Test;

/// Debug component 
/// 
#[derive(Component, Default)]
#[storage(DefaultVecStorage)]
pub struct Debug();

