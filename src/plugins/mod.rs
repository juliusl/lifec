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
use tokio::runtime::Handle;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;

use crate::AttributeGraph;

use self::events::StatusUpdate;

/// This trait is to facilitate extending working with the attribute graph
pub trait Plugin<T>
where
    T: AsRef<AttributeGraph> + AsMut<AttributeGraph> + From<AttributeGraph> + Component + Send + Sync,
{
    /// Returns the symbol name representing this plugin
    fn symbol() -> &'static str;

    /// implement call_with_context to allow for static extensions of attribute graph
    fn call_with_context(context: &mut T, handle: Option<Handle>) -> Option<JoinHandle<()>>;

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
        Self::call_with_context(context, None);

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

    /// Returns an event that runs the engine 
    fn event() -> Event {
        Event::from_plugin::<P>(Self::event_name(), Self::on_event)
    }

    /// Setup initial graph
    fn setup(_: &mut AttributeGraph) {
    }

    /// Parses a .runmd plugin entity, and then sets up the entity's event component
    fn parse_engine(path: impl AsRef<str>, world: &mut World) -> Option<Entity> {
        P::parse_entity(path, world, Self::setup, |entity|{
            entity.with(Self::event()).build()
        })
    }

    /// On event configures a tokio task, the default implementation simply calls the thunk
    fn on_event(entity: Entity, thunk: &Thunk, initial_context: &ThunkContext, status_updates: Sender<StatusUpdate>, handle: &Handle) -> JoinHandle<ThunkContext> {
        let thunk = thunk.clone();
        let initial_context = initial_context.clone();
        let thunk_handle = handle.clone();
        handle.spawn(async move {
            let progress_bar = ProgressBar(status_updates);
            progress_bar.update(entity, "started", 0.0).await;

            let mut context = initial_context;
            thunk.start(&mut context, thunk_handle).await;

            progress_bar.update(entity, "completed", 1.0).await;
            context
        })
    }
}
