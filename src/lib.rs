pub use atlier::system::{App, Extension, Value};
pub use atlier::system::{combine, combine_default};
use editor::{Call, RuntimeEditor};
use logos::{Lexer, Logos};
pub use specs::storage::BTreeStorage;
pub use specs::{Component, DispatcherBuilder, Entity, System, World, WorldExt};
pub use specs::{DefaultVecStorage, DenseVecStorage, HashMapStorage};
pub use specs::{Entities, Join, ReadStorage, WriteStorage};

use tracing::{event, Level};
use imgui::{ChildWindow, MenuItem, Ui, Window};
use plugins::{
    AsyncContext, BlockContext, Config, Connection, Engine, Event, Expect, OpenDir, OpenFile,
    Plugin, Println, Process, Project, Remote, Sequence, ThunkContext, Timer, WriteFile, Thunk,
};
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;
use std::{any::Any, collections::BTreeMap};

mod resources;
pub use resources::Resources;

mod open;
pub use open::open;

mod start;
pub use start::start;

pub mod editor;
pub mod plugins;

mod catalog;
pub use catalog::CatalogReader;
pub use catalog::CatalogWriter;
pub use catalog::Item;

mod state;
pub use state::AttributeGraph;
pub use state::AttributeGraphEvents;
pub use state::AttributeGraphElements;
pub use state::AttributeGraphErrors;
pub use state::Query;
pub use state::AttributeIndex;
pub use state::Operation;

mod host;
pub use host::Host;
pub use host::HostExitCode;

use crate::plugins::ProxyDispatcher;

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
/// 
#[derive(Component, Default, Clone)]
#[storage(DefaultVecStorage)]
pub struct Runtime {
    /// Project loaded w/ this runtime, typically from a .runmd file
    /// The project file usually contains different configurations for event scheduling
    project: Project,
    /// Table for creating new event components
    engine_plugin: BTreeMap<String, CreateFn>,
    /// Table for thunk configurations
    config: BTreeMap<String, ConfigFn>,
}

/// Event builder returned by a runtime, that can be used to schedule events w/ a world
/// 
pub struct EventSource {
    event: Event,
    create_fn: CreateFn,
    runtime: Arc<Runtime>, 
}

impl EventSource {
    /// Sets the config for the event
    /// 
    pub fn set_config(&mut self, config: Config) {
        self.event.set_config(config);
    }

    /// Sets the config from a config registered with the runtime
    /// 
    /// If the config doesn't exist then this is a no-op
    /// 
    pub fn set_config_from_runtime(&mut self, name: impl AsRef<str>) {
        if let Some(config_fn) = self.runtime.config.get(name.as_ref()) {
            self.event.set_config(Config("from_runtime", *config_fn));
        }
    }

    /// Configures the event to configure the context from the project 
    /// 
    /// **Caveat** The block name of the context's block context must be set
    /// 
    pub fn set_config_from_project(&mut self) {
        self.event.set_config(Config("from_project", |mut tc| {
            let tc_clone = tc.clone();
            if let Some(project) = tc_clone.project.as_ref() {
                project.configure(&mut tc);
            }
        }));
    }

    /// Creates the event w/ the world and returns the entity
    /// 
    pub fn create(&self, world: &World) -> Option<Entity> {
        (self.create_fn)(world, |_|{})
    }

    /// Creates and schedules an event to start
    /// 
    pub fn schedule(&self, world: &World) -> Option<Entity> {
        if let Some(created) = self.create(world) {
            Runtime::start_event(created, world);

            Some(created)
        } else {
            event!(Level::WARN, "did not schedule event {}", self.event);
            None
        }
    }

    /// Returns the event's plugin thunk
    /// 
    pub fn thunk(&self) -> Thunk {
        self.event.thunk()
    }
}

impl Runtime {
    /// Returns a runtime from a project, with no plugins installed
    /// 
    pub fn new(project: Project) -> Self {
        Self {
            project,
            engine_plugin: BTreeMap::default(),
            config: BTreeMap::default(),
        }
    }

    /// Creates a new event source
    /// 
    pub fn event_source<'a, E, P>(&'a self) -> EventSource 
    where
        E: Engine,
        P: Plugin + Send + Default
    {
        EventSource {
            create_fn: E::create::<P>,
            event: E::event::<P>(),
            runtime: Arc::new(self.clone()),
        }
    }

    /// Install an engine into the runtime. An engine provides functions for creating new component instances.
    pub fn install<E, P>(&mut self)
    where
        E: Engine,
        P: Plugin + Send + Default,
    {
        let event = E::event::<P>();
        self.engine_plugin.insert(event.to_string(), E::create::<P>);

        event!(Level::INFO, "install event: {}", event.to_string());
    }

    /// Registers a config w/ this runtime
    pub fn add_config(&mut self, config: Config) {
        let Config(name, config_fn) = config;

        self.config.insert(name.to_string(), config_fn);
    }

    fn find_config_and_create(
        &self,
        world: &World,
        block_name: impl AsRef<str>,
        config_name: impl AsRef<str>,
        create_event: CreateFn,
    ) -> Option<Entity> {
        if let Some(created) = self
            .config
            .get(config_name.as_ref())
            .and_then(|c| create_event(world, *c))
        {
            let mut tc = world.write_component::<ThunkContext>();

            if let Some(tc) = tc.get_mut(created) {
                tc.block.block_name = block_name.as_ref().to_string();
            }

            Some(created)
        } else {
            None
        }
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
                for (name, value) in config_block.as_ref().iter_attributes().filter_map(|a| {
                    if a.is_stable() {
                        Some((a.name(), a.value()))
                    } else {
                        None
                    }
                }) {
                    tc.as_mut().with(name, value.clone());
                }

                for a in config_block
                    .as_ref()
                    .iter_attributes()
                    .filter(|a| !a.is_stable())
                {
                    if let Some((_, value)) = a.transient() {
                        if let Value::Symbol(symbol) = a.value() {
                            let symbol = symbol.trim_end_matches("::");
                            let name = a.name().trim_end_matches(&format!("::{symbol}"));

                            tc.as_mut().define(name, symbol).edit_as(value.clone());
                        }
                    }
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
    fn menu(&mut self, _: &Ui) {}

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
                if let Some(created) = self.create_with_fn(world, event, config_fn) {
                    world
                        .write_component()
                        .insert(created, Sequence::default())
                        .ok();
                    world
                        .write_component()
                        .insert(created, Connection::default())
                        .ok();
                }
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
    /// Starts an event in the world w/ an entity
    ///
    /// Caveats: This will write to the Event component, so it cannot be called if there Event is already borrowed.
    /// It's safest after calling creat_event
    pub fn start_event(event: Entity, world: &World) {
        if let Some(context) = world.read_component::<ThunkContext>().get(event) {
            if let Some(event) = world.write_component::<Event>().get_mut(event) {
                event.fire(context.to_owned());
            }
        }
    }

    /// Creates a group of engines
    pub fn create_engine_group<E>(&self, world: &World, blocks: Vec<String>) -> Vec<Entity>
    where
        E: Engine,
    {
        let mut created = vec![];

        for block in blocks.iter() {
            if let Some(next) = self.create_engine::<E>(world, block.to_string()) {
                created.push(next);
            }
        }

        created
    }

    /// Creates a new engine,
    /// an engine is defined by a sequence of events.
    pub fn create_engine<E>(&self, world: &World, sequence_block_name: String) -> Option<Entity>
    where
        E: Engine,
    {
        if let Some(block) = self.project.find_block(&sequence_block_name) {
            if let Some(mut engine_root) = block.get_block(E::event_name()) {
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
                    if let Some((_, block_symbol)) = block_address.split_once("::") {
                        let engine_plugin_key = format!("{} {}", "call", block_symbol);
                        if let Some(create_fn) = self.engine_plugin.get(&engine_plugin_key) {
                            if let Some(created) =
                                self.create_plugin(world, block_address, value.clone(), *create_fn)
                            {
                                event!(
                                    Level::DEBUG, 
                                    "create event:\n\t{}\n\t{}\n\t{}\n\tconfig: {:?}",
                                    created.id(),
                                    block_name,
                                    engine_plugin_key,
                                    &value
                                );
                                engine_root.add_int_attr(block_name, created.id() as i32);
                                sequence.add(created);
                            }
                        }
                    }
                }

                engine_root.add_text_attr("sequence_name", &sequence_block_name);
                return E::initialize_sequence(engine_root, sequence, world);
            }
        } else {
            event!(Level::ERROR, "{} block not found", &sequence_block_name);
        }

        None
    }

    fn find_plugin<E>(&self, symbol: &String) -> Option<CreateFn>
    where
        E: Engine,
    {
        let engine_plugin_key = format!("{} {}", E::event_name(), symbol);
        self.engine_plugin
            .get(&engine_plugin_key)
            .and_then(|f| Some(*f))
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
                    if let Some(block) = blocks.find_block(block_name) {
                        let config = block
                            .get_block(plugin_symbol)
                            .and_then(|b| b.find_text("config"));
                        if let Some(config_name) = config {
                            return self.find_config_and_create(
                                world,
                                block.block_name,
                                config_name,
                                create_event,
                            );
                        }

                        let config_block = block
                            .get_block(plugin_symbol)
                            .and_then(|b| b.find_symbol("config"))
                            .and_then(|b| blocks.find_block(b));

                        if let Some(config_block) = config_block {
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
                    let mut block_address = BlockIdentifier::lexer(block_address.as_ref());
                    loop {
                        match block_address.next() {
                            Some(block_address) => match block_address {
                                BlockIdentifier::Prefix | BlockIdentifier::Seperator => {
                                    continue;
                                }
                                BlockIdentifier::Name(block_name) => {
                                    return self.find_config_and_create(
                                        world,
                                        block_name,
                                        config_name,
                                        create_event,
                                    );
                                }
                                BlockIdentifier::Error => return None,
                            },
                            None => return None,
                        };
                    }
                }
                atlier::system::Value::Symbol(config_block_name) => {
                    if let Some(config_block) = blocks.find_block(config_block_name) {
                        return self.find_config_block_and_create(
                            world,
                            block_name,
                            config_block,
                            create_event,
                        );
                    }
                }
                _ => {
                    event!(Level::WARN, "No config found for {}", block_address.as_ref());
                }
            }
        }

        None
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

    /// Creates a new event, returns an entity if successful
    pub fn create_event_with<E, P>(
        &self,
        world: &World,
        config_fn: fn(&mut ThunkContext),
    ) -> Option<Entity>
    where
        E: Engine,
        P: Plugin<ThunkContext> + Component + Send + Default,
    {
        self.create_with_fn(world, &E::event::<P>(), config_fn)
    }

    /// Schedule a new event on this runtime, returns an entity if the event was created/started
    pub fn schedule(
        &self,
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
        &self,
        world: &World,
        event: &Event,
        config_name: &'static str,
    ) -> Option<Entity> {
        if let Some(entity) = self.create_with_name(world, event, config_name) {
            Self::start_event(entity, world);
            Some(entity)
        } else {
            None
        }
    }

    /// Creates and starts a new event, and returns the entity if successful
    pub fn schedule_with_engine<E, P>(
        &self,
        world: &World,
        config_name: &'static str,
    ) -> Option<Entity>
    where
        E: Engine,
        P: Plugin<ThunkContext> + Component + Send + Default,
    {
        self.schedule_with_config(world, &E::event::<P>(), config_name)
    }
}

impl Plugin<ThunkContext> for Runtime {
    fn symbol() -> &'static str {
        "runtime"
    }

    fn description() -> &'static str {
        "Starts a runtime w/ it's own standalone world"
    }

    fn call_with_context(context: &mut ThunkContext) -> Option<AsyncContext> {
        context.clone().task(|cancel_source| {
            let tc = context.clone();
            async move {
                if let Some(project_src) = tc.as_ref().find_text("project_src") {
                    if let Some(project) = Project::load_file(project_src) {
                        let mut runtime = Runtime::new(project);
                        runtime.install::<Call, WriteFile>();
                        runtime.install::<Call, OpenFile>();
                        runtime.install::<Call, OpenDir>();
                        runtime.install::<Call, Process>();
                        runtime.install::<Call, Remote>();
                        runtime.install::<Call, Timer>();
                        runtime.install::<Call, Runtime>();
                        runtime.install::<Call, Expect>();
                        runtime.install::<Call, Println>();

                        // TODO - add some built in configs -

                        runtime.start::<Call>(&tc, cancel_source);
                    }
                }

                Some(tc)
            }
        })
    }
}

impl Runtime {
    /// Starts the runtime w/ the runtime editor extension
    pub fn start<E>(self, tc: &ThunkContext, cancel_source: tokio::sync::oneshot::Receiver<()>) 
        where 
        E: Engine
    {
        let mut runtime_editor = RuntimeEditor::new(self);

        Self::start_with::<RuntimeEditor, E>(
            &mut runtime_editor, 
            "runtime".to_string(), 
            tc, 
            cancel_source
        );
    }

    /// Starts the runtime and extension w/ a thunk_context and cancel_source
    /// Can be used inside a plugin to customize a runtime.
    pub fn start_with<Ext, E>(
        extension: &mut Ext,
        block_symbol: String,
        tc: &ThunkContext,
        mut cancel_source: tokio::sync::oneshot::Receiver<()>,
    ) where
        Ext: Extension + AsRef<Runtime>,
        E: Engine
    {
        let project = &extension.as_ref().project;

        let mut call_names = vec![];
        let mut connections = vec![];
        for (_, block) in project.iter_block() {
            if let Some(runtime_block) = block.get_block(&block_symbol) {
                for (engine_address, value) in runtime_block.find_symbol_values(E::event_name()) {
                    if let Some((engine_name, _)) = engine_address.split_once("::") {
                        call_names.push(engine_name.to_string());

                        if let Value::Symbol(connect_to) = value {
                            connections.push((engine_name.to_string(), connect_to));
                        }
                    }
                }
            }
        }

        let (mut world, mut dispatcher_builder) = E::standalone::<Ext>();

        if tc.as_ref().is_enabled("proxy_dispatcher").unwrap_or_default() {
            dispatcher_builder.add(ProxyDispatcher::from(tc.clone()), "proxy_dispatcher", &[]);
        }

        let mut dispatcher = dispatcher_builder.build();
        dispatcher.setup(&mut world);

        let mut engine_table = HashMap::<String, Entity>::default();

        for engine in call_names {
            if let Some(start) = extension
                .as_ref()
                .create_engine::<Call>(&world, engine.to_string())
            {
                engine_table.insert(engine, start);
            }
        }

        let mut schedule = vec![];
        let mut ignore = HashSet::<Entity>::default();
        // Connect sequences
        {
            let mut sequences = world.write_component::<Sequence>();
            for (from, to) in connections {
                if let Some(from) = engine_table.get(&from) {
                    if let Some(to) = engine_table.get(&to) {
                        if let Some(sequence) = (&mut sequences).get_mut(*from) {
                            sequence.set_cursor(*to);
                            ignore.insert(*to);

                            if !ignore.contains(from) {
                                schedule.push(*from);
                                ignore.insert(*from);
                                event!(Level::INFO, "schedule event:\n\t{} -> {}", from.id(), to.id());
                            }
                        }
                    }
                }
            }
        }

        // Start beginning of events
        {
            let contexts = world.read_component::<ThunkContext>();
            let mut events = world.write_component::<Event>();
            for e in schedule {
                if let Some(event) = events.get_mut(e) {
                    if let Some(context) = contexts.get(e) {
                        event.fire(context.clone());
                    }
                }
            }
        }

        event!(Level::INFO, "Starting loop");
        loop {
            dispatcher.dispatch(&world);
            extension.on_run(&world);

            world.maintain();
            extension.on_maintain(&mut world);

            if ThunkContext::is_cancelled(&mut cancel_source) {
                event!(Level::INFO, "Cancelling loop");
                break;
            }
        }

        if let Some(runtime) = world.remove::<tokio::runtime::Runtime>() {
            if let Some(handle) = tc.handle() {
                // dropping a tokio runtime needs to happen in a blocking context
                handle.spawn_blocking(move || {
                    runtime.shutdown_timeout(Duration::from_secs(5));
                });
            }
        }
    }
}

#[derive(Logos, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BlockIdentifier {
    /// Used for ordering, workaround for how things are stored in btree table
    /// ex. aa_{name}::{symbol}
    #[regex(r"[a-z]*[a-z]_")]
    Prefix,
    /// Name of the block
    #[regex(r"[a-z-.0-9]+", from_block_identifier)]
    Name(String),
    /// Seperator between name and symbol
    #[token("::")]
    Seperator,
    // Logos requires one token variant to handle errors,
    // it can be named anything you wish.
    #[error]
    // We can also use this variant to define whitespace,
    // or any other matches we wish to skip.
    #[regex(r"[ :\t\n\f]+", logos::skip)]
    Error,
}

fn from_block_identifier(lexer: &mut Lexer<BlockIdentifier>) -> Option<String> {
    Some(lexer.slice().trim_end_matches("::").to_string())
}

#[test]
fn test_block_identifier() {
    let mut block_identifier = BlockIdentifier::lexer("a_azcli:jinja2::install");

    assert_eq!(block_identifier.next(), Some(BlockIdentifier::Prefix));
    assert_eq!(
        block_identifier.next(),
        Some(BlockIdentifier::Name("azcli".to_string())),
    );

    assert_eq!(
        block_identifier.next(),
        Some(BlockIdentifier::Name("jinja2".to_string())),
    )
}
