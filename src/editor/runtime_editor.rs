use imgui::{ChildWindow, CollapsingHeader, Window};
use knot::store::Store;
use specs::{
    storage::DefaultVecStorage, Component, Entities, Join, Read,
    ReadStorage, System, Write, WriteStorage,
};
use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
    time::Instant,
};

use super::{
    event_graph::EventGraph, section::Section, unique_title, App, EventComponent, Value,
};
use crate::{Action, Runtime, RuntimeState, AttributeGraph};

#[derive(Clone)]
pub struct RuntimeEditor<S>
where
    S: RuntimeState,
{
    pub runtime: Runtime<S>,
    pub events: Vec<EventComponent>,
    pub sections: BTreeMap<u32, Section<S>>,
    pub running: (Option<bool>, Option<Instant>, Option<Instant>),
    pub dispatch_snapshot: Option<()>,
    pub dispatch_remove: Option<u32>,
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

        let mut sections: BTreeMap<u32, Section<S>> = BTreeMap::new();
        let sections = &mut sections;
        runtime.attributes.iter_attributes().for_each(|a| {
            if let Some(section) = sections.get_mut(&a.id()) {
                section.attributes.copy_attribute(a);
            } else {
                let mut section = Section::<S>::default();
                let section = section
                    .with_parent_entity_id(a.id())
                    .with_attribute(a)
                    .with_title(format!("Runtime Entity {}", a.id()));
                let section = section.to_owned();
                sections.insert(
                    a.id(),
                    section,
                );
            }
        });

        let sections = sections.clone();
        let next = Self {
            runtime,
            events,
            sections,
            running: (None, None, None),
            dispatch_snapshot: None,
            dispatch_remove: None,
        };
        next
    }
}

impl<'a, S> System<'a> for RuntimeEditor<S>
where
    S: RuntimeState + Component,
{
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Section<S>>,
        WriteStorage<'a, Loader>,
        Write<'a, RuntimeEditor<S>>,
        Write<'a, Dispatch>,
    );

    /// The runtime editor maintains a vector of sections that it displays
    /// This system coordinates updates to those sections, as well as initialization
    fn run(
        &mut self,
        (entities, read_sections, mut loader, mut runtime_editor, mut dispatcher): Self::SystemData,
    ) {
        if let Some(_) = self.dispatch_snapshot.take() {
            if let Some(state) = &self.runtime.state {
                let next = entities.create();
                let section = Section::new(
                    unique_title(format!("{}", self.runtime.context())),
                    AttributeGraph::from(next)
                        .with_text("context::", format!("{}", self.runtime.context()))
                        .with_bool("enable event builder", false)
                        .with_bool("enable node editor", false)
                        .with_text("project::name::", unique_title("snapshot"))
                        .with_bool("enable project", false),
                    |s, ui| {
                        s.edit_attr("Enable Node Editor", "enable node editor", ui);
                        s.edit_attr("Enable Event Editor", "enable event builder", ui);
                        s.edit_attr("Enable project", "enable project", ui);
                        if let Some(true) = s.is_attr_checkbox("enable project") {
                            s.edit_attr("Project name", "project::name::", ui);
                        }
                    },
                    state.clone(),
                )
                .enable_app_systems()
                .enable_edit_attributes();

                match loader.insert(next, Loader::LoadSection(section.attribute_graph().clone())) {
                    Ok(_) => {
                        self.sections.insert(next.id(), section);

                        println!("RuntimeEditor dispatching Loader for Snapshot {:?}", next);
                    }
                    Err(_) => {}
                }
            }
        }

        if let Some(to_remove) = self.dispatch_remove.take() {
            let msg = dispatcher.deref_mut();
            self.sections.remove(&to_remove);
            *msg = Dispatch::RemoveSnapshot(to_remove);
            return;
        }

        for (e, s) in (&entities, &read_sections).join() {
            match self.sections.get(&e.id()) {
                None => {
                    match loader.insert(
                        e,
                        Loader::LoadSection(s.attribute_graph().clone()),
                    ) {
                        Ok(_) => {
                            println!("RuntimeEditor dispatched Loader for {:?}", e);
                        }
                        Err(_) => {}
                    }
                }
                Some(section) => {
                    let Section {
                        gen,
                        enable_app_systems,
                        ..
                    } = section;

                    if *enable_app_systems && *gen != s.get_gen() {
                        self.sections.insert(e.id(), s.clone());
                    }
                }
            }
        }

        for section in read_sections.join() {
            if let None = self.sections.get(&section.get_parent_entity()) {
                println!(
                    "RuntimeEditor inserting section under snapshots {}",
                    section.get_parent_entity()
                );
                self.sections
                    .insert(section.get_parent_entity(), section.clone());
            }
        }

        let runtime_editor = runtime_editor.deref_mut();
        *runtime_editor = self.clone();
    }
}

#[derive(Default)]
pub struct RuntimeDispatcher<S>
where
    S: RuntimeState + Component,
{
    runtime: Option<RuntimeEditor<S>>,
}

impl<S> From<RuntimeEditor<S>> for RuntimeDispatcher<S>
where
    S: RuntimeState + Component,
{
    fn from(runtime: RuntimeEditor<S>) -> Self {
        Self {
            runtime: Some(runtime),
        }
    }
}

pub enum Dispatch {
    Empty,
    RemoveSnapshot(u32),
}

#[derive(Component)]
#[storage(DefaultVecStorage)]
pub enum Loader {
    Empty,
    LoadSection(AttributeGraph),
}

impl Default for Loader {
    fn default() -> Self {
        Loader::Empty
    }
}

impl Default for Dispatch {
    fn default() -> Self {
        Self::Empty
    }
}

impl<'a, S> System<'a> for RuntimeDispatcher<S>
where
    S: RuntimeState + Component,
{
    type SystemData = (
        Entities<'a>,
        Read<'a, RuntimeEditor<S>>,
        Write<'a, Dispatch>,
        WriteStorage<'a, Loader>,
        WriteStorage<'a, Section<S>>,
        WriteStorage<'a, AttributeGraph>,
        WriteStorage<'a, EventGraph>,
    );

    fn run(
        &mut self,
        (
            entities,
            runtime_editor,
            mut msg,
            mut loader,
            mut sections,
            mut section_attributes,
            mut event_graph,
        ): Self::SystemData,
    ) {
        if let Dispatch::RemoveSnapshot(id) = msg.deref() {
            let to_remove = entities.entity(*id);
            match entities.delete(to_remove) {
                Ok(_) => {
                    let msg = msg.deref_mut();
                    *msg = Dispatch::Empty;
                    sections.remove(to_remove);
                    section_attributes.remove(to_remove);
                    event_graph.remove(to_remove);
                    return;
                }
                Err(err) => eprintln!("RuntimeDispatcher Error {}", err),
            }
        }

        for entity in entities.join() {
            if let Some(Loader::LoadSection(attributes)) = loader.get(entity) {
                println!("Load section for {:?}", entity);

                match sections.get(entity) {
                    Some(_) => {
                        if let Some(section) = sections.get_mut(entity) {
                            println!("Existing section found, updating attributes");
                            attributes.iter_attributes().for_each(|a| {
                                section.attributes.copy_attribute(a);
                            });

                            section.next_gen();
                        }
                    }
                    None => {
                        println!(
                            "Section not found for {:?}, Generating section from attributes",
                            entity
                        );
                        let initial = S::from(attributes.clone());

                        let mut section = Section::<S>::default();
                        section.state = initial;

                        attributes.iter_attributes().for_each(|a| {
                            section.attributes.copy_attribute(a);
                        });

                        if let Some(Value::TextBuffer(title)) =
                            section.clone().attributes.find_attr_value("title::")
                        {
                            let section =
                                section.with_title(title.to_string()).with_parent_entity(entity);

                            match sections.insert(entity, section.clone()) {
                                Ok(_) => {
                                    println!(
                                        "RuntimeDispatcher added Section {:?}, {}",
                                        entity, &section.title
                                    );
                                }
                                Err(err) => {
                                    println!("section could not be loaded {}", err);
                                }
                            }
                        }
                    }
                }

                if let Some(v) = loader.get_mut(entity) {
                    *v = Loader::Empty;
                }
                return;
            }
        }

        self.runtime = Some(runtime_editor.clone());
        if let Some(runtime) = &self.runtime {
            for e in entities.join() {
                if let Some(section) = runtime.sections.get(&e.id()) {
                    match sections.insert(e, section.clone()) {
                        Ok(_) => {
                            let mut section = section.clone();
                            let section = section.with_parent_entity(e);
                            if let Some(state) = runtime.runtime.current() {
                                section.state = state.clone();
                            }

                            match section_attributes
                                .insert(e, section.attribute_graph().clone())
                            {
                                Ok(_) => {
                                    if let None = event_graph.get(e) {
                                        let mut store = Store::<EventComponent>::default();
                                        runtime.events.iter().cloned().for_each(|e| {
                                            store = store.node(e);
                                        });
                                        match event_graph.insert(e, EventGraph(store)) {
                                            Ok(v) => {
                                                println!("inserting graph for {:?}", e);
                                                println!("{:?}", v);
                                            }
                                            Err(_) => {}
                                        }
                                    }
                                }
                                Err(err) => eprintln!("RuntimeDispatcher Error {}", err),
                            }
                        }
                        Err(err) => eprintln!("RuntimeDispatcher Error {}", err),
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
            running: (None, None, None),
            dispatch_snapshot: None,
            dispatch_remove: None,
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
                ChildWindow::new("Sections").always_use_window_padding(true).size([1000.0, 0.0]).build(ui, || {
                    if CollapsingHeader::new("Snapshots").leaf(true).begin(ui) {
                        if ui.button("Take Snapshot of Runtime") {
                            self.dispatch_snapshot = Some(());
                            return;
                        }
                        ui.new_line();

                        self.show_snapshots(ui);
                    }
                });
                ui.same_line();
                ChildWindow::new("Runtime").always_use_window_padding(true).size([0.0, 0.0]).build(ui, || {
                    if CollapsingHeader::new(format!("Current Runtime"))
                        .leaf(true)
                        .begin(ui)
                    {
                        ui.separator();
                        self.show_current(ui);
                    }
                    ui.new_line();
                });
            });
    }
}

impl<S> RuntimeEditor<S>
where
    S: RuntimeState + Component,
{
    pub fn apply_section(section: Section<S>, mut runtime: Runtime<S>) -> Self {
        // This will apply the sections current state and attributes to the current runtime
        runtime.state = Some(S::from(section.attribute_graph().clone()));
        section.attributes.iter_attributes().for_each(|a| {
            runtime.attribute(a);
        });
        if let Some(Value::TextBuffer(event)) = section.attributes.find_attr_value("context::") {
            runtime = runtime.parse_event(&event);
        }

        RuntimeEditor::from(runtime)
    }

    pub fn show_current(&mut self, ui: &imgui::Ui) {
        match self.running {
            (Some(v), elapsed, stopped) => {
                if ui.button("Stop") {
                    self.dispatch_remove = None;
                    self.dispatch_snapshot = None;
                    self.running = (None, None, None);
                }

                match (v, elapsed, stopped) {
                    (true, Some(elapsed), None) => {
                        ui.same_line();
                        if ui.button("Pause") {
                            self.running = (Some(false), Some(elapsed), Some(Instant::now()));
                        }

                        ui.text(format!("Running {:#?}", elapsed.elapsed()));

                        if self.runtime.can_continue() {
                            self.runtime = self.runtime.step();
                        } else {
                            self.running = (None, Some(elapsed), Some(Instant::now()));
                        }
                    }
                    (false, Some(elapsed), Some(stopped)) => {
                        ui.same_line();
                        if ui.button("Continue") {
                            self.running = (Some(true), Some(elapsed), None);
                        }

                        ui.text(format!("Paused {:#?}", stopped.elapsed()));
                    }
                    _ => {}
                }
            }
            (None, Some(elapsed), Some(stopped)) => {
                if ui.button("Clear") {
                    self.dispatch_remove = None;
                    self.dispatch_snapshot = None;
                    self.running = (None, None, None);
                }
                ui.text(format!("Ran for {:#?}", stopped - elapsed));
            }
            _ => {}
        };

        let context = self.runtime.context();
        ui.label_text(format!("Current Event"), format!("{}", context));
        ui.disabled(self.running.0.is_some(), || {
            if ui.button("Setup") {
                self.runtime.parse_event("{ setup;; }");
            }
            ui.same_line();
            if ui.button("Start") {
                self.running = (Some(true), Some(Instant::now()), None);
            }

            ui.same_line();
            if ui.button("Step") {
                self.runtime = self.runtime.step();
            }

            ui.same_line();
            if ui.button("Clear Attributes") {
                self.runtime.attributes.clear_index();
            }
        });
        ui.new_line();
        ui.separator();
        if let Some(state) = self.runtime.current() {
            ui.input_text_multiline(
                format!("Current State"),
                &mut format!("{}", state),
                [0.0, 0.0],
            )
            .read_only(true)
            .build();
            ui.new_line();
        }

        if !self.runtime.attributes.is_index_empty() {
            if CollapsingHeader::new(format!("Runtime Attributes"))
                .leaf(true)
                .build(ui)
            {
                self.runtime.attributes.iter_attributes().for_each(|a| {
                    ui.text(format!("{}", a));
                });
            }
        }
    }

    pub fn show_snapshots(&mut self, ui: &imgui::Ui) {
        let current_runtime = self.runtime.clone();
        for (id, section) in self.sections.iter_mut() {
            ui.text(format!("{}: ", id));
            ui.same_line();
            ui.indent();

            section.show_editor(ui);
            if ui.button(format!("Apply {}", section.title)) {
                let applied = Self::apply_section(section.clone(), current_runtime);
                self.runtime = applied.runtime.clone();

                return;
            }

            ui.same_line();
            if ui.button(format!("Remove {}", section.title)) {
                self.dispatch_remove = Some(*id);
                return;
            }
            ui.new_line();
            ui.separator();
            ui.unindent();
        }
    }
}
