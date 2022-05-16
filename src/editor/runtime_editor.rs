use atlier::system::App;
use imgui::{Window, CollapsingHeader};
use imnodes::ColorStyle;
use specs::{Component, Entities, Join, ReadStorage, System, WriteStorage};
use std::collections::BTreeMap;

use crate::{Runtime, RuntimeState, Action};

use super::{section::Section, EventComponent};

pub struct RuntimeEditor<S>
where
    S: RuntimeState,
{
    pub runtime: Runtime<S>,
    events: Vec<EventComponent>,
    sections: BTreeMap<u32, Section<S>>,
}

impl<S: RuntimeState> From<Runtime<S>> for RuntimeEditor<S> {
    fn from(runtime: Runtime<S>) -> Self {
        let events = runtime
            .get_listeners()
            .iter()
            .enumerate()
            .filter_map(|(id, l)| match (&l.action, &l.next) {
                (Action::Dispatch(msg), Some(transition)) => Some(EventComponent {
                    label: format!("Event {}", id),
                    on: l.event.to_string(),
                    dispatch: msg.to_string(),
                    call: String::default(),
                    transitions: vec![transition.to_string()],
                    // flags: parse_flags(l.extensions.get_args()),
                    // variales: parse_variables(l.extensions.get_args()),
                }),
                (Action::Call(call), _) => Some(EventComponent {
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
                //     flags: parse_flags(l.extensions.get_args()),
                //     variales: parse_variables(l.extensions.get_args()),
                }),
                _ => None,
            })
            .collect();

        Self { runtime, events, sections: BTreeMap::new() }
    }
}

impl<'a, S> System<'a> for RuntimeEditor<S>
where
    S: RuntimeState + Component,
{
    type SystemData = (Entities<'a>, ReadStorage<'a, Section<S>>);

    fn run(&mut self, (entities, read_sections): Self::SystemData) {
        for (e, s) in (&entities, &read_sections).join() {
            match self.sections.get(&e.id()) {
                None => {
                    self.sections.insert(e.id(), s.clone());
                }
                Some(Section {  enable_app_systems, state, .. }) => {
                    if *enable_app_systems {
                        let state = state.merge_with(&s.state);
                        self.sections.insert(e.id(), {
                            let mut s = s.clone();
                            s.state = state;
                            s
                        });
                    }
                }
            } 
        }
    }
}

impl<S> Default for RuntimeEditor<S>
where
    S: RuntimeState + Component,
{
    fn default() -> Self {
        Self {
            runtime: Default::default(),
            events: Default::default(),
            sections: Default::default(),
        }
    }
}

impl<S> App for RuntimeEditor<S>
where
    S: RuntimeState + Component,
{
    fn name() -> &'static str {
        "Runtime Editor"
    }

    fn window_size() -> &'static [f32; 2] {
        &[1500.0, 720.0]
    }

    fn show_editor(&mut self, ui: &imgui::Ui) {
        Window::new(Self::name())
            .size(*Self::window_size(), imgui::Condition::Appearing)
            .build(ui, || {
                ui.text("Extensions");
                ui.new_line();
                for (_, section) in self.sections.iter_mut() {
                    section.show_editor(ui);
                }

                ui.new_line();
                ui.text("Runtime Tools");
                ui.new_line();
                if CollapsingHeader::new(format!("Current Runtime Information")).begin(ui) {
                    ui.indent();

                    if let Some(state) = self.runtime.current() {
                        ui.text(format!("Current State: "));
                        ui.text_wrapped(format!("{}", state));
                        ui.new_line();
                    }

                    let context = self.runtime.context(); 
                    ui.label_text(format!("Current Context"), format!("{}", context));

                    ui.unindent();
                }
                
                if CollapsingHeader::new(format!("Events")).begin(ui) {
                    ui.indent();
                    for e in self.events.iter_mut() {
                        EventComponent::show_editor( e, ui);
                    }
                    ui.unindent();
                }
            });
    }
}
