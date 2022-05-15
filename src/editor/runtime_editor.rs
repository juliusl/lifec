use std::collections::{BTreeMap, HashMap};

use atlier::system::{App, Extension};
use imgui::{CollapsingHeader, Window};
use specs::{
    Component, DenseVecStorage, DispatcherBuilder, Entities, HashMapStorage, Join, ReadStorage,
    System, World, WriteStorage,
};

use crate::{Runtime, RuntimeState};

use super::{
    event_node_editor::EventNodeEditor,
    node_editor::{NodeEditor, NodeEditorId},
};

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

#[derive(Clone)]
pub struct Section {
    pub title: String,
}

impl Component for Section {
    type Storage = HashMapStorage<Self>;
}

pub struct RuntimeEditor<S>
where
    S: RuntimeState,
{
    runtime: Runtime<S>,
    sections: BTreeMap<u32, Section>,
    // node_editor: NodeEditor,
    // event_node_editors: HashMap<u32, EventNodeEditor>,
}

impl<'a, S> System<'a> for RuntimeEditor<S>
where
    S: RuntimeState + Component,
{
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Section>,
    );

    fn run(&mut self, (entities, sections): Self::SystemData) {
        //self.node_editor.run((entities, read_node_editors));

        for (e, s) in (&entities, &sections).join() {
            if let None = self.sections.get(&e.id()) {
                self.sections.insert(e.id(), s.clone());
            }
        }
    }
}

impl<S> Default for RuntimeEditor<S> 
where
    S: RuntimeState + Component,
{
    fn default() -> Self {
        Self { runtime: Default::default(), sections: Default::default() }
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

    fn show_editor(&mut self, ui: &imgui::Ui, _: &mut Self::State) {
        Window::new(Self::title())
            .size(*Self::window_size(), imgui::Condition::Appearing)
            .build(ui, || {
                for title in self.sections.iter().map(|(_, Section { title })| title) {
                    if CollapsingHeader::new(title).build(ui) {
                        ui.text("test");
                    }
                }
            });
    }
}
