pub use atlier::system::{App, Extension, WindowEvent};
use specs::{
    Builder, Component, Entity, EntityBuilder, Join,
    System, World, WorldExt,
};

mod block;
pub use block::BlockContext;
pub use block::Project;

mod events;
pub use events::Event;
pub use events::EventRuntime;
pub use events::Listen;
pub use events::Sequence;
pub use events::Connection;

mod process;
pub use process::Process;
pub use process::Remote;

mod thunks;
pub use thunks::Config;
pub use thunks::CancelThunk;
pub use thunks::OpenDir;
pub use thunks::OpenFile;
pub use thunks::WriteFile;
pub use thunks::StatusUpdate;
pub use thunks::Thunk;
pub use thunks::ThunkContext;
pub use thunks::Timer;

use crate::AttributeGraph;
use crate::editor::List;
use crate::editor::Task;
use tokio::task::JoinHandle;

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
    fn call_with_context(context: &mut T) -> Option<(JoinHandle<T>, tokio::sync::oneshot::Sender<()>)>;

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
        // Note: Left as an extension point, but mainly shouldn't need to be implemented
    }

    /// Initialize event for entity, this is called by init<P>
    /// By default this method inserts the event as a component w/ the entity being built
    /// Implement to additional logic. Note: that you must add the event component to the entity being built.
    fn init_event(entity: EntityBuilder, event: Event) -> EntityBuilder {
        entity.with(event)
    }

    /// Create an event with an entity, this is called by create<P>
    /// By default this is a no-op. Implement to add additional logic to create<P>
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
    fn create<P>(world: &World, config: fn(&mut ThunkContext)) -> Option<Entity>
    where
        P: Plugin<ThunkContext> + Component + Send + Default,
    {
        let entities = world.entities();
        let mut events = world.write_component::<Event>();

        let entity = entities.create();

        match events.insert(entity, Self::event::<P>()) {
            Ok(_) => {
                Self::create_event(entity, world);

                let mut initial_context = ThunkContext::default();
                config(&mut initial_context);
                initial_context.as_mut().set_parent_entity(entity);

                match world
                    .write_component::<ThunkContext>()
                    .insert(entity, initial_context)
                {
                    Ok(_) => Some(entity),
                    Err(err) => {
                        eprintln!(
                            "could not finish creating event {}, {}, src_desc: inserting context",
                            P::symbol(),
                            err
                        );
                        entities.delete(entity).ok();
                        None
                    }
                }
            }
            Err(err) => {
                eprintln!(
                    "could not finish creating event {}, {}, src_desc: inserting event",
                    P::symbol(),
                    err
                );
                entities.delete(entity).ok();
                None
            }
        }
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

    /// Initializes the sequence and returns the first entity for the first event
    fn initialize_sequence(
        engine_root: impl AsRef<AttributeGraph>,
        mut sequence: Sequence,
        world: &World,
    ) -> Option<Entity> {
        let sequence_list = sequence.clone();
        if let Some(first) = sequence.next() {
            if let Some(true) = engine_root.as_ref().is_enabled("repeat") {
                sequence.set_cursor(first);
            }
            
            if let Some(text) = engine_root.as_ref().find_text("repeat") {
                eprintln!("Looking for {}, to set cursor", text);
                if let Some(entity) = engine_root.as_ref().find_int(text) {
                    let entity = world.entities().entity(entity as u32);
                    sequence.set_cursor(entity);               
                    eprintln!("Cursor found");
                } else {
                    eprintln!("Cursor not found");
                }
            }
            let mut sequence_list = List::<Task>::edit_block_view(Some(sequence_list));

            if let Some(sequence_name) = engine_root.as_ref().find_text("sequence_name") {
                sequence_list.set_title(sequence_name);
            }

            world
                .write_component::<Sequence>()
                .insert(first, sequence.clone())
                .ok();
            world
                .write_component::<List<Task>>()
                .insert(first, sequence_list)
                .ok();

            Some(first)
        } else {
            None
        }
    }
}
