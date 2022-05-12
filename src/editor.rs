use crate::{Action, Runtime, RuntimeState, WithArgs};
use imnodes::{AttributeFlag, IdentifierGenerator, Link, LinkId, NodeId};
use imnodes::{AttributeId, ImVec2, InputPinId, OutputPinId};
use knot::store::{Store, Visitor};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Display;

pub use atlier::system::App;

#[derive(Default, Clone)]
pub struct EditorRuntime<S>
where
    S: RuntimeState<State = S> + Default + Clone,
{
    count: usize,
    events: Vec<EditorEvent>,
    runtime: Runtime<S>,
    links: HashSet<Link>,
    initial_str: String,
}

#[derive(Clone, Default, PartialEq, Hash, Eq)]
pub struct EditorEvent {
    label: String,
    on: String,
    dispatch: String,
    call: String,
    transitions: Vec<String>,
}

impl<S> From<Runtime<S>> for EditorRuntime<S>
where
    S: RuntimeState<State = S> + Default + Clone,
{
    fn from(state: Runtime<S>) -> Self {
        let events = state
            .listeners
            .iter()
            .enumerate()
            .filter_map(|(id, l)| match (&l.action, &l.next) {
                (Action::Dispatch(msg), Some(transition)) => Some(EditorEvent {
                    label: format!("Event {}", id),
                    on: l.event.to_string(),
                    dispatch: msg.to_string(),
                    call: String::default(),
                    transitions: vec![transition.to_string()],
                }),
                (Action::Call(call), _) => Some(EditorEvent {
                    label: format!("Event {}", id),
                    on: l.event.to_string(),
                    call: call.to_string(),
                    dispatch: String::default(),
                    transitions: l
                        .extensions
                        .tests
                        .iter()
                        .map(|(_, t)| t.to_owned())
                        .collect(),
                }),
                _ => None,
            })
            .collect();

        let mut next = Self {
            runtime: state,
            events,
            ..Default::default()
        };

        next.count = next.events.len();

        next
    }
}

impl<S> App for EditorRuntime<S>
where
    S: RuntimeState<State = S> + Default + Clone + Display,
{
    fn title() -> &'static str {
        "Event Editor"
    }

    fn show(
        ui: &imgui::Ui,
        state: &Self,
        imnodes: Option<&mut imnodes::EditorContext>,
    ) -> Option<Self> {
        let mut next = state.clone();

        if let Some(window) = imgui::Window::new("runtime")
            .size([1280.0, 1080.0], imgui::Condition::FirstUseEver)
            .begin(ui)
        {
            let mut count = next.count.try_into().unwrap();
            imgui::InputInt::new(ui, "number of events", &mut count).build();
            next.count = count.try_into().unwrap();

            if ui.button("Create") {
                next.events.clear();
                for i in 0..count {
                    let i: usize = i.try_into().unwrap();
                    let label = format!("Event {}", i);
                    next.events.push(EditorEvent {
                        label,
                        ..Default::default()
                    });
                }
            }
            ui.same_line();
            if ui.button("Compile") {
                let mut runtime_state = Runtime::<S>::default();
                let runtime_state = &mut runtime_state;
                for e in next.events.iter().cloned() {
                    let on = e.on;

                    match (e.dispatch.as_str(), e.call.as_str()) {
                        (dispatch, "") => {
                            let transition = e.transitions.join(" ");

                            runtime_state
                                .on(&on)
                                .dispatch(&dispatch, &transition.as_str());
                        }
                        ("", call) => {
                            runtime_state.on(&on).call(&call);
                        }
                        _ => {}
                    }
                }
                next.runtime = runtime_state.parse_event("{ setup;; }");
            }

            ui.set_next_item_width(120.0);
            ui.input_text("Initial Event", &mut next.initial_str)
                .build();
            ui.same_line();
            if ui.button("Parse Event") {
                next.runtime = next.runtime.parse_event("{ setup;; }");
            }

            if ui.button("Process") {
                next.runtime = next.runtime.process();
            }
            if let (Some(state), context) = (next.runtime.current(), next.runtime.context()) {
                ui.same_line();
                ui.text(format!("Current Event: {} State: {}", context, state));

                if let Some(l) = next.runtime.next_listener() {
                    match l.action {
                        Action::Call(call) => {
                            ui.text(format!("Call: {}", call));

                            if l.extensions.tests.len() > 0 {
                                ui.text("Known Transitions:");
                                l.extensions.tests.iter().map(|(_, t)| t).for_each(|t| {
                                    ui.text(format!("- {}", t));
                                });
                            }
                        }
                        Action::Dispatch(dispatch) => {
                            ui.text(format!("Dispatch: {}", dispatch));
                            if let Some(next) = l.next {
                                ui.text(format!("Next: {}", next));
                            }
                        }
                        Action::Thunk(_) | Action::NoOp => {}
                    }

                    if l.extensions.args.len() > 0 {
                        let args = WithArgs::<S> {
                            state: state.clone(),
                            args: l.extensions.args,
                        };

                        let flags = args.parse_flags();
                        if flags.len() > 0 {
                            ui.text(format!("Flags:"));
                            for (key, value) in flags {
                                if key.len() == 1 {
                                    ui.text(format!("{}: {}", key, value));
                                } else {
                                    ui.text(format!("{}: {}", key, value));
                                }
                            }
                        }

                        let env = args.parse_variables(); 
                        if env.len() > 0 {
                            ui.text(format!("Env:"));
                            for (key, value) in env {
                                ui.text(format!("{}: {}", key, value));
                            }
                        }
                    }
                }
            }

            let mut next_events = next.events.clone();

            if let Some(editor_context) = imnodes {
                let mut node_index = HashMap::<NodeId, EditorEvent>::new();
                let mut link_index = HashMap::<LinkId, Link>::new();
                let detach = editor_context.push(AttributeFlag::EnableLinkDetachWithDragClick);
                let idgen = &mut editor_context.new_identifier_generator();
                let mut previous = state.links.clone();

                if ui.button("re-arrange") {
                    let mut store = Store::<NodeId>::default();
                    let mut first: Option<NodeId> = None;

                    // This first part arranges the events horizontally
                    for _ in 0..previous.len() {
                        for Link {
                            start_node,
                            end_node,
                            ..
                        } in previous.clone().iter()
                        {
                            let ImVec2 { x, y } =
                                start_node.get_position(imnodes::CoordinateSystem::GridSpace);
                            let start_x = x + 400.0;
                            let start_y = y + 75.0;

                            end_node.set_position(
                                start_x,
                                start_y,
                                imnodes::CoordinateSystem::GridSpace,
                            );
                            store = store
                                .link_create_if_not_exists(start_node.clone(), end_node.clone());

                            if first.is_none() {
                                first = Some(start_node.clone());
                            }
                        }
                    }

                    // This next part arranges the events that need space vertically, usually only places where events branch
                    if let Some(first) = first {
                        let (seen, _) =
                            store.new_walk::<_, Printer>(first, Some(&Printer::default()));

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
                                            let ImVec2 { x: _, y } = id
                                                .get_position(imnodes::CoordinateSystem::GridSpace);

                                            let start_y = y + (pos as f32) * 325.0;

                                            let ImVec2 { x, y: _ } = end_node
                                                .get_position(imnodes::CoordinateSystem::GridSpace);
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

                if ui.button("graph state") {
                    let idgen = &mut editor_context.new_identifier_generator();
                    let mut store = Store::<EditorEvent>::default();
                    let mut graph_index = HashMap::<
                        EditorEvent,
                        (NodeId, InputPinId, AttributeId, OutputPinId),
                    >::new();
                    let next_events = next.events.clone();
                    for e in &next_events {
                        store = store.node(e.clone());

                        if !&e.transitions.is_empty() {
                            for to in
                                next.events.clone().iter().filter(|o| {
                                    e.transitions.iter().find(|p| **p == o.on).is_some()
                                })
                            {
                                store = store.link_create_if_not_exists(e.clone(), to.to_owned());
                            }
                        }

                        graph_index.insert(
                            e.clone(),
                            (
                                idgen.next_node(),
                                idgen.next_input_pin(),
                                idgen.next_attribute(),
                                idgen.next_output_pin(),
                            ),
                        );
                    }

                    let printer = &mut Printer::default();
                    store.new_walk_mut(next_events[0].clone(), Some(printer));
                    previous.clear();
                    for s in &printer.state {
                        if let (Some((from, _, _, from_pin)), Some((to, to_pin, _, _))) =
                            (graph_index.get(&s.0), graph_index.get(&s.1))
                        {
                            previous.insert(Link {
                                start_node: *from,
                                end_node: *to,
                                start_pin: *from_pin,
                                end_pin: *to_pin,
                                craeated_from_snap: false,
                            });
                        }

                        println!("{} -> {}", &s.0.on, &s.1.on);
                    }
                }

                let outer = imnodes::editor(editor_context, |mut scope| {
                    let mut i = 0;
                    for e in next.events {
                        let node_id = idgen.next_node();
                        scope.add_node(node_id, |node| {
                            if let Some(next_e) = EditorEvent::show_node(ui, &e, node, idgen) {
                                node_index.insert(node_id, next_e.clone());
                                next_events[i] = next_e;
                            }
                        });
                        i += 1;
                    }

                    for link in &previous {
                        let link_id = idgen.next_link();
                        scope.add_link(link_id, link.end_pin, link.start_pin);
                        link_index.insert(link_id, link.clone());
                    }
                });

                let mut next_links = HashSet::new();
                if let Some(link) = outer.links_created() {
                    next_links.insert(link);

                    if let (Some(start), Some(end)) = (
                        node_index.get(&link.start_node),
                        node_index.get(&link.end_node),
                    ) {
                        if let Some(start_pos) = next_events.iter().position(|e| *e == *start) {
                            let mut updated_start = start.clone();
                            updated_start.transitions.push(end.on.to_owned());
                            next_events[start_pos] = updated_start;
                        }
                    }
                }

                if let Some(destroyed) = outer.get_dropped_link() {
                    if let Some(link) = link_index.get(&destroyed) {
                        previous.remove(link);

                        let start_node_id = link.start_node;
                        if let Some(start) = node_index.get(&start_node_id) {
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
                next.links = previous.union(&next_links).cloned().collect();
            } else {
                for (i, e) in next.events.iter().enumerate() {
                    if let Some(next_e) = EditorEvent::show(ui, e, None) {
                        if next_e != *e {
                            next_events[i] = next_e.clone()
                        }
                    }
                }
            }

            next.events = next_events;
            window.end()
        }

        Some(next)
    }
}

#[derive(Default)]
struct Printer {
    state: HashSet<(EditorEvent, EditorEvent)>,
}

impl Visitor<NodeId> for Printer {
    fn visit(&self, from: &NodeId, to: &NodeId) -> bool {
        true
    }
}

impl Visitor<EditorEvent> for Printer {
    fn visit(&self, from: &EditorEvent, to: &EditorEvent) -> bool {
        println!("{} -> {}", from.on, to.on);
        true
    }

    fn visit_mut(&mut self, from: &EditorEvent, to: &EditorEvent) -> bool {
        if from.transitions.iter().find(|t| **t == to.on).is_some() {
            self.state.insert((from.clone(), to.clone()));
        }
        true
    }
}

impl App for EditorEvent {
    fn title() -> &'static str {
        "Edit Event"
    }

    fn show(ui: &imgui::Ui, state: &Self, _: Option<&mut imnodes::EditorContext>) -> Option<Self> {
        let mut next = state.clone();
        if imgui::CollapsingHeader::new(&state.label).begin(ui) {
            let group = ui.begin_group();
            ui.input_text(format!("{} on", &state.label), &mut next.on)
                .build();
            ui.input_text(format!("{} dispatch", &state.label), &mut next.dispatch)
                .build();
            ui.input_text(
                format!("{} transition", &state.label),
                &mut next.transitions.join("\n"),
            )
            .build();
            group.end();
        }

        Some(next)
    }

    fn show_node(
        ui: &imgui::Ui,
        state: &Self,
        mut node_scope: imnodes::NodeScope,
        idgen: &mut IdentifierGenerator,
    ) -> Option<Self> {
        let mut next = state.clone();
        node_scope.add_titlebar(|| ui.text(&next.label));

        node_scope.add_input(idgen.next_input_pin(), imnodes::PinShape::Circle, || {
            ui.set_next_item_width(200.0);
            ui.input_text("on", &mut next.on).build();
        });

        node_scope.attribute(idgen.next_attribute(), || {
            ui.set_next_item_width(200.0);

            if next.dispatch.is_empty() {
                ui.input_text("call", &mut next.call).build();
            } else {
                ui.input_text("dispatch", &mut next.dispatch).build();
            }
        });

        node_scope.add_output(idgen.next_output_pin(), imnodes::PinShape::Circle, || {
            ui.set_next_item_width(200.0);
            for t in &next.transitions {
                ui.text(t);
            }
        });

        Some(next)
    }
}
