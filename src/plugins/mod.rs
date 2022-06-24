use specs::{Builder, Component, Entity, EntityBuilder, World, WorldExt};

mod block;
pub use block::BlockContext;
pub use block::Project;

mod events;
pub use events::Event;
pub use events::EventRuntime;
pub use events::ProgressBar;
pub use events::Progress;

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

pub mod demos {
    pub use super::thunks::demo::*;
}

mod render;
pub use render::Display;
pub use render::Edit;
pub use render::Render;

use crate::AttributeGraph;

/// This trait is to facilitate extending working with the attribute graph
pub trait Plugin<T>
where
    T: AsRef<AttributeGraph> + AsMut<AttributeGraph> + From<AttributeGraph> + Component + Send + Sync,
{
    /// Returns the symbol name representing this plugin
    fn symbol() -> &'static str;

    /// implement call_with_context to allow for static extensions of attribute graph
    fn call_with_context(context: &mut T);

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
pub trait Engine
where
    Self: Plugin<ThunkContext> + Component + Send + Default,
    <Self as specs::Component>::Storage: Default
{
    /// The name of the event this engine produces
    fn event_name() -> &'static str;

    fn event() -> Event {
        Event::from_plugin::<Self>(Self::event_name())
    }

    fn parse_engine(path: impl AsRef<str>, world: &mut World) -> Option<Entity> {
        Self::parse_entity(path, world, |_|{}, |entity|{
            entity.with(Self::event()).build()
        })
    }
}