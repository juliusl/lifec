use crate::{Runtime, RuntimeState, Action};
use imnodes::{ Link, NodeId, LinkId, AttributeFlag, IdentifierGenerator };
use std::collections::HashSet;
use atlier::prelude::*;
use std::fmt::Display;
use std::collections::HashMap;

#[derive(Default, Clone)]
pub struct EditorRuntime<S>
where
    S: RuntimeState<State = S> + Default + Clone
{
    count: usize,
    events: Vec<EditorEvent>,
    state: Runtime<S>,
    links: HashSet<Link>,
}

#[derive(Clone, Default, PartialEq)]
pub struct EditorEvent {
    label: String,
    on: String,
    dispatch: String,
    transition: String,
}

impl<S> From<Runtime<S>> for EditorRuntime<S> 
where
    S: RuntimeState<State = S> + Default + Clone {
        fn from(state: Runtime<S>) -> Self {
            let events = state.listeners.iter().enumerate().filter_map(|(id, l)| {
                if let (Action::Dispatch(msg), Some(transition)) = (&l.action, &l.next) {
                    Some(EditorEvent {
                        label: format!("Event {}", id),
                        on: l.event.to_string(),
                        dispatch: msg.to_string(),
                        transition: transition.to_string(),
                    })
                } else {
                    None
                }
            }).collect();

            Self { state, events, ..Default::default() }
        }
    }

impl<S> App for EditorRuntime<S>
where
    S: RuntimeState<State = S> + Default + Clone + Display,
{
    fn show(
        ui: &imgui::Ui,
        state: &Self,
        imnodes: Option<&mut imnodes::EditorContext>,
    ) -> Option<Self> {
        let mut next = state.clone();

        if let Some(window) = imgui::Window::new("runtime")
            .size([800.0, 600.0], imgui::Condition::FirstUseEver)
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
                    let dispatch = e.dispatch;
                    let transition = e.transition;
                    runtime_state.on(&on).dispatch(&dispatch, &transition);
                }
                next.state = runtime_state.parse_event("{ setup;; }");
            }
            ui.same_line();
            if ui.button("Process") {
                next.state = next.state.process();
            }

            if let Some(state) = next.state.current() {
                ui.same_line();
                ui.text(format!("{}", state));
            }

            let mut next_events = next.events.clone();

            if let Some(editor_context) = imnodes {
                let mut node_index = HashMap::<NodeId, EditorEvent>::new();
                let mut link_index = HashMap::<LinkId, Link>::new();
                let detach = editor_context.push(AttributeFlag::EnableLinkDetachWithDragClick);

                let idgen = &mut editor_context.new_identifier_generator();
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

                    for e in next.links {
                        let link_id = idgen.next_link();
                        scope.add_link(link_id, e.end_pin, e.start_pin);
                        link_index.insert(link_id, e.clone());

                        if let (Some(start), Some(end)) =
                            (node_index.get(&e.start_node), node_index.get(&e.end_node))
                        {
                            if let Some(start_pos) = next_events.iter().position(|e| *e == *start) {
                                let mut updated_start = start.clone();
                                updated_start.transition = end.transition.to_owned();
                                next_events[start_pos] = updated_start;
                            }
                        }
                    }
                });

                let mut previous = state.links.clone();
                let mut next_links = HashSet::new();
                if let Some(link) = outer.links_created() {
                    next_links.insert(link);
                }

                if let Some(destroyed) = outer.get_dropped_link() {
                    if let Some(link) = link_index.get(&destroyed) {
                        previous.remove(link);
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

impl App for EditorEvent {
    fn show(ui: &imgui::Ui, state: &Self, _: Option<&mut imnodes::EditorContext>) -> Option<Self> {
        let mut next = state.clone();
        if imgui::CollapsingHeader::new(&state.label).begin(ui) {
            let group = ui.begin_group();
            ui.input_text(format!("{} on", &state.label), &mut next.on)
                .build();
            ui.input_text(format!("{} dispatch", &state.label), &mut next.dispatch)
                .build();
            ui.input_text(format!("{} transition", &state.label), &mut next.transition)
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
            ui.set_next_item_width(130.0);
            ui.input_text("on", &mut next.on).build();
        });

        node_scope.attribute(idgen.next_attribute(), || {
            ui.set_next_item_width(130.0);
            ui.input_text("dispatch", &mut next.dispatch).build();
        });

        node_scope.add_output(idgen.next_output_pin(), imnodes::PinShape::Circle, || {
            ui.set_next_item_width(130.0);
            ui.label_text("transition", &next.transition);
        });

        Some(next)
    }
}
