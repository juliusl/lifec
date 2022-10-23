use crate::prelude::*;
use crate::engine::Plugins;
use specs::World;
use tokio::sync::oneshot::Sender;
use tokio::task::JoinHandle;

/// Trait for executing a sequence of events,
///
pub trait Executor
where
    Self: AsRef<World>,
{
    /// Executes a sequence of events,
    ///
    /// Looks for a `sequence` property in thunk context which is a list of plugin call entities,
    ///
    fn execute(&self, thunk_context: &ThunkContext) -> (JoinHandle<ThunkContext>, Sender<()>);

    /// Executes a events from a sequence,
    ///
    fn execute_sequence(
        &self,
        thunk_context: &ThunkContext,
        calls: Sequence,
    ) -> (JoinHandle<ThunkContext>, Sender<()>) {
        let thunk_context = thunk_context.commit();

        let plugins = self.as_ref().system_data::<Plugins>();

        plugins.start_sequence(&calls, Some(&thunk_context)).into()
    }
}

impl Executor for Host {
    fn execute(&self, thunk_context: &ThunkContext) -> (JoinHandle<ThunkContext>, Sender<()>) {
        let entities = self.as_ref().entities();

        let event = thunk_context
            .search()
            .find_int("event_id")
            .and_then(|i| Some(entities.entity(i as u32)))
            .expect("should have been registered with an event id");

        let sequence = self.as_ref().read_component::<Sequence>();
        let sequence = sequence.get(event).expect("should have a sequence");

        self.execute_sequence(thunk_context, sequence.clone())
    }
}
