use std::collections::HashMap;

use atlier::system::App;
use imnodes::{AttributeId, EditorScope, InputPinId, NodeId, OutputPinId};
use specs::{Component, DenseVecStorage, Entities, System, WriteStorage, Join};

use crate::{Runtime, RuntimeState};

#[derive(Default)]
pub struct RuntimeEditor<S>
where
    S: RuntimeState,
{
    count: usize,
    initial_str: String,
    runtime: Runtime<S>,
    imnodes: Option<imnodes::Context>,
    // events: Vec<EventEditor>,
}

impl<'a, S> System<'a> for RuntimeEditor<S>
where
    S: RuntimeState + Component,
{
    type SystemData = (Entities<'a>, WriteStorage<'a, S>);

    fn run(&mut self, _: Self::SystemData) {
        if let None = self.imnodes {
            self.imnodes = Some(imnodes::Context::new());
        }
    }
}

impl<S> App for RuntimeEditor<S>
where
    S: RuntimeState + Component,
{
    type State = S;

    fn title() -> &'static str {
        "Runtime Editor"
    }

    fn show_editor(&mut self, _: &imgui::Ui, _: &mut Self::State) {
        todo!()
    }
}

struct EventNode {
    editor_scope: HashMap<u32, EditorScope>,
    id_gen: HashMap<u32, imnodes::IdentifierGenerator>,
}

impl<'a> System<'a> for EventNode {
    type SystemData = (
            Entities<'a>, 
            WriteStorage<'a, EventNodeState>
    );

    fn run(&mut self, mut data: Self::SystemData) {
        let entities = &data.0;
        for e in (entities).join() {
            // if let  None = self.editor_scope.get(&e.id()) {
            //     self.editor_scope.insert()
            // }

            if let Some(EventNodeState { title: None, node_id: None, input_pin_id: None, attribute_id: None, output_pin_id: None, .. }) = data.1.get(e) {
                if let Some(node_state) = data.1.get_mut(e) {
                    let id: u32 = e.id();
                    if let Some(id_gen) = self.id_gen.get_mut(&id) {
                        node_state.title = Some(format!("Node: {}", e.id()));
                        node_state.node_id = Some(id_gen.next_node());
                        node_state.input_pin_id = Some(id_gen.next_input_pin());
                        node_state.attribute_id = Some(id_gen.next_attribute());
                        node_state.output_pin_id = Some(id_gen.next_output_pin());
                    }

                }
            }
        }
    }
}

#[derive(Clone)]
struct EventNodeState {
    parent_id: Option<u32>,
    title: Option<String>,
    node_id: Option<NodeId>,
    input_pin_id: Option<InputPinId>,
    attribute_id: Option<AttributeId>,
    output_pin_id: Option<OutputPinId>,
    on: Option<String>,
    dispatch: Option<String>,
    call: Option<String>,
    transitions: Vec<String>,
}

impl Component for EventNodeState {
    type Storage = DenseVecStorage<Self>;
}

impl App for EventNode {
    type State = EventNodeState;

    fn title() -> &'static str {
        "Event"
    }

    fn show_editor(&mut self, ui: &imgui::Ui, node_state: &mut Self::State) {
        if let EventNodeState {
            parent_id: Some(id),
            title: Some(title),
            node_id: Some(node_id),
            input_pin_id: Some(input_pin_id),
            attribute_id: Some(attribute_id),
            output_pin_id: Some(output_pin_id),
            ..
        } = node_state.clone()
        { 
            if let Some(editor_scope) = self.editor_scope.get_mut(&id) {
                editor_scope.add_node(node_id, |mut node_scope| {
                    node_scope.add_titlebar(|| ui.text(&title));
    
                    if let EventNodeState { on: Some(on), .. } = node_state {
                        node_scope.add_input(input_pin_id, imnodes::PinShape::Circle, || {
                            ui.set_next_item_width(200.0);
                            ui.input_text("on", on).build();
                        });
                    }
    
                    match node_state {
                        EventNodeState {
                            call: Some(call), ..
                        } => {
                            node_scope.attribute(attribute_id, || {
                                ui.set_next_item_width(200.0);
                                ui.input_text("call", call).build();
                            });
                        }
                        EventNodeState {
                            dispatch: Some(dispatch),
                            ..
                        } => {
                            node_scope.attribute(attribute_id, || {
                                ui.set_next_item_width(200.0);
    
                                ui.input_text("dispatch", dispatch).build();
                            });
                        }
                        _ => {}
                    }
    
                    let EventNodeState { transitions, .. } = node_state;
    
                    node_scope.add_output(output_pin_id, imnodes::PinShape::Circle, || {
                        ui.set_next_item_width(200.0);
                        for t in transitions {
                            ui.text(t);
                        }
                    });
                });
            }
        }
    }
}

// impl<S> RuntimeEditor<S>
// where
//     S: RuntimeState<State = S> + Send + Sync + Any + Sized + Display,
// {
//     fn open_node_editor(ui: &imgui::Ui, mut next: Self, mut editor_context: EditorContext) {
//         // if ui.checkbox("Open Graph Editor", &mut next.show_graph_editor) {
//         //     let mut graph_editor = EventGraphEditor {
//         //         events: next.events.clone(),
//         //         ..Default::default()
//         //     };

//         //     let mut idgen = editor_context.new_identifier_generator();
//         //     graph_editor.graph_contents(&mut idgen);

//         //     next.graph_editor = Some(graph_editor);
//         // }

//         // if next.show_graph_editor {
//         //     if let Some(ref graph_editor) = next.graph_editor {
//         //         let mut next_graph_editor = graph_editor.clone();
//         //         imgui::Window::new("Graph Editor")
//         //             .size([1280.0, 720.0], imgui::Condition::Appearing)
//         //             .build(ui, || {
//         //                 if let Some(updated_graph_editor) =
//         //                     EventGraphEditor::show(ui, &next_graph_editor, &mut editor_context)
//         //                 {
//         //                     next_graph_editor = updated_graph_editor;
//         //                     next.events = next_graph_editor.clone().events;
//         //                 }
//         //             });

//         //         next.events = next_graph_editor.clone().events;
//         //         next.graph_editor = Some(next_graph_editor);
//         //     }
//         // }
//     }
// }

// impl<S> From<Runtime<S>> for RuntimeEditor<S>
// where
//     S: RuntimeState<State = S> + Send + Sync + Any + Sized + Display,
// {
//     fn from(state: Runtime<S>) -> Self {
//         let events = state
//             .get_listeners()
//             .iter()
//             .enumerate()
//             .filter_map(|(id, l)| match (&l.action, &l.next) {
//                 (Action::Dispatch(msg), Some(transition)) => Some(EventEditor {
//                     label: format!("Event {}", id),
//                     on: l.event.to_string(),
//                     dispatch: msg.to_string(),
//                     call: String::default(),
//                     transitions: vec![transition.to_string()],
//                     flags: parse_flags(l.extensions.get_args()),
//                     variales: parse_variables(l.extensions.get_args()),
//                 }),
//                 (Action::Call(call), _) => Some(EventEditor {
//                     label: format!("Event {}", id),
//                     on: l.event.to_string(),
//                     call: call.to_string(),
//                     dispatch: String::default(),
//                     transitions: l
//                         .extensions
//                         .tests
//                         .iter()
//                         .map(|(_, t)| t.to_owned())
//                         .collect(),
//                     flags: parse_flags(l.extensions.get_args()),
//                     variales: parse_variables(l.extensions.get_args()),
//                 }),
//                 _ => None,
//             })
//             .collect();

//         let mut next = Self {
//             runtime: state,
//             events,
//             ..Default::default()
//         };

//         next.count = next.events.len();

//         next
//     }
// }

// impl<S> App for RuntimeEditor<S>
// where
//     S: RuntimeState<State = S> + Send + Sync + Any + Sized + Display,
// {
//     fn title() -> &'static str {
//         "Runtime Editor"
//     }

//     // fn open(
//     //     ui: &imgui::Ui,
//     //     editor: &Self,
//     //     imnodes: &mut imnodes::Context,
//     // ) -> (Option<Self>, HashMap<String, EditorContext>) {
//     //     let mut graph_editors = HashMap::<String, EditorContext>::new();
//     //     let mut next_editor = editor.clone();
//     //     let mut updated = false;

//     //     for (name, Extension::<S>{ open, ..}) in &editor.extensions {
//     //         match open(ui, &editor, imnodes) {
//     //             (Some(e), Some(node)) => {
//     //                 graph_editors.insert(name.to_string(), node);
//     //                 next_editor = e;
//     //                 updated = true;
//     //             }
//     //             (Some(e), None) => {
//     //                 next_editor = e;
//     //                 updated = true;
//     //             }
//     //             _ => (),
//     //         }
//     //     }

//     //     if updated {
//     //         (
//     //             Some(next_editor),
//     //             graph_editors,
//     //         )
//     //     } else {
//     //         (None, graph_editors)
//     //     }
//     // }

//     // fn show(ui: &imgui::Ui, state: &Self, imnodes: &mut HashMap<String, EditorContext>) -> Option<Self> {
//     //     let mut next = state.clone();

//     //     if let Some(window) = imgui::Window::new("Runtime Editor")
//     //         .size([1280.0, 1080.0], imgui::Condition::FirstUseEver)
//     //         .begin(ui)
//     //     {
//     //         let extensions = &state.extensions;

//     //         for (name, Extension { edit, .. }) in extensions {
//     //             if CollapsingHeader::new(name.as_str()).begin(ui) {
//     //                 if let Some(next_state) = edit(ui, &next, imnodes.get_mut(name)) {
//     //                     next = next_state;
//     //                 }
//     //             }
//     //         }

//     //         let mut count = next.count.try_into().unwrap();
//     //         imgui::InputInt::new(ui, "number of events", &mut count).build();
//     //         next.count = count.try_into().unwrap();

//     //         if ui.button("Create") {
//     //             next.events.clear();
//     //             for i in 0..count {
//     //                 let i: usize = i.try_into().unwrap();
//     //                 let label = format!("Event {}", i);
//     //                 next.events.push(EventEditor {
//     //                     label,
//     //                     ..Default::default()
//     //                 });
//     //             }
//     //         }
//     //         ui.same_line();
//     //         if ui.button("Compile") {
//     //             let mut runtime_state = next.runtime.clone();
//     //             let runtime_state = &mut runtime_state;
//     //             runtime_state.reset_listeners(true);

//     //             for e in next.events.iter().cloned() {
//     //                 let on = e.on;

//     //                 match (e.dispatch.as_str(), e.call.as_str()) {
//     //                     (dispatch, "") => {
//     //                         let transition = e.transitions.join(" ");

//     //                         runtime_state
//     //                             .on(&on)
//     //                             .dispatch(&dispatch, &transition.as_str());
//     //                     }
//     //                     ("", call) => {
//     //                         runtime_state.on(&on).call(&call);
//     //                     }
//     //                     _ => {}
//     //                 }
//     //             }
//     //             next.runtime = runtime_state.parse_event("{ setup;; }");
//     //         }

//     //         ui.set_next_item_width(120.0);
//     //         ui.input_text("Initial Event", &mut next.initial_str)
//     //             .build();
//     //         ui.same_line();
//     //         if ui.button("Parse Event") {
//     //             next.runtime = next.runtime.parse_event(&next.initial_str);
//     //         }

//     //         if ui.button("Process") {
//     //             next.runtime = next.runtime.process();
//     //         }

//     //         // Display Current State of Runtime
//     //         if let (Some(state), context) = (next.runtime.current(), next.runtime.context()) {
//     //             ui.same_line();
//     //             ui.text(format!("Current Event: {} State: {}", context, state));

//     //             if let Some(l) = next.runtime.next_listener() {
//     //                 match l.action {
//     //                     Action::Call(call) => {
//     //                         ui.text(format!("Call: {}", call));

//     //                         if l.extensions.tests.len() > 0 {
//     //                             ui.text("Known Transitions:");
//     //                             l.extensions.tests.iter().map(|(_, t)| t).for_each(|t| {
//     //                                 ui.text(format!("- {}", t));
//     //                             });
//     //                         }
//     //                     }
//     //                     Action::Dispatch(dispatch) => {
//     //                         ui.text(format!("Dispatch: {}", dispatch));
//     //                         if let Some(next) = l.next {
//     //                             ui.text(format!("Next: {}", next));
//     //                         }
//     //                     }
//     //                     Action::Thunk(_) | Action::NoOp => {}
//     //                 }

//     //                 if l.extensions.args.len() > 0 {
//     //                     let flags = parse_flags(l.extensions.get_args());
//     //                     if flags.len() > 0 {
//     //                         ui.text(format!("Flags:"));
//     //                         for (key, value) in flags {
//     //                             if key.len() == 1 {
//     //                                 ui.text(format!("{}: {}", key, value));
//     //                             } else {
//     //                                 ui.text(format!("{}: {}", key, value));
//     //                             }
//     //                         }
//     //                     }

//     //                     let env = parse_variables(l.extensions.get_args());
//     //                     if env.len() > 0 {
//     //                         ui.text(format!("Env:"));
//     //                         for (key, value) in env {
//     //                             ui.text(format!("{}: {}", key, value));
//     //                         }
//     //                     }
//     //                 }
//     //             }
//     //         }

//     //         if CollapsingHeader::new("Edit").begin(ui) {}

//     //         window.end()
//     //     }

//     //     Some(next)
//     // }
// }
