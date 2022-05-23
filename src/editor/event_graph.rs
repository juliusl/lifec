use serde::{Deserialize, Serialize};
use specs::Component;
use specs::storage::DefaultVecStorage;

use super::EventComponent;

#[derive(Clone, Default, Serialize, Deserialize, Component)]
#[storage(DefaultVecStorage)]
pub struct EventGraph(pub knot::store::Store<EventComponent>);
