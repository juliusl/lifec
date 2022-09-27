use std::time::Duration;

use crate::plugins::{ThunkContext, AsyncContext};
use specs::{Component, DefaultVecStorage, Entity};
use tokio::{sync::oneshot::Receiver, select};
use tracing::{event, Level};

/// An operation encapsulates an async task and it's context
/// Where the result of the task is the next version of the context.
/// 
/// If a task exists, a join handle, and a oneshot that can be used to signal 
/// cancellation will be provided.
/// 
/// The fields of the operation are also the elements of executing an Event w/ 
/// an Engine/Plugin.
/// 
/// This component also implements, Item, Into<ThunkContext>, Clone so it can be used w/ 
/// Query<I>::thunk() as the item implementation. This allows operation to be a good starting point
/// for Systems using CatalogReader/CatalogWriter. Also, since thunk context can also be used as a src, 
/// operations can also transform into an attribute index.
/// 
/// Although, Operation implements Clone, it will not try to clone the underlying task if one exists.
/// This is useful for introspection on the initial_context used w/ an existing task.
/// 
/// # Background
/// 
/// In general, an event is generated outside of the host system/runtime. The event runtime is focused on 
/// serializing these events so that even though the events originate outside of the system from many places, state changes to the system as a whole
/// are processed from a single place. This is mostly good enough for simple cases.
/// 
/// The design of the operation type is focused on creating a self-contained version of what the event runtime does. This allows
/// plugins to implement more complicated sequences of tasks without taxing the event runtime. 
/// 
/// If this type did not exist, plugins would need to spawn additional events, and in order to get the results of those events,
/// the event runtime would need to maintain the relationship hierachy between all events. This would be out of scope of the primary
/// function of the event runtime, and lead to an unpredictable amount of entities being generated at runtime. 
/// 
/// Since an operation is self-contained, lifecycle management is shifted over to the plugin. To put it differently,
/// operations are useful for *internal* transitions, where as events are useful for *external* transitions.
/// 
/// This lets the event runtime to treat the operation as just another component of the entity.
/// 
#[derive(Component, Default)]
#[storage(DefaultVecStorage)]
pub struct Operation {
    pub context: ThunkContext,
    pub task: Option<AsyncContext>,
}

impl Operation {
    /// Returns a new empty operation w/o an existing task which can be used 
    /// as an Item implementation
    /// 
    pub fn new(entity: Entity, handle: tokio::runtime::Handle) -> Self {
        let tc = ThunkContext::default();
        let context = tc.enable_async(entity, handle);
        Self { context, task: None }
    }

    /// **Destructive** - calling this method will take and handle resolving this task to completion,
    /// 
    /// Returns some context if the task returned a context, also sets that context as the current context of the operation,
    /// otherwise returns None
    /// 
    /// **Note** If None is returned, that implies that self.context is the latest state
    /// 
    pub async fn task(&mut self, cancel_source: Receiver<()>) -> Option<ThunkContext>  {
        if let Some((task, cancel)) = self.task.take() {
            select! {
                r = task => {
                    match r {
                        Ok(tc) => {
                            self.context = tc.clone();
                            Some(tc)
                        },
                        Err(err) => {
                            event!(Level::ERROR, "error executing task {err}");
                            None
                        },
                    }
                }
                _ = cancel_source => {
                    event!(Level::INFO, "cancelling operation");
                    cancel.send(()).ok();
                    None
                }
            }
        } else {
            None
        }
    }

    /// **Destructive** - calling this method will take and handle resolving this task to completion,
    /// 
    /// Blocks the current thread indefinitely, until the task completes
    /// 
    /// If successfuly returns the resulting thunk context, and updates it's current context.
    /// 
    /// **See .task() for other mutation details**
    /// 
    pub fn wait(&mut self) -> Option<ThunkContext> {
        if let Some((task, _)) = self.task.take() {
            if let Some(handle) = self.context.handle() {
                return handle.block_on(async {
                    match task.await {
                        Ok(tc) => {
                            self.context = tc.clone();
                            Some(tc)
                        },
                        Err(err) => {
                            event!(Level::ERROR, "operation's task returned an error, {err}");
                            None
                        },
                    }
                })
            }
        }

        None
    }

    /// **Destructive** - calling this method will take and handle resolving this task to completion,
    /// 
    /// Blocks the current thread to wait for the underlying task to complete
    /// 
    /// The task must complete before the timeout expires
    /// 
    /// If successfuly returns the resulting thunk context, and updates it's current context.
    /// 
    /// **See .task() for other mutation details**
    /// 
    pub fn wait_with_timeout(&mut self, timeout: Duration) -> Option<ThunkContext> {
        if let Some((task, _)) = self.task.take() {
            if let Some(handle) = self.context.handle() {
                return handle.block_on(async {
                    match tokio::time::timeout(timeout, task).await {
                        Ok(result) => match result {
                            Ok(tc) => {
                                self.context = tc.clone();
                                Some(tc)
                            },
                            Err(err) => {
                                event!(Level::ERROR, "operation's task returned an error, {err}");
                                None
                            },
                        },
                        Err(elapsed) => {
                            event!(Level::ERROR, "operation timed out, elapsed {elapsed}");
                            None
                        },
                    }
                })
            }
        }

        None
    }

    /// Waits for the underlying task to complete if the task is ready,
    /// **otherwise** No-OP
    /// 
    pub fn wait_if_ready(&mut self) -> Option<ThunkContext> {
        if let Some(task) = self.task.as_ref() {
            if task.0.is_finished() {
                return self.wait();
            }
        }

        None 
    }
}

impl Clone for Operation {
    fn clone(&self) -> Self {
        Self { context: self.context.clone(), task: None }
    }
}