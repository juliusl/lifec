use specs::{Builder, Component, DefaultVecStorage, WorldExt};

pub struct Test;

/// Debug component 
/// 
#[derive(Component, Default)]
#[storage(DefaultVecStorage)]
pub struct Debug();

