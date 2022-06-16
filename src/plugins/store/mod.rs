use knot::store::Store;
use specs::Component;
use specs::storage::HashMapStorage;
use crate::AttributeGraph;


#[derive(Component)]
#[storage(HashMapStorage)]
pub struct StoreContext {
    graph: AttributeGraph,
    _store: Store<()>,
}

impl AsRef<AttributeGraph> for StoreContext {
    fn as_ref(&self) -> &AttributeGraph {
        &self.graph
    }
}

impl AsMut<AttributeGraph> for StoreContext {
    fn as_mut(&mut self) -> &mut AttributeGraph {
        &mut self.graph
    }
}

impl From<AttributeGraph> for StoreContext {
    fn from(graph: AttributeGraph) -> Self {
        Self { graph, _store: Store::default() }
    }
}