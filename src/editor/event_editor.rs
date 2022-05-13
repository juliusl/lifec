use std::collections::BTreeMap;

use atlier::system::App;
use imnodes::IdentifierGenerator;

#[derive(Clone, Default, PartialEq, Hash, Eq)]
pub struct EventEditor {
    pub label: String,
    pub on: String,
    pub dispatch: String,
    pub call: String,
    pub transitions: Vec<String>,
    pub flags: BTreeMap<String, String>,
    pub variales: BTreeMap<String, String>,
}

impl App for EventEditor {
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
