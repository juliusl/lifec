use atlier::system::App;
use imgui::Window;
use specs::{Component, DenseVecStorage, Entities, Join, ReadStorage, System};
use std::collections::BTreeMap;

use crate::{Runtime, RuntimeState};

use super::section::Section;

#[derive(Clone)]
pub struct EventComponent {
    pub on: String,
    pub dispatch: String,
    pub call: String,
    pub transitions: Vec<String>,
}

impl Component for EventComponent {
    type Storage = DenseVecStorage<Self>;
}

pub struct RuntimeEditor<S>
where
    S: RuntimeState,
{
    runtime: Runtime<S>,
    sections: BTreeMap<u32, Section<S>>,
}

impl<'a, S> System<'a> for RuntimeEditor<S>
where
    S: RuntimeState + Component,
{
    type SystemData = (Entities<'a>, ReadStorage<'a, Section<S>>);

    fn run(&mut self, (entities, sections): Self::SystemData) {
        for (e, s) in (&entities, &sections).join() {
            self.sections.insert(e.id(), s.clone());
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
    fn title() -> &'static str {
        "Runtime Editor"
    }

    fn window_size() -> &'static [f32; 2] {
        &[1280.0, 720.0]
    }

    fn show_editor(&mut self, ui: &imgui::Ui) {
        Window::new(Self::title())
            .size(*Self::window_size(), imgui::Condition::Appearing)
            .build(ui, || {
                for (_, section) in self.sections.iter_mut() {
                    section.show_editor(ui);
                }
            });
    }
}
