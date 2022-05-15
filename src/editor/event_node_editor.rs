// use atlier::system::App;
// use imnodes::{AttributeId, EditorContext, InputPinId, NodeId, OutputPinId};
// use specs::{Component, DenseVecStorage, Entities, Join, System, WriteStorage};

// use super::runtime_editor::EventComponent;

// #[derive(Clone)]
// pub struct EventNodeState {
//     title: String,
//     node_id: NodeId,
//     input_pin_id: InputPinId,
//     attribute_id: AttributeId,
//     output_pin_id: OutputPinId,
// }

// impl Component for EventNodeState {
//     type Storage = DenseVecStorage<Self>;
// }

// pub struct EventNodeEditor {
//     editor_context: EditorContext,
//     id_gen: imnodes::IdentifierGenerator,
// }

// impl<'a> System<'a> for EventNodeEditor {
//     type SystemData = (Entities<'a>, WriteStorage<'a, EventNodeState>);

//     fn run(&mut self, (entities, mut event_node_state): Self::SystemData) {
//         for e in entities.join() {
//             if let None = event_node_state.get(e) {
//                 match event_node_state.insert(
//                     e,
//                     EventNodeState {
//                         title: format!("Event {}", e.id()),
//                         node_id: self.id_gen.next_node(),
//                         input_pin_id: self.id_gen.next_input_pin(),
//                         attribute_id: self.id_gen.next_attribute(),
//                         output_pin_id: self.id_gen.next_output_pin(),
//                     },
//                 ) {
//                     Ok(_) => println!("Added node"),
//                     Err(e) => eprintln!("{}", e),
//                 }
//             }
//         }
//     }
// }

// impl App for EventNodeEditor {
//     fn title() -> &'static str {
//         "Event"
//     }

//     fn show_editor(&mut self, ui: &imgui::Ui) {
//         let editor_context = &mut self.editor_context;
//         imnodes::editor(editor_context, |mut editor_scope| {
//             for (node_state, mut event_component) in nodes.clone() {
//                 let EventNodeState {
//                     title,
//                     node_id,
//                     input_pin_id,
//                     attribute_id,
//                     output_pin_id,
//                 } = node_state;
//                 editor_scope.add_node(node_id, |mut node_scope| {
//                     node_scope.add_titlebar(|| ui.text(&title));

//                     let EventComponent {
//                         on, call, dispatch, ..
//                     } = &mut event_component;
//                     node_scope.add_input(input_pin_id, imnodes::PinShape::Circle, || {
//                         ui.set_next_item_width(200.0);
//                         ui.input_text("on", on).build();
//                     });

//                     if call.is_empty() {
//                         node_scope.attribute(attribute_id, || {
//                             ui.set_next_item_width(200.0);

//                             ui.input_text("dispatch", dispatch).build();
//                         });
//                     } else {
//                         node_scope.attribute(attribute_id, || {
//                             ui.set_next_item_width(200.0);
//                             ui.input_text("call", call).build();
//                         });
//                     }

//                     let EventComponent { transitions, .. } = event_component;
//                     node_scope.add_output(output_pin_id, imnodes::PinShape::Circle, || {
//                         ui.set_next_item_width(200.0);
//                         for t in transitions {
//                             ui.text(t);
//                         }
//                     });
//                 });
//             }
//         });
//     }
// }
