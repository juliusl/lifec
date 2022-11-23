use specs::{Component, VecStorage};
use tokio::sync::oneshot::{Sender, Receiver, self};
use crate::prelude::ThunkContext;

/// When an event completes usually the original result is dropped after passing a reference to the next event,
/// 
/// If this component is added to an entity, then when it starts an activity it will use the thunk context found in this component to start,
/// When the activity completes it will remove this component and send the result to the channel,
/// 
#[derive(Debug, Component)]
#[storage(VecStorage)]
pub struct Yielding(
    /// Channel to send the result to, also closing can cancel the operation,
    pub Sender<ThunkContext>, 
    /// Initial context to use,
    pub ThunkContext
);

impl Yielding {
    /// Returns a new yielding component and receiver,
    /// /
    pub fn new(tc: ThunkContext) -> (Yielding, Receiver<ThunkContext>) {
        let (tx, rx) = oneshot::channel();
        (Yielding(tx, tc), rx)
    }
}