use std::time::Duration;

use crate::prelude::*;

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
#[derive(Component)]
#[storage(VecStorage)]
pub struct Operation {
    /// Handle to tokio runtime for driving the task in a standalone manner,
    handle: tokio::runtime::Handle,
    /// Running task that returns a thunk context.
    task: Option<AsyncContext>,
    /// Result returned from the task,
    result: Option<ThunkContext>,
    /// True if cancelled,
    cancelled: bool,
}

impl Operation {
    /// Returns an empty operation,
    ///
    pub fn empty(handle: Handle) -> Self {
        Self {
            handle,
            task: None,
            result: None,
            cancelled: false,
        }
    }

    /// Starts a task from a plugin call and returns it w/ self,
    ///
    pub fn start<P>(&self, context: &mut ThunkContext) -> Self
    where
        P: Plugin,
    {
        self.start_with(&Thunk::from_plugin::<P>(), context)
    }

    /// Starts a task with a thunk and returns it w/ self,
    ///
    pub fn start_with(&self, Thunk(_, _, func, _): &Thunk, context: &mut ThunkContext) -> Self {
        if self.result.is_some() {
            event!(
                Level::WARN,
                "A result already exists for operation, this will return an operation that will override the result on completion"
            );
        }

        let mut clone = self.clone();
        clone.task = func(context);
        clone
    }

    /// Replaces any ongoing task by cancelling the previous task, and setting a new one
    ///
    pub fn replace(&mut self, Thunk(symbol, _, func, _): &Thunk, context: &mut ThunkContext) {
        event!(
            Level::DEBUG,
            "Replacing operation for {symbol} for entity {}",
            context.state().entity_id()
        );
        self.cancel();

        self.task = func(context);
        self.cancelled = false;
    }

    /// Returns self with a new async context,
    ///
    pub fn with_task(mut self, async_context: AsyncContext) -> Self {
        self.cancel();

        self.task = Some(async_context);
        self.cancelled = false;
        self
    }

    /// Sets the async context in place,
    ///
    pub fn set_task(&mut self, async_context: impl Into<AsyncContext>) {
        // Cancel any previous tasks that may have been running
        self.cancel();

        self.task = Some(async_context.into());
        self.cancelled = false;
    }

    /// **Destructive** - calling this method will take and handle resolving this task to completion,
    ///
    /// Returns some context if the task returned a context, also sets that context as the current context of the operation,
    /// otherwise returns None
    ///
    /// **Note** If None is returned, that implies that self.context is the latest state
    ///
    pub async fn task(
        &mut self,
        cancel_source: tokio::sync::oneshot::Receiver<()>,
    ) -> Option<ThunkContext> {
        if let Some((task, cancel)) = self.task.take() {
            select! {
                r = task => {
                    match r {
                        Ok(tc) => {
                            self.result = Some(tc.clone());
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
            self.result.clone()
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
            return self.handle.block_on(async {
                match task.await {
                    Ok(tc) => {
                        self.result = Some(tc.clone());
                        Some(tc)
                    }
                    Err(err) => {
                        event!(Level::ERROR, "operation's task returned an error, {err}");
                        None
                    }
                }
            });
        }

        self.result.clone()
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
        if let Some((task, cancel)) = self.task.take() {
            return self.handle.block_on(async {
                match tokio::time::timeout(timeout, task).await {
                    Ok(result) => match result {
                        Ok(tc) => {
                            self.result = Some(tc.clone());
                            Some(tc)
                        }
                        Err(err) => {
                            event!(Level::ERROR, "operation's task returned an error, {err}");
                            None
                        }
                    },
                    Err(elapsed) => {
                        event!(Level::ERROR, "operation timed out, elapsed {elapsed}");
                        cancel.send(()).ok();
                        None
                    }
                }
            });
        }

        self.result.clone()
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

        self.result.clone()
    }

    /// Returns the result from this operation,
    ///
    pub fn result(&self) -> Option<&ThunkContext> {
        self.result.as_ref()
    }

    /// Returns true if the operation has completed,
    ///
    pub fn is_completed(&self) -> bool {
        self.task.is_none() && self.result.is_some()
    }

    /// Returns true if the underlying is ready
    ///
    pub fn is_ready(&self) -> bool {
        if let Some(task) = self.task.as_ref() {
            task.0.is_finished()
        } else {
            false
        }
    }

    /// Returns true if this is an empty operation,
    ///
    pub fn is_empty(&self) -> bool {
        self.task.is_none() && self.result.is_none() && !self.cancelled
    }

    /// Returns true if this event has been cancelled,
    ///
    /// TODO: Try to handle this differently
    ///
    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    /// Cancels any ongoing task,
    ///
    /// returns true if a change was made
    ///
    pub fn cancel(&mut self) -> bool {
        if let Some((j, cancel)) = self.task.take() {
            cancel.send(()).ok();
            j.abort();
            self.cancelled = true;
            true
        } else {
            false
        }
    }
}

impl Clone for Operation {
    fn clone(&self) -> Self {
        Self {
            handle: self.handle.clone(),
            result: self.result.clone(),
            task: None,
            cancelled: self.cancelled,
        }
    }
}

impl Into<AsyncContext> for Operation {
    fn into(mut self) -> AsyncContext {
        self.task
            .take()
            .expect("should be an operation w/ a task to consume")
    }
}
