use std::{collections::BTreeMap, ops::Deref};

use crate::{AttributeParser, BlockObject, CustomAttribute, SpecialAttribute, Plugin};
use specs::{Component, DefaultVecStorage, WorldExt};
use tracing::event;
use tracing::Level;

mod event_source;
pub use event_source::EventSource;

/// Runtime provides access to the underlying project, and function tables for creating components
///
#[derive(Component, Default, Clone)]
#[storage(DefaultVecStorage)]
pub struct Runtime {
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
    /// Installs a plugin on this runtime,
    /// 
    pub fn install<P>(&mut self, event_name: &'static str)
    where
        P: Plugin + Send + Default,
    {
        let mut event_name = event_name.to_string();
        if event_name.is_empty() {
            event_name = "call".to_string();
        }
        
        // Register event sources
        self.plugins.insert(
            format!("{}::{}", &event_name, P::symbol()),  
            EventSource::new::<P>(self.clone(), &event_name)
        );
    }

    /// Installs a plugin on this runtime and also adds the plugin as a custom attribute,
    /// 
    /// If the plugin is using the parser feature, an entity and event component
    /// is created when the runmd block is compiled. Otherwise the event source
    /// is required.
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
    use crate::*;
    
    let mut runtime = Runtime::default();
    runtime.install_with_custom::<Process>("");
    runtime.install_with_custom::<Install>("");

    let mut world = specs::World::new();
    world.register::<Runtime>();
    world.register::<Event>();
    world.insert(runtime);

    let parser = Parser::new_with(world).with_special_attr::<Runtime>();

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
            .read_component::<Block>()
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
                .read_component::<BlockProperties>()
                .get(process)
        );
        eprintln!(
            "{:#?}",
            world.read_component::<BlockIndex>().get(process)
        );
    }

    let process = world.entities().entity(3);
    {
        eprintln!(
            "{:#?}",
            world
                .read_component::<BlockProperties>()
                .get(process)
        );
    }
    // assert!(runtime.find_event_source("call println").is_some());
}
