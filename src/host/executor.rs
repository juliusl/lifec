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
use tracing::{event, Level};

/// Trait for executing a sequence of events,
///
pub trait Executor {
    /// Executes a sequence of events,
    ///
    /// Looks for a sequence property in thunk context which is a list of properties,
    ///
    fn execute(&mut self, thunk_context: &ThunkContext) -> ThunkContext;
}

impl Executor for Host {
    fn execute(&mut self, thunk_context: &ThunkContext) -> ThunkContext {
        let mut thunk_context = thunk_context.clone();

        thunk_context.commit();

        let handle = {
            let runtime = self.world().read_resource::<tokio::runtime::Runtime>();
            let handle = runtime.handle().clone();
            handle
        };

        for e in thunk_context
            .state()
            .find_values("sequence")
            .iter()
            .filter_map(|v| {
                if let Value::Int(i) = v {
                    Some(self.world().entities().entity(*i as u32))
                } else {
                    None
                }
            })
        {        
            thunk_context = thunk_context.enable_async(e, handle.clone());

            let event = self.world().read_component::<Event>();
            let graphs = self.world().read_component::<AttributeGraph>();
            let Event(event_name, Thunk(plugin_name, call, ..), ..) =
                event.get(e).expect("should exist");
            let graph = graphs.get(e).expect("should exist");

            event!(Level::DEBUG, "Starting {event_name} {plugin_name}");

            let mut operation = Operation {
                context: thunk_context.with_state(graph.clone()),
                task: call(&thunk_context),
            };

            // TODO -- probably need to have a way to configure this
            if let Some(result) = operation.wait_with_timeout(Duration::from_secs(300)) {
                thunk_context = result.commit();
            } else {
                event!(
                    Level::ERROR,
                    "Error, couldn't finish operation {event_name} {plugin_name}"
                );
                thunk_context.error(|g| {
                    g.add_symbol("error", "Couldn't finish executing sequence");
                })
            }
        }

        thunk_context
    }
}
