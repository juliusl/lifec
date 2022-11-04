use std::collections::HashMap;

use reality::Block;
use specs::{prelude::*, Entities, Entity, ReadStorage, SystemData};
use tokio::{select, sync::oneshot};

use crate::{prelude::*, engine::Completion};

mod listener;
pub use listener::PluginListener;

mod broker;
pub use broker::Broker as PluginBroker;

mod features;
pub use features::Features as PluginFeatures;

/// System data for plugins,
///
#[derive(SystemData)]
pub struct Plugins<'a> { 
    features: PluginFeatures<'a>,
    entities: Entities<'a>,
    thunks: ReadStorage<'a, Thunk>,
    blocks: ReadStorage<'a, Block>,
    graphs: WriteStorage<'a, AttributeGraph>,
}

impl<'a> Plugins<'a> {
    /// Returns a reference to plugin features,
    /// 
    pub fn features(&self) -> &PluginFeatures<'a> {
        let Plugins { features, .. } = self;

        features
    }

    /// Updates a graph,
    /// 
    pub fn update_graph(&mut self, graph: AttributeGraph) -> bool {
        let Plugins { entities, graphs, .. } = self;
        let entity = entities.entity(graph.entity_id());
        if let Some(_) = graphs.insert(entity, graph).expect("should be able to insert") {
            true
        } else {
            false
        }
    }

    /// Returns a new context,
    /// 
    pub fn new_context(&self) -> ThunkContext {
        let Plugins { entities, .. } = self;

        let entity = entities.create();

        self.initialize_context(entity, None)
    }

    /// Returns an initialized context,
    ///
    pub fn initialize_context(
        &self,
        entity: Entity,
        initial_context: Option<&ThunkContext>,
    ) -> ThunkContext {
        let Plugins { features, blocks, graphs, .. }  = self;

        let mut context =
            features.enable(entity, initial_context.unwrap_or(&ThunkContext::default()));

        if let Some(block) = blocks.get(entity) {
            context = context.with_block(block);
        }

        if let Some(graph) = graphs.get(entity) {
            context = context.with_state(graph.clone());
        }

        context
    }

    /// Combines a sequence of plugin calls into an operation,
    ///
    pub fn start_sequence(
        &self,
        event: Entity,
        sequence: &Sequence,
        initial_context: Option<&ThunkContext>,
    ) -> Operation {
        let Plugins { features, thunks, blocks, graphs, .. } =
            self;

        let sequence = sequence.clone();
        let handle = features.handle();
        let entity = sequence.peek().expect("should have at least 1 entity");

        let thunk_context = self.initialize_context(entity, initial_context);

        let mut thunk_map = HashMap::<Entity, Thunk>::default();
        let mut graph_map = HashMap::<Entity, AttributeGraph>::default();
        let mut block_map = HashMap::<Entity, Block>::default();
        for call in sequence.iter_entities() {
            let thunk = thunks
                .get(call)
                .expect("should have a thunk")
                .clone();
            let graph = graphs
                .get(call)
                .expect("should have a graph")
                .clone();
            let block = blocks
                .get(call)
                .expect("should have a block")
                .clone();
            thunk_map.insert(call, thunk);
            graph_map.insert(call, graph);
            block_map.insert(call, block);
        }

        let (tx, rx) = tokio::sync::oneshot::channel::<()>();

        let task = handle.spawn(async move {
            let mut context = thunk_context.clone();
            let handle = context.handle().expect("should be a handle");
            let mut cancel_source = Some(rx);

            for e in sequence.iter_entities() {
                if let Some(mut _rx) = cancel_source.take() {
                    let (_tx, rx) = oneshot::channel::<()>();

                    context.set_entity(e);

                    let thunk = thunk_map.get(&e).expect("should have a thunk");
                    let graph = graph_map.get(&e).expect("should exist");
                    let block = block_map.get(&e).expect("should exist");

                    context.set_state(graph.clone());
                    context.set_block(block);

                    let mut operation = Operation::empty(handle.clone())
                        .start_with(thunk, &mut context);
                    {
                        let _rx = &mut _rx;
                        select! {
                            result = operation.task(rx) => {
                                match result {
                                    Some(mut result) => { 
                                        let mut completion = Completion {
                                            event,
                                            thunk: e,
                                            control_values: context.control_values().clone(),
                                            query: context.properties().clone(),
                                            returns: None,
                                        };

                                        context = result.consume();

                                        completion.returns = Some(context.properties().clone());

                                        context.dispatch_completion(completion);
                                    }
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

                    event!(
                        Level::DEBUG,
                        "\n\n\t{:?}\n\tcompleted\n\tplugin {}\n",
                        thunk,
                        e.id(),
                    );

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
