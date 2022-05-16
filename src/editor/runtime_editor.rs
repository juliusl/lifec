use atlier::system::App;
use imgui::Window;
use specs::{Component, Entities, Join, ReadStorage, System};
use std::collections::BTreeMap;

use crate::{Runtime, RuntimeState};

use super::section::Section;

pub struct RuntimeEditor<S>
where
    S: RuntimeState,
{
    runtime: Runtime<S>,
    sections: BTreeMap<u32, Section<S>>,
}

impl<S: RuntimeState> From<Runtime<S>> for RuntimeEditor<S> {
    fn from(runtime: Runtime<S>) -> Self {
        Self { runtime, sections: BTreeMap::new() }
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
                },
                _ => {},
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
        &[640.0, 720.0]
    }

    fn show_editor(&mut self, ui: &imgui::Ui) {
        Window::new(Self::name())
            .size(*Self::window_size(), imgui::Condition::Appearing)
            .build(ui, || {
                for (_, section) in self.sections.iter_mut() {
                    section.show_editor(ui);
                }
            });
    }
}
