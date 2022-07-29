use specs::Component;
use specs::DenseVecStorage;

/// Component to identify that this entity has a listener enabled
#[derive(Component)]
#[storage(DenseVecStorage)]
pub struct Listener;
