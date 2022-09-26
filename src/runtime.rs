use std::{collections::BTreeMap, ops::Deref, sync::Arc};

use reality::{AttributeParser, BlockObject, CustomAttribute, SpecialAttribute};
use specs::{Component, DefaultVecStorage, WorldExt};
use tracing::event;
use tracing::Level;

use crate::{
    Event, EventSource, Plugin, Project, 
};

/// Runtime provides access to the underlying project, and function tables for creating components
///
#[derive(Component, Default, Clone)]
#[storage(DefaultVecStorage)]
pub struct Runtime {
    /// Project loaded w/ this runtime, typically from a .runmd file
    /// The project file usually contains different configurations for event scheduling
    project: Project,
    /// Table of functions for creating new event components
    plugins: BTreeMap<String, EventSource>,
    /// Set of custom attributes to use, added from install()
    custom_attributes: BTreeMap<String, CustomAttribute>,
}

impl SpecialAttribute for Runtime {
    fn ident() -> &'static str {
        "runtime"
    }

    fn parse(parser: &mut AttributeParser, _: impl AsRef<str>) {
        // Converts all installed plugins
        if let Some(world) = parser.world() {
            if let Some(entity) = parser.entity() {
                // First, check to see if the entity has a runtime component
                let runtime = world
                    .read_component::<Runtime>()
                    .get(entity)
                    .and_then(|r| Some(r.clone()))
                    // Otherwise, check to see if the world has a runtime resource
                    .unwrap_or(world.read_resource::<Runtime>().deref().clone());

                for (_, c) in runtime.custom_attributes {
                    parser.add_custom(c);
                }
            }
        }
    }
}

impl Runtime {
    /// Returns a runtime from a project, with no plugins installed
    ///
    pub fn new(project: Project) -> Self {
        Self {
            project,
            plugins: BTreeMap::default(),
            custom_attributes: BTreeMap::default(),
        }
    }

    /// Creates a new event source
    ///
    pub fn event_source<'a, P>(&'a self, event_name: &'static str) -> EventSource
    where
        P: Plugin + Send + Default,
    {
        EventSource {
            event: Event::from_plugin::<P>(event_name),
            runtime: Arc::new(self.clone()),
            setup: None,
        }
    }

    /// Install an engine into the runtime. An engine provides functions for creating new component instances.
    pub fn install<P>(&mut self, event_name: &'static str)
    where
        P: Plugin + Send + Default,
    {
        // Register event sources
        let event_source = self.event_source::<P>(event_name);
        self.plugins.insert(format!("{}::{}", event_name, P::symbol()), event_source);
    }

    /// Installs a plugin and custom attribute to the runtime,
    ///
    pub fn install_with_custom<P>(&mut self, event_name: &'static str)
    where
        P: Plugin + BlockObject + Send + Default,
    {
        self.install::<P>(event_name);

        if let Some(custom_attr) = P::default().parser() {
            self.custom_attributes
                .insert(custom_attr.ident(), custom_attr);

            event!(Level::INFO, "install custom attribute: .{}", P::symbol());
        }
    }
}

#[test]
#[tracing_test::traced_test]
fn test_runtime() {
    use crate::Process;
    use crate::Install;
    
    let mut runtime = Runtime::default();
    runtime.install_with_custom::<Process>("call");
    runtime.install_with_custom::<Install>("call");

    let mut world = specs::World::new();
    world.register::<Runtime>();
    world.insert(runtime);

    let parser = reality::Parser::new_with(world).with_special_attr::<Runtime>();

    let parser = parser.parse(
        r#"
    ``` test plugin
    : description .symbol This is a test plugin

    + .runtime
    : .process cargo update 
    : .process cargo build 
    : RUST_LOG .env lifec=trace
    : WORK_DIR .env .world
    : .install test.sh
    ```
    "#,
    );

    let world = parser.commit();
    let process = world.entities().entity(1);
    {
        // TODO: Write assertions
        let block = world
            .read_component::<reality::Block>()
            .get(process)
            .unwrap()
            .clone();
        eprintln!("{:#?}", block.index());
        eprintln!("{:#?}", block.map_control());
    }

    let process = world.entities().entity(2);
    {
        eprintln!(
            "{:#?}",
            world
                .read_component::<reality::BlockProperties>()
                .get(process)
        );
        eprintln!(
            "{:#?}",
            world.read_component::<reality::BlockIndex>().get(process)
        );
    }

    let process = world.entities().entity(3);
    {
        eprintln!(
            "{:#?}",
            world
                .read_component::<reality::BlockProperties>()
                .get(process)
        );
    }
    // assert!(runtime.find_event_source("call println").is_some());
}
