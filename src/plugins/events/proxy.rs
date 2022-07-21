use atlier::system::Value;
use specs::{Component, System, ReadStorage, Entities, WriteStorage, Join};
use specs::storage::DefaultVecStorage;
use tracing::{event, Level};
use crate::plugins::{ThunkContext, Project, BlockContext};

#[derive(Component, Default)]
#[storage(DefaultVecStorage)]
pub struct Proxy;

pub struct ProxyDispatcher(ThunkContext); 

impl From<ThunkContext> for ProxyDispatcher {
    fn from(tc: ThunkContext) -> Self {
        Self(tc)
    }
}

impl<'a> System<'a> for ProxyDispatcher {
    type SystemData = (
        Entities<'a>, 
        ReadStorage<'a, ThunkContext>,
        WriteStorage<'a, Proxy> 
    );

    fn run(&mut self, (entities, contexts, mut proxies): Self::SystemData) {
        for (entity, context) in (&entities, &contexts).join() {
            if context.as_ref().is_enabled("proxy").unwrap_or_default() && !proxies.contains(entity) {
                match proxies.insert(entity, Proxy::default()) {
                    Ok(_) => {
                        if let Some(dispatcher) = self.0.dispatcher() {
                            let mut graph = context.as_ref().clone(); 
                            let mut message = Project::default();

                            if let (Some(block_name), Some(block_symbol)) = (graph.find_text("block_name"), graph.find_text("block_symbol")) {
                                message = message.with_block(block_name, block_symbol, |c| {
                                    for attr in BlockContext::iter_block_attrs_mut(&mut graph) {
                                        // TODO - this is copied from lib.rs, need to consolidate
                                        if !attr.is_stable() {
                                            if let Some((_, value)) = attr.transient() {
                                                if let Value::Symbol(symbol) = attr.value() {
                                                    let symbol = symbol.trim_end_matches("::");
                                                    let name = attr.name().trim_end_matches(&format!("::{symbol}"));
                        
                                                    c.as_mut()
                                                        .define(name, symbol)
                                                        .edit_as(value.clone());
                                                }
                                            }
                                        } else {
                                            let (name, value) = (attr.name(), attr.value());
                                            c.with(name, value.clone());
                                        }
                                    }
                                });
                            }

                            match dispatcher.try_send(message.as_ref().clone()) {
                                Ok(_) => {
                                    event!(Level::DEBUG, "proxied {:?}", entity);
                                },
                                Err(err) => {
                                    event!(Level::ERROR, "error proxying {err}");
                                },
                            }
                        }
                    },
                    Err(err) => {
                        event!(Level::ERROR, "error inserting proxy component for {}, error: {err}", entity.id());
                    },
                }
            }
        }
    }
}
