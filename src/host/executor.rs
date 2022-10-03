use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::AttributeGraph;
use crate::AttributeIndex;
use crate::Event;
use crate::Host;
use crate::Operation;
use crate::Thunk;
use crate::ThunkContext;
use crate::Value;
use crate::WorldExt;
use specs::Entity;
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
    fn execute(&mut self, thunk_context: &ThunkContext) -> (JoinHandle<ThunkContext>, Sender<()>);
}

impl Executor for Host {
    fn execute(&mut self, thunk_context: &ThunkContext) -> (JoinHandle<ThunkContext>, Sender<()>){
        let thunk_context = thunk_context.commit();

        let handle = thunk_context.handle().expect("should be a handle").clone();

        let entities = self.world().entities();
        let calls = thunk_context
            .state()
            .find_values("sequence")
            .iter()
            .filter_map(|v| {
                if let Value::Int(i) = v {
                    Some(entities.entity(*i as u32))
                } else {
                    None
                }
            }).collect::<Vec<_>>();

        let event_components = self.world().read_component::<Event>();
        let graph_components = self.world().read_component::<AttributeGraph>();
        let mut events = HashMap::<Entity, Event>::default();
        let mut graphs = HashMap::<Entity, AttributeGraph>::default();
        for call in calls.iter() {
            let event = event_components.get(*call).expect("should exist").duplicate();
            let graph = graph_components.get(*call).expect("should exist").clone();

            events.insert(*call, event);
            graphs.insert(*call, graph);
        }

        let (tx, rx) = oneshot::channel::<()>();

        let task = handle.spawn(async move {
            let mut thunk_context = thunk_context.clone();
            let handle = thunk_context.handle().expect("should be a handle");
            let rx = rx;
            for e in calls
            {
                // TODO -- link this with above
                let (tx, rx) = oneshot::channel::<()>();

                thunk_context = thunk_context.enable_async(e, handle.clone());

                let Event(event_name, Thunk(plugin_name, call, ..), ..) =
                    events.get(&e).expect("should exist");
                let graph = graphs.get(&e).expect("should exist");

                event!(Level::DEBUG, "Starting {event_name} {plugin_name}");

                let mut operation = Operation {
                    context: thunk_context.clone(),
                    task: call(&thunk_context.with_state(graph.clone())),
                };

                if let Some(context) = operation.task(rx).await {
                    thunk_context = context.commit();
                }

                // TODO -- probably need to have a way to configure this
                // if let Some(result) = operation.task().await {
                //     thunk_context = result.commit();
                // } else {
                //     event!(
                //         Level::ERROR,
                //         "Error, couldn't finish operation {event_name} {plugin_name}"
                //     );
                //     thunk_context.error(|g| {
                //         g.add_symbol("error", "Couldn't finish executing sequence");
                //     })
                // }
            }

            thunk_context
        });

        (task, tx)
    }
}
