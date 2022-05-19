use atlier::system::{App, Attribute, Extension};
use imnodes::{editor, AttributeFlag, AttributeId, InputPinId, Link, LinkId, NodeId, OutputPinId};
use specs::{Entities, Join, ReadStorage, RunNow, System};
use std::collections::{HashMap, HashSet};

use super::{node_editor_graph::NodeEditorGraph, SectionAttributes};

pub struct NodeEditor {
    pub imnodes: imnodes::Context,
    pub imnode_editors: HashMap<u32, (imnodes::EditorContext, imnodes::IdentifierGenerator)>,
    pub nodes: HashMap<u32, Vec<NodeComponent>>,
    pub links: HashMap<u32, (HashSet<Link>, HashMap<LinkId, Link>)>,
}

#[derive(Clone)]
pub struct NodeComponent {
    title: String,
    node_id: NodeId,
    input_id: InputPinId,
    output_id: OutputPinId,
    attribute_id: AttributeId,
    attribute: Attribute,
}

impl NodeEditor {
    pub fn new() -> NodeEditor {
        NodeEditor {
            imnodes: imnodes::Context::new(),
            imnode_editors: HashMap::new(),
            nodes: HashMap::new(),
            links: HashMap::new(),
        }
    }
}

impl Extension for NodeEditor {
    fn extend_app_world(&mut self, world: &specs::World, ui: &imgui::Ui) {
        self.run_now(world);
        self.show_editor(ui);
    }
}

impl<'a> System<'a> for NodeEditor {
    type SystemData = (Entities<'a>, ReadStorage<'a, SectionAttributes>);

    /// This system initializes a node editor when it detects
    /// the attribute "enable node editor" has been set to true
    /// It will read all the attributes in the collection with the prefix node::
    /// and initialize the node_editor state
    /// When the attribute is set to false, this system will remove those resources from this
    /// system
    fn run(&mut self, (entities, attributes): Self::SystemData) {
        entities.join().for_each(|e| {
            if let Some(attributes) = attributes.get(e) {
                match attributes.is_attr_checkbox("enable node editor") {
                    Some(true) => match self.imnode_editors.get(&e.id()) {
                        None => {
                            let editor_context = self.imnodes.create_editor();
                            let mut idgen = editor_context.new_identifier_generator();

                            let mut nodes = vec![];

                            for attr in attributes
                                .clone_attrs()
                                .iter_mut()
                                .filter(|a| a.name().starts_with("node::"))
                            {
                                nodes.push(NodeComponent {
                                    title: attr.name()[6..].to_string(),
                                    node_id: idgen.next_node(),
                                    input_id: idgen.next_input_pin(),
                                    output_id: idgen.next_output_pin(),
                                    attribute_id: idgen.next_attribute(),
                                    attribute: attr.clone(),
                                });
                            }

                            self.nodes.insert(e.id(), nodes);
                            self.imnode_editors.insert(e.id(), (editor_context, idgen));
                            self.links.insert(e.id(), (HashSet::new(), HashMap::new()));
                        }
                        _ => (),
                    },
                    Some(false) => {
                        self.nodes.remove(&e.id());
                        self.imnode_editors.remove(&e.id());
                        self.links.remove(&e.id());
                    }
                    _ => (),
                }
            }
        })
    }
}

impl App for NodeEditor {
    fn name() -> &'static str {
        "Node Editor"
    }

    fn show_editor(&mut self, ui: &imgui::Ui) {
        use imgui::Condition;
        use imgui::Window;

        for (id, (context, idgen)) in self.imnode_editors.iter_mut() {
            if let Some(nodes) = self.nodes.get_mut(id) {
                Window::new(format!("Node editor {}", id))
                    .size([1920.0, 1080.0], Condition::Appearing)
                    .build(ui, || {
                        if ui.button("Rearrange") {
                            if let Some((links, _)) = self.links.get_mut(id) {
                                NodeEditorGraph::rearrange(links);
                            }
                        }

                        let detatch = context.push(AttributeFlag::EnableLinkDetachWithDragClick);

                        let outer_scope = editor(context, |mut editor_scope| {
                            editor_scope.add_mini_map(imnodes::MiniMapLocation::BottomRight);
                            nodes.iter_mut().for_each(|node_component| {
                                let NodeComponent {
                                    title,
                                    node_id,
                                    input_id,
                                    output_id,
                                    attribute_id,
                                    attribute,
                                } = node_component;

                                ui.set_next_item_width(130.0);
                                editor_scope.add_node(*node_id, |mut node_scope| {
                                    ui.set_next_item_width(130.0);
                                    node_scope.add_titlebar(|| {
                                        ui.text(title);
                                    });
                                    node_scope.attribute(*attribute_id, || {
                                        ui.set_next_item_width(130.0);
                                        attribute.edit(ui);
                                    });

                                    node_scope.add_input(
                                        *input_id,
                                        imnodes::PinShape::Circle,
                                        || {
                                            ui.set_next_item_width(130.0);
                                            ui.text("in");
                                        },
                                    );

                                    node_scope.add_output(
                                        *output_id,
                                        imnodes::PinShape::Circle,
                                        || {
                                            ui.set_next_item_width(130.0);
                                            ui.text("out");
                                        },
                                    );
                                });
                            });

                            if let Some((_, link_index)) = self.links.get(id) {
                                link_index.iter().for_each(|(link_id, link)| {
                                    editor_scope.add_link(*link_id, link.end_pin, link.start_pin);
                                });
                            }
                        });

                        if let Some(link) = outer_scope.links_created() {
                            if let Some((links, link_index)) = self.links.get_mut(id) {
                                if links.insert(link) {
                                    link_index.insert(idgen.next_link(), link);
                                }
                            }
                        }

                        if let Some(dropped) = outer_scope.get_dropped_link() {
                            if let Some((links, link_index)) = self.links.get_mut(id) {
                                if let Some(dropped_link) = link_index.get(&dropped) {
                                    links.remove(dropped_link);
                                    link_index.remove(&dropped);
                                }
                            }
                        }

                        detatch.pop();
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
