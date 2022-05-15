use std::collections::HashMap;

use atlier::system::{App, Extension};
use specs::{
    Component, DenseVecStorage, DispatcherBuilder, Entities, Join, ReadStorage, System, World,
    WriteStorage,
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

pub struct RuntimeEditor<S>
where
    S: RuntimeState,
{
    runtime: Runtime<S>,
    node_editor: NodeEditor,
    event_node_editors: HashMap<u32, EventNodeEditor>,
}

impl<S> Extension for RuntimeEditor<S>
where
    S: RuntimeState + Component,
{
    fn extend(
        mut self,
        _: &mut World,
        dispatcher: DispatcherBuilder<'static, 'static>,
    ) -> DispatcherBuilder<'static, 'static> {
        self.node_editor = NodeEditor {
            imnodes: imnodes::Context::new(),
            imnode_editors: HashMap::new(),
        };

        dispatcher.with_thread_local(self)
    }
}

impl<'a, S> System<'a> for RuntimeEditor<S>
where
    S: RuntimeState + Component,
{
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, S>,
        WriteStorage<'a, EventComponent>,
        WriteStorage<'a, NodeEditorId>,
        ReadStorage<'a, NodeEditorId>,
    );

    fn run(
        &mut self,
        (entities, _, event_components, write_node_editors, read_node_editors): Self::SystemData,
    ) {
        self.node_editor.run((entities, read_node_editors));
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

    fn show_editor(&mut self, ui: &imgui::Ui, _: &mut Self::State) {}
}
