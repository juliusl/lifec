use imnodes::{ImVec2, Link, NodeId};
use knot::store::{Store, Visitor};
use std::collections::HashSet;

#[derive(Default, Clone)]
pub struct NodeEditorGraph;

impl NodeEditorGraph {
    /// rearrange a set of linked nodes
    pub fn rearrange(links: &mut HashSet<Link>) {
        let mut store = Store::<NodeId>::default();
        store.walk_unique = false;

        let mut first: Option<NodeId> = None;
        let coordinate_system = imnodes::CoordinateSystem::GridSpace;

        // This first part arranges the events horizontally
        // meanwhile, start's adding links to the above store
        for _ in 0..links.len() {
            for Link {
                start_node,
                end_node,
                ..
            } in links.clone().iter()
            {
                let ImVec2 { x, y } = start_node.get_position(coordinate_system);
                let start_x = x + 400.0;
                let start_y = y;

                end_node.set_position(start_x, start_y, coordinate_system);
                store = store.link_create_if_not_exists(start_node.clone(), end_node.clone());

                if first.is_none() {
                    first = Some(start_node.clone());
                }
            }
        }

        // This next part arranges the events that need space vertically, usually only places where events branch
        // we use the store we created above to rewalk the graph in order to figure out if we have branches
        // if we have branches, then the children of the parent need to spaced vertically.
        // if we don't have any branches, then we don't need any spacing vertically
        if let Some(first) = first {
            let (seen, _) =
                store.new_walk::<_, NodeEditorGraph>(first, Some(&NodeEditorGraph::default()));

            for s in seen {
                let node = store.get(s);
                if let Some((id, refs)) = node.1 {
                    if refs.len() >= 2 {
                        println!("vertical rearranging {:?} {:?}", id, refs);
                        for (pos, end_node) in store
                            .clone()
                            .visit(*id)
                            .iter()
                            .filter_map(|r| r.1)
                            .enumerate()
                        {
                            let ImVec2 { x: _, y } = id.get_position(coordinate_system);

                            let start_y = y + (pos as f32) * 200.0;

                            let ImVec2 { x, y: _ } = end_node.get_position(coordinate_system);
                            end_node.set_position(x, start_y, coordinate_system);
                        }
                    }
                }
            }
        }
    }
}

impl Visitor<NodeId> for NodeEditorGraph {
    fn visit(&self, _: &NodeId, _: &NodeId) -> bool {
        true
    }
}
