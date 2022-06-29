use atlier::system::App;
use imgui::{ChildWindow, MenuItem, Ui, Window};
use plugins::{Engine, Event, Plugin, Project, ThunkContext};
use specs::{Component, Entity, System, World, WorldExt};
use std::collections::HashMap;
use std::fmt::Display;
use std::{any::Any, collections::BTreeMap};

pub mod editor;
pub mod plugins;

mod state;
pub use state::AttributeGraph;

pub trait RuntimeDispatcher: AsRef<AttributeGraph> + AsMut<AttributeGraph>
where
    Self: Sized,
{
    type Error;

    /// dispatch_mut is a function that should take a string message that can mutate state
    /// and returns a result
    fn dispatch_mut(&mut self, msg: impl AsRef<str>) -> Result<(), Self::Error>;

    /// dispatch calls dispatch_mut on a clone of Self and returns the clone
    fn dispatch(&self, msg: impl AsRef<str>) -> Result<Self, Self::Error>
    where
        Self: Clone,
    {
        let mut next = self.to_owned();
        match next.dispatch_mut(msg) {
            Ok(_) => Ok(next.to_owned()),
            Err(err) => Err(err),
        }
    }

    fn batch(&self, msgs: impl AsRef<str>) -> Result<Self, Self::Error>
    where
        Self: Clone,
    {
        let mut next = self.clone();
        for message in msgs
            .as_ref()
            .trim()
            .lines()
            .filter(|line| !line.trim().is_empty())
        {
            next = next.dispatch(message)?;
        }

        Ok(next)
    }

    fn batch_mut(&mut self, msg: impl AsRef<str>) -> Result<(), Self::Error> {
        for message in msg
            .as_ref()
            .trim()
            .split("\n")
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
        {
            self.dispatch_mut(message)?;
        }
        Ok(())
    }

    /// Dispatch a batch of messages from a file.
    fn from_file(&mut self, path: impl AsRef<str>) -> Result<(), Self::Error> {
        use std::fs;

        if let Some(initial_setup) = fs::read_to_string(path.as_ref()).ok() {
            self.batch_mut(initial_setup)?;
        }

        Ok(())
    }
}

pub trait RuntimeState:
    Any + Sized + Clone + Sync + Default + Send + Display + From<AttributeGraph>
{
    type Dispatcher: RuntimeDispatcher;

    // /// try to save the current state to a String
    fn save(&self) -> Option<String> {
        match serde_json::to_string(self.state()) {
            Ok(val) => Some(val),
            Err(_) => None,
        }
    }

    /// load should take the serialized form of this state
    /// and create a new instance of Self
    fn load(&self, init: impl AsRef<str>) -> Self {
        if let Some(attribute_graph) = serde_json::from_str::<AttributeGraph>(init.as_ref()).ok() {
            Self::from(attribute_graph)
        } else {
            self.clone()
        }
    }

    /// Returns a mutable dispatcher for this runtime state
    fn dispatcher_mut(&mut self) -> &mut Self::Dispatcher {
        todo!("dispatcher is not implemented for runtime state")
    }

    // Returns the dispatcher for this runtime state
    fn dispatcher(&self) -> &Self::Dispatcher {
        todo!("dispatcher is not implemented for runtime state")
    }

    // Returns the current state from the dispatcher
    fn state(&self) -> &AttributeGraph {
        self.dispatcher().as_ref()
    }

    // Returns the current state as mutable from dispatcher
    fn state_mut(&mut self) -> &mut AttributeGraph {
        self.dispatcher_mut().as_mut()
    }

    /// merge_with merges a clone of self with other
    fn merge_with(&self, other: &Self) -> Self {
        let mut next = self.clone();

        next.state_mut().merge(other.state());

        next
    }
}

/// Runtime provides access to the underlying project, and function tables for creating components
pub struct Runtime {
    project: Project,
    /// Table for creating new event components
    create_event: BTreeMap<String, fn(&World, fn(&mut ThunkContext)) -> Option<Entity>>,
    receivers: HashMap<String, tokio::sync::broadcast::Receiver<Entity>>,
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            project: Default::default(),
            create_event: BTreeMap::default(),
            receivers: HashMap::default(),
        }
    }
}

impl Runtime {
    /// returns a runtime from a project, with no plugins installed
    pub fn new(project: Project) -> Self {
        Self {
            project,
            create_event: BTreeMap::default(),
            receivers: HashMap::default(),
        }
    }

    /// returns the next thunk context that has been updated by the event runtime, if registered to broadcasts
    pub fn listen<P>(&mut self, world: &World) -> Option<ThunkContext>
    where
        P: Plugin<ThunkContext>,
    {
        self.listen_with(world, P::symbol())
    }

    /// subscribe to thunk contexts updated from the event runtime
    pub fn subscribe<P>(&mut self, world: &World)
    where
        P: Plugin<ThunkContext>,
    {
        self.subscribe_with(world, P::symbol());
    }

    /// returns the next thunk context that has been updated by the event runtime, if registered to broadcasts
    pub fn listen_with(
        &mut self,
        world: &World,
        with_key: impl AsRef<str>,
    ) -> Option<ThunkContext> {
        if let Some(rx) = self.receivers.get_mut(with_key.as_ref()) {
            match rx.try_recv() {
                Ok(entity) => {
                    let contexts = world.read_component::<ThunkContext>();
                    contexts.get(entity).and_then(|c| Some(c.clone()))
                }
                Err(_) => None,
            }
        } else {
            // If not already subscribed, the plugin will miss any events it generated before calling listen
            // this is probably not too bad since this is called inside the loop, so in most situations, the subscriber will get a
            // chance to subscribe before it has a chance to make any changes
            // TODO: Can add a way to subscribe in the runtime's system trait
            self.subscribe_with(world, with_key);
            None
        }
    }

    /// subscribe to thunk contexts updated from the event runtime
    pub fn subscribe_with(&mut self, world: &World, with_key: impl AsRef<str>) {
        self.receivers.insert(with_key.as_ref().to_string(), Event::subscribe(world));
    }

    /// Install an engine into the runtime. An engine provides functions for creating new component instances.
    pub fn install<E, P>(&mut self)
    where
        E: Engine,
        P: Plugin<ThunkContext> + Component + Send + Default,
    {
        let event = E::event::<P>();
        self.create_event.insert(event.to_string(), E::create::<P>);

        println!("install event: {}", event.to_string());
    }

    /// initialize and configure an event component and it's deps for a new entity, and insert into world.
    pub fn create(
        &self,
        world: &World,
        event: &Event,
        config_fn: fn(&mut ThunkContext),
    ) -> Option<Entity> {
        let key = event.to_string();

        if let Some(create_fn) = self.create_event.get(&key) {
            (create_fn)(world, config_fn)
        } else {
            None
        }
    }

    pub fn schedule(
        &mut self,
        world: &World,
        event: &Event,
        config: impl FnOnce(&mut ThunkContext),
    ) -> Option<Entity> {
        if let Some(entity) = self.create(world, event, |_| {}) {
            let mut contexts = world.write_component::<ThunkContext>();
            let mut events = world.write_component::<Event>();
            if let Some(tc) = contexts.get_mut(entity) {
                config(tc);
                if let Some(event) = events.get_mut(entity) {
                    event.fire(tc.clone());
                    return Some(entity);
                }
            }
        }

        None
    }

    pub fn sequence(&mut self, world: &World, _initial: ThunkContext, events: Vec<Event>) {
        let mut entities = vec![];
        for event in events.iter() {
            if let Some(entity) = self.create(world, event, |_| {}) {
                entities.push(entity);
            }
        }

        let sequence_key = _initial.label("sequence_key");
        self.subscribe_with(world, sequence_key);

        let mut events = world.write_component::<Event>();
        for entity in entities {
            if let Some(_event) = events.get_mut(entity) {
                _event.fire(_initial.clone());
            }
        }
    }

    /// Generate runtime_state from the underlying project graph
    pub fn state<S>(&self) -> S
    where
        S: RuntimeState,
    {
        S::from(self.project.as_ref().clone())
    }
}

impl<'a> System<'a> for Runtime {
    type SystemData = ();

    fn run(&mut self, _: Self::SystemData) {}
}

impl Runtime {
    fn menu(&mut self, ui: &Ui) {
        ui.menu("Edit", || {
            for (event_name, _) in self.create_event.iter() {
                ui.menu(event_name, || {})
            }
        });
    }

    pub fn create_event_menu_item(
        &mut self,
        world: &World,
        event: &Event,
        config_fn: fn(&mut ThunkContext),
        tooltip: impl AsRef<str>,
        ui: &Ui,
    ) {
        ui.menu("Edit", || {
            ui.menu("Events", || {
                let label = format!("Add '{}'", event.to_string());
                if MenuItem::new(label).build(ui) {
                    self.create(world, event, config_fn);
                }
                if ui.is_item_hovered() {
                    ui.tooltip_text(tooltip);
                }
            });
        });
    }
}

impl App for Runtime {
    fn name() -> &'static str {
        "runtime"
    }

    fn window_size() -> &'static [f32; 2] {
        &[1500.0, 720.0]
    }

    fn edit_ui(&mut self, ui: &imgui::Ui) {
        Window::new(format!(
            "Runtime - hash: {}",
            self.project.as_ref().hash_code()
        ))
        .size(*Self::window_size(), imgui::Condition::Appearing)
        .menu_bar(true)
        .build(ui, || {
            ui.menu_bar(|| {
                let project = &mut self.project;
                project.edit_project_menu(ui);

                self.menu(ui);
            });

            let project = &mut self.project;
            if let Some(tabbar) = ui.tab_bar("runtime_tabs") {
                for (_, block) in project.iter_block_mut().enumerate() {
                    let (block_name, block) = block;

                    let thunk_symbol = if let Some(thunk) = block.get_block("thunk") {
                        thunk.find_text("thunk_symbol")
                    } else {
                        None
                    };

                    let thunk_symbol = thunk_symbol.unwrap_or("entity".to_string());

                    if let Some(token) = ui.tab_item(format!("{} {}", thunk_symbol, block_name)) {
                        ui.group(|| {
                            block.edit_block_view(true, ui);
                            ChildWindow::new(&format!("table_view_{}", block_name))
                                .size([0.0, 0.0])
                                .build(ui, || {
                                    block.edit_block_table_view(ui);
                                });
                        });

                        token.end();
                    }
                }
                tabbar.end();
            }
        });
    }

    fn display_ui(&self, _: &imgui::Ui) {}
}
