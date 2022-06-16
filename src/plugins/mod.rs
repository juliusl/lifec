mod block;
pub use block::BlockContext;

mod files;
pub use files::FileContext;

mod nodes;
pub use nodes::Node;
pub use nodes::NodeContext;

mod store;
pub use store::StoreContext;

mod process;
pub use process::Process;

mod thunks;
use specs::Builder;
use specs::Component;
use specs::Entities;
use specs::Entity;
use specs::EntityBuilder;
use specs::Join;
use specs::System;
use specs::World;
use specs::WorldExt;
use specs::WriteStorage;
pub use thunks::Println;
pub use thunks::ThunkContext;
pub use thunks::WriteFiles;

pub mod demos {
    pub use super::thunks::demo::*;
    pub use super::nodes::demo::*;
}

mod render;
pub use render::Display;
pub use render::Edit;
pub use render::Render;

use crate::AttributeGraph;

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
    /// Calls init to complete building the entity.
    fn parse_entity(path: impl AsRef<str>, world: &mut World, init: impl Fn(EntityBuilder) -> Entity) -> Option<Entity> {
        if let Some(node) = AttributeGraph::load_from_file(path) {
            let context = T::from(node);

            let entity = world.create_entity().with(context);

            Some(init(entity))
        } else {
            None
        }
    }

    fn on_event(&mut self, context: &mut T) 
        where 
            Self: Engine + Sized
    {
        let attributes = context.as_mut();
        self.next_mut(attributes);
        self.exit(&attributes);
    }
}

/// An engine is a sequence of at least 2 events
pub trait Engine {
    /// next_mut is called after attributes has been updated
    fn next_mut(&mut self, attributes: &mut AttributeGraph);

    /// exit is always called, regardless if attributes has been updated
    fn exit(&mut self, attributes: &AttributeGraph);
}

/// Ensure attribute graph id is synced to its parent entity
pub struct AttributeGraphSync;

impl<'a> System<'a> for AttributeGraphSync {
    type SystemData = (Entities<'a>, WriteStorage<'a, AttributeGraph>);

    fn run(&mut self, (entities, mut graphs): Self::SystemData) {
        for (e, g) in (&entities, &mut graphs).join() {
            if g.entity() != e.id() {
                g.set_parent_entity(e);
            }
        }
    }
}
