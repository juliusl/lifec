pub use atlier::system::{App, Value, Extension};
use imgui::{ChildWindow, MenuItem, Ui, Window};
use plugins::{BlockContext, Config, Engine, Event, Plugin, Project, Sequence, ThunkContext};
pub use specs::{Component, Entity, System, World, WorldExt};
use std::collections::HashMap;
use std::fmt::Display;
use std::{any::Any, collections::BTreeMap};

mod open;
pub use open::open;

mod start;
pub use start::start;

pub mod editor;
pub mod plugins;

mod state;
pub use state::AttributeGraph;

pub trait RuntimeDispatcher: AsRef<AttributeGraph> + AsMut<AttributeGraph>
where
    Self: Sized,
{
    type Error;

    /// Dispatch_mut is a function that should take a string message that can mutate state
    /// and returns a result
    fn dispatch_mut(&mut self, msg: impl AsRef<str>) -> Result<(), Self::Error>;

    /// Dispatch calls dispatch_mut on a clone of Self and returns the clone
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

    /// Interpret several msgs w/ a clone of self
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

    /// Interpret several msgs, applying changes to self
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

    /// Try to save the current state to a String
    fn save(&self) -> Option<String> {
        match serde_json::to_string(self.state()) {
            Ok(val) => Some(val),
            Err(_) => None,
        }
    }

    /// Load should take the serialized form of this state
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

pub type CreateFn = fn(&World, fn(&mut ThunkContext)) -> Option<Entity>;
pub type ConfigFn = fn(&mut ThunkContext);

/// Runtime provides access to the underlying project, and function tables for creating components
pub struct Runtime {
    project: Project,
    /// Table for creating new event components
    engine_plugin: BTreeMap<String, CreateFn>,
    /// Table for thunk configurations
    config: BTreeMap<String, ConfigFn>,
    /// Table of broadcast receivers
    receivers: HashMap<String, tokio::sync::broadcast::Receiver<Entity>>,
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            project: Default::default(),
            engine_plugin: BTreeMap::default(),
            config: BTreeMap::default(),
            receivers: HashMap::default(),
        }
    }
}

impl Runtime {
    /// Returns a runtime from a project, with no plugins installed
    pub fn new(project: Project) -> Self {
        Self {
            project,
            engine_plugin: BTreeMap::default(),
            config: BTreeMap::default(),
            receivers: HashMap::default(),
        }
    }

    /// Returns the next thunk context that has been updated by the event runtime, if registered to broadcasts.
    /// Uses the plugin symbol as the subscriber key.
    pub fn listen<P>(&mut self, world: &World) -> Option<ThunkContext>
    where
        P: Plugin<ThunkContext>,
    {
        self.listen_with(world, P::symbol())
    }

    /// Subscribe to thunk contexts updates, with the plugin symbol as the subscriber key
    pub fn subscribe<P>(&mut self, world: &World)
    where
        P: Plugin<ThunkContext>,
    {
        self.subscribe_with(world, P::symbol());
    }

    /// Install an engine into the runtime. An engine provides functions for creating new component instances.
    pub fn install<E, P>(&mut self)
    where
        E: Engine,
        P: Plugin<ThunkContext> + Component + Send + Default,
    {
        let event = E::event::<P>();
        self.engine_plugin.insert(event.to_string(), E::create::<P>);

        println!("install event: {}", event.to_string());
    }

    /// Registers a config w/ this runtime
    pub fn add_config(&mut self, config: Config) {
        let Config(name, config_fn) = config;

        self.config.insert(name.to_string(), config_fn);
    }

    /// Generate runtime_state from the underlying project graph
    pub fn state<S>(&self) -> S
    where
        S: RuntimeState,
    {
        S::from(self.project.as_ref().clone())
    }

    fn find_config_and_create(
        &self,
        world: &World,
        config_name: impl AsRef<str>,
        create_event: CreateFn,
    ) -> Option<Entity> {
        self.config
            .get(config_name.as_ref())
            .and_then(|c| create_event(world, *c))
    }

    fn find_config_block_and_create(
        &self,
        world: &World,
        block_name: impl AsRef<str>,
        config_block: BlockContext,
        create_event: CreateFn,
    ) -> Option<Entity> {
        if let Some(created) = create_event(world, |_| {}) {
            let mut tc = world.write_component::<ThunkContext>();
            if let Some(tc) = tc.get_mut(created) {
                for (name, value) in config_block
                    .as_ref()
                    .iter_attributes()
                    .filter(|a| a.is_stable())
                    .map(|a| (a.name(), a.value()))
                {
                    tc.as_mut().with(name, value.clone());
                }
                tc.block.block_name = block_name.as_ref().to_string();
            }

            return Some(created);
        } else {
            None
        }
    }
}

impl<'a> System<'a> for Runtime {
    type SystemData = ();

    fn run(&mut self, _: Self::SystemData) {}
}

impl Runtime {
    fn menu(&mut self, _: &Ui) {
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
            let label = format!("Add '{}' event", event.to_string());
            if MenuItem::new(label).build(ui) {
                self.create_with_fn(world, event, config_fn);
            }
            if ui.is_item_hovered() {
                ui.tooltip_text(tooltip);
            }
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
        .bg_alpha(0.99)
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

                    let thunk_symbol = block
                        .as_ref()
                        .find_text("thunk_symbol")
                        .unwrap_or("entity".to_string());

                    if let Some(token) = ui.tab_item(format!("{} {}", thunk_symbol, block_name)) {
                        ui.group(|| {
                            block.edit_block_view(false, ui);
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

/// Methods for creating engines & plugins
impl Runtime {
    /// Creates a new engine,
    /// an engine is defined by a sequence of events.
    pub fn create_engine<E>(
        &self,
        world: &World,
        sequence_block_name: &'static str,
    ) -> Option<Entity>
    where
        E: Engine,
    {
        eprintln!("Creating engine for {}", sequence_block_name);
        if let Some(block) = self.project.find_block(sequence_block_name) {
            if let Some(mut engine_root) = block.get_block(E::event_name()) {
                eprintln!(
                    "Found engine root for {} {}",
                    block.block_name,
                    E::event_name()
                );
                let mut sequence = Sequence::default();

                for (block_address, block_name, value) in
                    engine_root.clone().iter_attributes().filter_map(|a| {
                        let name = a.name();
                        if let Some((tname, tvalue)) = a.transient() {
                            Some((name.to_string(), tname.clone(), tvalue.clone()))
                        } else {
                            None
                        }
                    })
                {
                    eprintln!(
                        "Found definition for {}, {} {:?}",
                        block_name, block_address, value
                    );
                    if let Some((_, block_symbol)) = block_address.split_once("::") {
                        let engine_plugin_key = format!("{} {}", "call", block_symbol);
                        eprintln!("Looking for engine_plugin {}", engine_plugin_key);
                        if let Some(create_fn) = self.engine_plugin.get(&engine_plugin_key) {
                            if let Some(created) =
                                self.create_plugin(world, block_address, value.clone(), *create_fn)
                            {
                                eprintln!(
                                    "\tCreated event {:?}, {}, {}, config: {:?}",
                                    created, engine_plugin_key, block_name, &value
                                );
                                engine_root.add_int_attr(block_name, created.id() as i32);
                                sequence.add(created);
                            }
                        }
                    }
                }

                engine_root.add_text_attr("sequence_name", sequence_block_name);
                return E::initialize_sequence(engine_root, sequence, world);
            }
        } else {
            eprintln!("{} block not found", sequence_block_name);
        }

        None
    }

    /// Reads/creates events from symbols defined at the root of the graph for a given plugin.
    /// Symbols defined at the root specify a block_address in the format,
    /// {block_name}::{block_symbol},
    ///     for example test.sh::file, can have a corresponding block ``` test.sh file
    /// If the transient value is empty, the runtime will try to find the corresponding block
    /// in order to get the name of the config to use. If the block isn't found, than nothing will be created.
    /// If the transient is a symbol or text value, this value will be used to look up the config to use.
    pub fn create_plugin(
        &self,
        world: &World,
        block_address: impl AsRef<str>,
        value: Value,
        create_event: CreateFn,
    ) -> Option<Entity> {
        let blocks = self.project.clone();

        if let Some((block_name, plugin_symbol)) = block_address.as_ref().split_once("::") {
            match value {
                atlier::system::Value::Empty => {
                    eprintln!(
                        "plugin symbol defined w/ block_address {}",
                        block_address.as_ref(),
                    );

                    eprintln!("block_name: {}", block_name);
                    if let Some(block) = blocks.find_block(block_name) {
                        eprintln!("found block {}", block.block_name);
                        let config = block
                            .get_block(plugin_symbol)
                            .and_then(|b| b.find_text("config"));
                        if let Some(config_name) = config {
                            eprintln!("config block {}", config_name);
                            return self.find_config_and_create(world, config_name, create_event);
                        }

                        let config_block = block
                            .get_block(plugin_symbol)
                            .and_then(|b| b.find_symbol("config"))
                            .and_then(|b| blocks.find_block(b));

                        if let Some(config_block) = config_block {
                            eprintln!("config block {}", config_block.block_name);
                            return self.find_config_block_and_create(
                                world,
                                block_name,
                                config_block,
                                create_event,
                            );
                        }
                    }
                }
                atlier::system::Value::TextBuffer(config_name) => {
                    eprintln!(
                        "plugin symbol defined w/ block_address {}, config assigned: {}",
                        block_address.as_ref(),
                        config_name
                    );
                    return self.find_config_and_create(world, config_name, create_event);
                }
                atlier::system::Value::Symbol(config_block_name) => {
                    eprintln!(
                        "plugin symbol defined w/ block_address {}, looking for config block {} {}",
                        block_address.as_ref(),
                        config_block_name,
                        plugin_symbol,
                    );
                    if let Some(config_block) = blocks.find_block(config_block_name) {
                        eprintln!("config block {}", config_block.block_name);
                        return self.find_config_block_and_create(
                            world,
                            block_name,
                            config_block,
                            create_event,
                        );
                    }
                }
                _ => {
                    eprintln!("No config found for {}", block_address.as_ref());
                }
            }
        }

        None
    }
}

/// Methods for accessing broadcast channel
impl Runtime {
    /// Returns the next thunk context that has been updated by the event runtime, if registered to broadcasts
    /// If the subscriber key did not exist, this method will subscribe_with the key so that the next call is successful.
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
            self.subscribe_with(world, with_key);
            None
        }
    }

    /// Subscribe to thunk contexts updates, with a subscriber key
    pub fn subscribe_with(&mut self, world: &World, with_key: impl AsRef<str>) {
        self.receivers
            .insert(with_key.as_ref().to_string(), Event::subscribe(world));
    }
}

/// Methods to create/schedule events
impl Runtime {
    /// Initialize and configure an event component and it's deps for a new entity, and insert into world.
    pub fn create_with_fn(
        &self,
        world: &World,
        event: &Event,
        config_fn: fn(&mut ThunkContext),
    ) -> Option<Entity> {
        let key = event.to_string();

        if let Some(create_fn) = self.engine_plugin.get(&key) {
            (create_fn)(world, config_fn)
        } else {
            None
        }
    }

    /// Initialize and configure an event component and it's deps for a new entity, and insert into world.
    pub fn create_with_config(
        &self,
        world: &World,
        event: &Event,
        config: impl AsRef<Config>,
    ) -> Option<Entity> {
        let key = event.to_string();

        if let Some(create_fn) = self.engine_plugin.get(&key) {
            (create_fn)(world, config.as_ref().1.clone())
        } else {
            None
        }
    }

    /// Initialize and configure an event component and it's deps for a new entity, and insert into world.
    pub fn create_with_name(
        &self,
        world: &World,
        event: &Event,
        config_name: &'static str,
    ) -> Option<Entity> {
        if let Some(config) = self.config.get(config_name) {
            self.create_with_config(world, event, Config(config_name, *config))
        } else {
            None
        }
    }

    /// Creates a new event, returns an entity if successful
    pub fn create_event<E, P>(&self, world: &World, config_name: &'static str) -> Option<Entity>
    where
        E: Engine,
        P: Plugin<ThunkContext> + Component + Send + Default,
    {
        self.create_with_name(world, &E::event::<P>(), config_name)
    }

    /// Schedule a new event on this runtime, returns an entity if the event was created/started
    pub fn schedule(
        &mut self,
        world: &World,
        event: &Event,
        config: impl FnOnce(&mut ThunkContext),
    ) -> Option<Entity> {
        if let Some(entity) = self.create_with_fn(world, event, |_| {}) {
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

    /// Schedules a new event with a registered config, returns an entity if the event was created
    /// started. If the config does not exist, this is a no-op.
    pub fn schedule_with_config(
        &mut self,
        world: &World,
        event: &Event,
        config_name: &'static str,
    ) -> Option<Entity> {
        if let Some(entity) = self.create_with_name(world, event, config_name) {
            let mut contexts = world.write_component::<ThunkContext>();
            let mut events = world.write_component::<Event>();
            if let Some(tc) = contexts.get_mut(entity) {
                if let Some(event) = events.get_mut(entity) {
                    event.fire(tc.clone());
                    return Some(entity);
                }
            }
        }

        None
    }
}
