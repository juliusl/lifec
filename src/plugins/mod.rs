use specs::{Builder, Component, Entity, EntityBuilder, World, WorldExt};

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
pub use thunks::Println;
pub use thunks::ThunkContext;
pub use thunks::WriteFiles;
pub use thunks::Thunk;
pub use thunks::StatusUpdate;

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
    T: AsRef<AttributeGraph> + AsMut<AttributeGraph> + From<AttributeGraph> + Component + Send + Sync,
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
    fn parse_entity(path: impl AsRef<str>, world: &mut World, handle: impl FnOnce(&mut AttributeGraph), init: impl Fn(EntityBuilder) -> Entity) -> Option<Entity> {
        if let Some(mut graph) = AttributeGraph::load_from_file(path) {
            handle(&mut graph);
            let context = T::from(graph);

            let entity = world.create_entity().with(context);
            Some(init(entity))
        } else {
            None
        }
    }
}

/// The engine trait is to enable an event struct to be created which handles the dynamics for an entity
pub trait Engine<P>
where
    P: Plugin<ThunkContext> + Component + Send + Default,
{
    /// The name of the event this engine produces
    fn event_name() -> &'static str;

    /// Setup graph
    fn setup(_: &mut AttributeGraph);

    /// Returns an event that runs the engine 
    fn event() -> Event {
        Event::from_plugin::<P>(Self::event_name())
    }

    /// Parses a .runmd plugin entity, and then sets up the entity's event component
    fn parse_engine(path: impl AsRef<str>, world: &mut World) -> Option<Entity> {
        P::parse_entity(path, world, Self::setup, |entity|{
            entity.with(Self::event()).build()
        })
    }
}
