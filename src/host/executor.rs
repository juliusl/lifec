use crate::AttributeGraph;
use crate::AttributeIndex;
use crate::Host;
use crate::Operation;
use crate::Sequence;
use crate::Thunk;
use crate::ThunkContext;
use crate::WorldExt;
use reality::Block;
use specs::Entity;
use specs::World;
use std::collections::HashMap;
use tokio::select;
use tokio::sync::oneshot;
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

        let handle = thunk_context.handle().expect("should be a handle").clone();

        let thunk_components = self.as_ref().read_component::<Thunk>();
        let graph_components = self.as_ref().read_component::<AttributeGraph>();
        let block_components = self.as_ref().read_component::<Block>();
        let mut thunks = HashMap::<Entity, Thunk>::default();
        let mut graphs = HashMap::<Entity, AttributeGraph>::default();
        let mut blocks = HashMap::<Entity, Block>::default();
        for call in calls.iter_entities() {
            let thunk = thunk_components.get(call).expect("should have a thunk").clone();
            let graph = graph_components.get(call).expect("should have a graph").clone();
            let block = block_components.get(call).expect("should have a block").clone();
            thunks.insert(call, thunk);
            graphs.insert(call, graph);
            blocks.insert(call, block);
        }

        let (tx, rx) = oneshot::channel::<()>();

        let task = handle.spawn(async move {
            let mut context = thunk_context.clone();
            let handle = context.handle().expect("should be a handle");
            let mut cancel_source = Some(rx);

            for e in calls.iter_entities() {
                if let Some(mut _rx) = cancel_source.take() {
                    let (_tx, rx) = oneshot::channel::<()>();

                    context = context.enable_async(e, handle.clone());

                    let thunk = thunks.get(&e).expect("should have a thunk");
                    let graph = graphs.get(&e).expect("should exist");
                    let block = blocks.get(&e).expect("should exist");

                    let mut operation = Operation::empty(handle.clone())
                        .start_with(thunk, &context.with_state(graph.clone()).with_block(block));

                    {
                        let _rx = &mut _rx;
                        select! {
                            result = operation.task(rx) => {
                                match result {
                                    Some(result) => context = result.commit(),
                                    None => {
                                    }
                                }
                            },
                            _ = _rx => {
                                _tx.send(()).ok();
                                break;
                            }
                        }
                    }

                    cancel_source = Some(_rx);
                } else {
                    break;
                }
            }

            context
        });

        (task, tx)
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
