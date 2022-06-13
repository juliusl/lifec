mod nodes;
pub use nodes::Node;
pub use nodes::NodeContext;

mod process;
pub use process::Process;

mod project;
pub use project::Document;
pub use project::Project;

mod thunks;
use specs::Entities;
use specs::Join;
use specs::System;
use specs::WriteStorage;
pub use thunks::Println;
pub use thunks::ThunkContext;
pub use thunks::WriteFiles;

pub mod demos {
    pub use super::thunks::demo::*;
}

mod render;
pub use render::Display;
pub use render::Edit;
pub use render::Render;

use crate::AttributeGraph;

pub trait Plugin<T>
where
    T: AsRef<AttributeGraph> + AsMut<AttributeGraph> + From<AttributeGraph>,
{
    /// Returns the symbol name representing this plugin
    fn symbol() -> &'static str;

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

    /// implement call_with_context to allow for static extensions of attribute graph
    fn call_with_context(context: &mut T);
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
                g.set_parent_entity(e, true);
            }
        }
    }
}
