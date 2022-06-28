pub use atlier::system::{App, Extension, WindowEvent};
pub use specs::{
    Builder, Component, DispatcherBuilder, Entities, Entity, EntityBuilder, Join, ReadStorage,
    System, World, WorldExt, WriteStorage,
};

mod block;
pub use block::BlockContext;
pub use block::Project;

mod events;
pub use events::Event;
pub use events::EventRuntime;

mod nodes;
pub use nodes::Node;
pub use nodes::NodeContext;

mod process;
pub use process::Process;

mod thunks;
pub use thunks::OpenFile;
pub use thunks::OpenDir;
pub use thunks::Println;
pub use thunks::StatusUpdate;
pub use thunks::Thunk;
pub use thunks::ThunkContext;
pub use thunks::WriteFiles;

pub mod demos {
    pub use super::thunks::demo::*;
}

mod render;
pub use render::Display;
pub use render::Edit;
pub use render::Render;
use tokio::task::JoinHandle;

use crate::AttributeGraph;

/// This trait is to facilitate extending working with the attribute graph
pub trait Plugin<T>
where
    T: AsRef<AttributeGraph>
        + AsMut<AttributeGraph>
        + From<AttributeGraph>
        + Component
        + Send
        + Sync,
{
    /// Returns the symbol name representing this plugin
    fn symbol() -> &'static str;

    /// implement call_with_context to allow for static extensions of attribute graph
    fn call_with_context(context: &mut T) -> Option<JoinHandle<T>>;

    /// Returns a short string description for this plugin
    fn description() -> &'static str {
        ""
    }

    /// Transforms attribute graph into a thunk context and calls call_with_context
    /// Updates graph afterwards.
    fn call(attributes: &mut AttributeGraph) {
        use crate::RuntimeState;

        let mut context = T::from(attributes.clone());
        let context = &mut context;
        Self::call_with_context(context);

        *attributes = attributes.merge_with(context.as_ref());
    }

    /// Parses entity from a .runmd file and add's T as a component from the parsed graph.
    /// Calls handle to handle any actions on the graph before T::from(graph)
    /// Calls init to complete building the entity.
    fn parse_entity(
        path: impl AsRef<str>,
        world: &mut World,
        setup: impl FnOnce(&mut AttributeGraph),
        init: impl FnOnce(EntityBuilder) -> Entity,
    ) -> Option<Entity> {
        if let Some(mut graph) = AttributeGraph::load_from_file(path) {
            setup(&mut graph);
            let context = T::from(graph);

            let entity = world.create_entity().with(context);
            Some(init(entity))
        } else {
            None
        }
    }
}

/// The engine trait is to enable an event struct to be created which handles the dynamics for an entity
pub trait Engine {
    /// The name of the event this engine produces
    fn event_name() -> &'static str;

    /// Setup graph
    fn setup(_: &mut AttributeGraph) {
        // No-op
        // Note: Left as an extension point, but mainly shouldn't be needed
    }

    /// Initialize event for entity
    fn init_event(entity: EntityBuilder, event: Event) -> EntityBuilder {
        entity.with(event)
    }

    /// Create an event with an entity
    fn create_event(_: Entity, _: &World) {}

    /// Initialize an instance of this engine
    fn init<P>(world: &mut World, config: fn(&mut ThunkContext))
    where
        P: Plugin<ThunkContext> + Component + Send + Default,
    {
        let mut initial_context = ThunkContext::default();
        config(&mut initial_context);
        let entity = Self::init_event(world.create_entity(), Self::event::<P>()).build();
        initial_context.as_mut().set_parent_entity(entity);

        match world
            .write_component::<ThunkContext>()
            .insert(entity, initial_context)
        {
            Ok(_) => {}
            Err(_) => {}
        }
    }

    /// Creates an instance of this engine
    fn create<P>(world: &World, config: fn(&mut ThunkContext)) -> Entity
    where
        P: Plugin<ThunkContext> + Component + Send + Default,
    {
        let entities = world.entities();
        let mut events = world.write_component::<Event>();

        let entity = entities.create();

        match events.insert(entity, Self::event::<P>()) {
            Ok(_) => {}
            Err(_) => {}
        }

        Self::create_event(entity, world);

        let mut initial_context = ThunkContext::default();
        config(&mut initial_context);
        initial_context.as_mut().set_parent_entity(entity);

        match world
            .write_component::<ThunkContext>()
            .insert(entity, initial_context)
        {
            Ok(_) => {}
            Err(_) => {}
        }

        entity
    }

    /// Returns an event that runs the engine
    fn event<P>() -> Event
    where
        P: Plugin<ThunkContext> + Component + Send + Default,
    {
        Event::from_plugin::<P>(Self::event_name())
    }

    /// Parses a .runmd plugin entity, and then sets up the entity's event component
    fn parse_engine<P>(path: impl AsRef<str>, world: &mut World) -> Option<Entity>
    where
        P: Plugin<ThunkContext> + Component + Send + Default,
    {
        P::parse_entity(path, world, Self::setup, |entity| {
            Self::init_event(entity, Self::event::<P>()).build()
        })
    }
}
