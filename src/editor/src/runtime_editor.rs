use std::fmt::Display;

use atlier::system::App;
use lifec::{RuntimeState, Runtime, Action, parse_flags, parse_variables};

use crate::event_editor::EventEditor;
use crate::event_graph_editor::EventGraphEditor;

#[derive(Default, Clone)]
pub struct RuntimeEditor<S>
where
    S: RuntimeState<State = S> + Default + Clone,
{
    count: usize,
    initial_str: String,
    events: Vec<EventEditor>,
    runtime: Runtime<S>,
    show_graph_editor: bool,
    graph_editor: Option<EventGraphEditor>,
}

impl<S> From<Runtime<S>> for RuntimeEditor<S>
where
    S: RuntimeState<State = S> + Default + Clone,
{
    fn from(state: Runtime<S>) -> Self {
        let events = state
            .get_listeners()
            .iter()
            .enumerate()
            .filter_map(|(id, l)| match (&l.action, &l.next) {
                (Action::Dispatch(msg), Some(transition)) => Some(EventEditor {
                    label: format!("Event {}", id),
                    on: l.event.to_string(),
                    dispatch: msg.to_string(),
                    call: String::default(),
                    transitions: vec![transition.to_string()],
                    flags: parse_flags(l.extensions.get_args()),
                    variales: parse_variables(l.extensions.get_args()),
                }),
                (Action::Call(call), _) => Some(EventEditor {
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
                    flags: parse_flags(l.extensions.get_args()),
                    variales: parse_variables(l.extensions.get_args()),
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

impl<S> App for RuntimeEditor<S>
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

        if let Some(window) = imgui::Window::new("Runtime Editor")
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
                    next.events.push(EventEditor {
                        label,
                        ..Default::default()
                    });
                }
            }
            ui.same_line();
            if ui.button("Compile") {
                let mut runtime_state = next.runtime.clone();
                let runtime_state = &mut runtime_state;
                runtime_state.reset_listeners(true);

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
                next.runtime = next.runtime.parse_event(&next.initial_str);
            }

            if ui.button("Process") {
                next.runtime = next.runtime.process();
            }
            
            // Display Current State of Runtime
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
                        let flags = parse_flags(l.extensions.get_args());
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

                        let env = parse_variables(l.extensions.get_args());
                        if env.len() > 0 {
                            ui.text(format!("Env:"));
                            for (key, value) in env {
                                ui.text(format!("{}: {}", key, value));
                            }
                        }
                    }
                }
            }

            if let Some(editor_context) = imnodes {
                if ui.checkbox("Open Graph Editor", &mut next.show_graph_editor) {
                    let mut graph_editor =EventGraphEditor {
                        events: next.events.clone(),
                        ..Default::default()
                    };

                    let mut idgen = editor_context.new_identifier_generator();
                    graph_editor.graph_contents(&mut idgen);
                    
                    next.graph_editor = Some(graph_editor);
                }

                if next.show_graph_editor {
                    if let Some(ref graph_editor) = next.graph_editor {
                        let mut next_graph_editor = graph_editor.clone();
                        imgui::Window::new("Graph Editor").size([1280.0, 720.0], imgui::Condition::Appearing).build(ui, || {
                            if let Some(updated_graph_editor) = EventGraphEditor::show(ui, &next_graph_editor, Some(editor_context)) {
                                next_graph_editor = updated_graph_editor;
                                next.events = next_graph_editor.clone().events;
                            }
                        });

                        next.events = next_graph_editor.clone().events;
                        next.graph_editor = Some(next_graph_editor);
                    }
                }
            } else {
                let mut next_events = next.events.clone();
                for (i, e) in next.events.iter().enumerate() {
                    if let Some(next_e) = EventEditor::show(ui, e, None) {
                        if next_e != *e {
                            next_events[i] = next_e.clone()
                        }
                    }
                }
                next.events = next_events;
            }

            window.end()
        }

        Some(next)
    }
}
