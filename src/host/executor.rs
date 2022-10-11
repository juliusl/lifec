use crate::AttributeGraph;
use crate::AttributeIndex;
use crate::Event;
use crate::Host;
use crate::Operation;
use crate::Sequence;
use crate::Thunk;
use crate::ThunkContext;
use crate::Value;
use crate::WorldExt;
use crate::engine::Activity;
use specs::Entity;
use specs::World;
use std::collections::HashMap;
use tokio::select;
use tokio::sync::oneshot;
use tokio::sync::oneshot::Sender;
use tokio::task::JoinHandle;
use tracing::{event, Level};

/// Trait for executing a sequence of events,
///
pub trait Executor {
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
    ) -> (JoinHandle<ThunkContext>, Sender<()>)
    where
        Self: AsRef<World>,
    {
        let thunk_context = thunk_context.commit();

        let handle = thunk_context.handle().expect("should be a handle").clone();

        let event_components = self.as_ref().read_component::<Event>();
        let graph_components = self.as_ref().read_component::<AttributeGraph>();
        let mut events = HashMap::<Entity, Event>::default();
        let mut graphs = HashMap::<Entity, AttributeGraph>::default();

        for call in calls.iter_entities() {
            let event = event_components
                .get(call)
                .expect("should exist")
                .duplicate();
            let graph = graph_components.get(call).expect("should exist").clone();

            events.insert(call, event);
            graphs.insert(call, graph);
        }

        let (tx, rx) = oneshot::channel::<()>();

        let task = handle.spawn(async move {
            let mut thunk_context = thunk_context.clone();
            let handle = thunk_context.handle().expect("should be a handle");
            let mut cancel_source = Some(rx);

            for e in calls.iter_entities() {
                if let Some(mut _rx) = cancel_source.take() {
                    let (_tx, rx) = oneshot::channel::<()>();

                    thunk_context = thunk_context.enable_async(e, handle.clone());

                    let Event(event_name, Thunk(plugin_name, call, ..), ..) =
                        events.get(&e).expect("should exist");
                    let graph = graphs.get(&e).expect("should exist");

                    event!(Level::DEBUG, "Starting {event_name} {plugin_name}");

                    let mut operation = Operation {
                        context: thunk_context.clone(),
                        task: call(&thunk_context.with_state(graph.clone())),
                    };

                    {
                        let _rx = &mut _rx;
                        select! {
                            result = operation.task(rx) => {
                                match result {
                                    Some(context) => thunk_context = context.commit(),
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

            thunk_context
        });

        (task, tx)
    }
}

impl Executor for Host {
    fn execute(&self, thunk_context: &ThunkContext) -> (JoinHandle<ThunkContext>, Sender<()>) {
        let mut sequence = Sequence::default();
        {
            let entities = self.world().entities();
            for call in thunk_context
                .state()
                .find_values("sequence")
                .iter()
                .filter_map(|v| {
                    if let Value::Int(i) = v {
                        Some(entities.entity(*i as u32))
                    } else {
                        None
                    }
                })
            {
                sequence.add(call);
            }
        }

        self.execute_sequence(thunk_context, sequence)
    }
}
