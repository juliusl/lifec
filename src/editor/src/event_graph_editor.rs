use std::collections::{HashSet, HashMap};

use atlier::system::App;
use imnodes::{Link, LinkId, NodeId, InputPinId, AttributeId, OutputPinId, AttributeFlag, IdentifierGenerator, ImVec2};
use knot::store::{Store, Visitor};

use crate::event_editor::EventEditor;

#[derive(Default, Clone)]
pub struct EventGraphEditor {
    pub events: Vec<EventEditor>,
    pub links: HashSet<Link>,
    pub link_index: HashMap::<LinkId, Link>,
    pub node_index: HashMap<NodeId, EventEditor>,
    pub graph_index: HashMap<EventEditor, (NodeId, InputPinId, AttributeId, OutputPinId)>,
}

impl App for EventGraphEditor {
    fn title() -> &'static str {
        "Graph Editor"
    }

    fn show(
        ui: &imgui::Ui,
        state: &Self,
        imnode_editor: Option<&mut imnodes::EditorContext>,
    ) -> Option<Self> {
        if let Some(editor_context) = imnode_editor {
            let mut next = state.clone();

            if ui.button("Re-Arrange") {
                next.rearrange();
            }

            let detach = editor_context.push(AttributeFlag::EnableLinkDetachWithDragClick);
            let idgen = &mut editor_context.new_identifier_generator();
            let mut next_events = next.events.clone();

            let outer = imnodes::editor(editor_context, |mut scope| {
                scope.add_mini_map(imnodes::MiniMapLocation::TopRight);

                let mut i = 0;
                for e in &next.events {
                    let node_id = idgen.next_node();
                    scope.add_node(node_id, |node| {
                        if let Some(next_e) = EventEditor::show_node(ui, &e, node, idgen) {
                            next.node_index.insert(node_id, next_e.clone());
                            next_events[i] = next_e;
                        }
                    });
                    i += 1;
                }

                for link in &next.links {
                    let link_id = idgen.next_link();
                    scope.add_link(link_id, link.end_pin, link.start_pin);
                    next.link_index.insert(link_id, link.clone());
                }
            });

            let mut next_links = HashSet::new();
            if let Some(link) = outer.links_created() {
                next_links.insert(link);

                if let (Some(start), Some(end)) = (
                    next.node_index.get(&link.start_node),
                    next.node_index.get(&link.end_node),
                ) {
                    if let Some(start_pos) = next_events.iter().position(|e| *e == *start) {
                        let mut updated_start = start.clone();
                        updated_start.transitions.push(end.on.to_owned());
                        next_events[start_pos] = updated_start;
                    }
                }
            }

            if let Some(destroyed) = outer.get_dropped_link() {
                if let Some(link) = next.link_index.get(&destroyed) {
                    next.links.remove(link);

                    let start_node_id = link.start_node;
                    if let Some(start) = next.node_index.get(&start_node_id) {
                        if let Some(start_pos) = next_events.iter().position(|e| *e == *start) {
                            let mut updated_start = next_events[start_pos].clone();
                            updated_start.transitions = updated_start
                                .transitions
                                .iter()
                                .filter(|s| **s != start.on)
                                .map(|s| s.to_owned())
                                .collect();
                            next_events[start_pos] = updated_start;
                        }
                    }
                }
            }

            detach.pop();
            if next_links.len() > 0 || next.links.len() != state.links.len() {
                next.links = next.links.union(&next_links).cloned().collect();
                Some(next)
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl EventGraphEditor {
    fn push_node(&mut self, idgen: &mut IdentifierGenerator, event: EventEditor) {
        self.graph_index.insert(
            event.clone(),
            (
                idgen.next_node(),
                idgen.next_input_pin(),
                idgen.next_attribute(),
                idgen.next_output_pin(),
            ),
        );
    }

    pub fn graph_contents(&mut self, idgen: &mut IdentifierGenerator) {
        let mut store = Store::<EventEditor>::default();

        let next_events = self.events.clone();
        for e in &next_events {
            store = store.node(e.clone());

            if !&e.transitions.is_empty() {
                for to in
                    self.events.clone().iter().filter(|o| {
                        e.transitions.iter().find(|p| **p == o.on).is_some()
                    })
                {
                    store = store.link_create_if_not_exists(e.clone(), to.to_owned());
                }
            }

            self.push_node(idgen, e.clone());
        }

        store.new_walk_mut(next_events[0].clone(), Some(self));
    }

    fn rearrange(&mut self) {
        let mut store = Store::<NodeId>::default();
        let mut first: Option<NodeId> = None;

        // This first part arranges the events horizontally
        for _ in 0..self.links.len() {
            for Link {
                start_node,
                end_node,
                ..
            } in self.links.clone().iter()
            {
                let ImVec2 { x, y } = start_node.get_position(imnodes::CoordinateSystem::GridSpace);
                let start_x = x + 400.0;
                let start_y = y + 75.0;

                end_node.set_position(start_x, start_y, imnodes::CoordinateSystem::GridSpace);
                store = store.link_create_if_not_exists(start_node.clone(), end_node.clone());

                if first.is_none() {
                    first = Some(start_node.clone());
                }
            }
        }

        // This next part arranges the events that need space vertically, usually only places where events branch
        if let Some(first) = first {
            let (seen, _) = store.new_walk::<_, EventGraphEditor>(first, Some(&EventGraphEditor::default()));

            for s in seen {
                let node = store.get(s);
                if let Some((id, refs)) = node.1 {
                    if refs.len() >= 3 {
                        for _ in 0..refs.len() - 1 {
                            for (pos, end_node) in store
                                .clone()
                                .visit(*id)
                                .iter()
                                .skip(1)
                                .filter_map(|r| r.1)
                                .enumerate()
                            {
                                let ImVec2 { x: _, y } =
                                    id.get_position(imnodes::CoordinateSystem::GridSpace);

                                let start_y = y + (pos as f32) * 325.0;

                                let ImVec2 { x, y: _ } =
                                    end_node.get_position(imnodes::CoordinateSystem::GridSpace);
                                end_node.set_position(
                                    x,
                                    start_y,
                                    imnodes::CoordinateSystem::GridSpace,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Visitor<NodeId> for EventGraphEditor {
    fn visit(&self, _: &NodeId, _: &NodeId) -> bool {
        true
    }
}

impl Visitor<EventEditor> for EventGraphEditor {
    fn visit(&self, from: &EventEditor, to: &EventEditor) -> bool {
        println!("{} -> {}", from.on, to.on);
        true
    }

    fn visit_mut(&mut self, from: &EventEditor, to: &EventEditor) -> bool {
        if from.transitions.iter().find(|t| **t == to.on).is_some() {
            if let (Some((from, _, _, from_pin)), Some((to, to_pin, _, _))) =
                (self.graph_index.get(from), self.graph_index.get(to))
            {
                self.links.insert(Link {
                    start_node: *from,
                    end_node: *to,
                    start_pin: *from_pin,
                    end_pin: *to_pin,
                    craeated_from_snap: false,
                });
            }
        }
        true
    }
}