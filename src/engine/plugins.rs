use std::collections::HashMap;

use reality::Block;
use specs::{prelude::*, Entities, Entity, ReadStorage, SystemData};
use tokio::{select, sync::oneshot};

use crate::prelude::*;

mod listener;
pub use listener::Listener as PluginListener;

mod broker;
pub use broker::Broker as PluginBroker;

mod features;
pub use features::PluginFeatures;

/// System data for plugins,
///
#[derive(SystemData)]
pub struct Plugins<'a>(
    PluginFeatures<'a>,
    Entities<'a>,
    ReadStorage<'a, Thunk>,
    ReadStorage<'a, Block>,
    ReadStorage<'a, AttributeGraph>,
);

impl<'a> Plugins<'a> {
    /// Returns an initialized context,
    ///
    pub fn initialize_context(
        &self,
        entity: Entity,
        initial_context: Option<&ThunkContext>,
    ) -> ThunkContext {
        let Plugins(plugin_features, .., blocks, graphs) = self;

        let context =
            plugin_features.enable(entity, initial_context.unwrap_or(&ThunkContext::default()));

        let block = blocks.get(entity).expect("should have a block");
        let graph = graphs.get(entity).expect("should have a graph");

        context.with_state(graph.clone()).with_block(block)
    }

    /// Combines a sequence of plugin calls into an operation,
    ///
    pub fn start_sequence(
        &self,
        sequence: &Sequence,
        initial_context: Option<&ThunkContext>,
    ) -> Operation {
        let Plugins(plugin_features, .., thunk_components, block_components, graph_components) =
            self;

        let sequence = sequence.clone();
        let handle = plugin_features.handle();
        let entity = sequence.peek().expect("should have at least 1 entity");

        let thunk_context = self.initialize_context(entity, initial_context);

        let mut thunks = HashMap::<Entity, Thunk>::default();
        let mut graphs = HashMap::<Entity, AttributeGraph>::default();
        let mut blocks = HashMap::<Entity, Block>::default();
        for call in sequence.iter_entities() {
            let thunk = thunk_components
                .get(call)
                .expect("should have a thunk")
                .clone();
            let graph = graph_components
                .get(call)
                .expect("should have a graph")
                .clone();
            let block = block_components
                .get(call)
                .expect("should have a block")
                .clone();
            thunks.insert(call, thunk);
            graphs.insert(call, graph);
            blocks.insert(call, block);
        }

        let (tx, rx) = tokio::sync::oneshot::channel::<()>();

        let task = handle.spawn(async move {
            let mut context = thunk_context.clone();
            let handle = context.handle().expect("should be a handle");
            let mut cancel_source = Some(rx);

            for e in sequence.iter_entities() {
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

        Operation::empty(handle.clone()).with_task((task, tx))
    }
}
