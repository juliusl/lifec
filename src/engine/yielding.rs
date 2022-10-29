use specs::{Component, VecStorage};
use tokio::sync::oneshot::{Sender, Receiver, self};
use crate::prelude::ThunkContext;

/// When an event completes the original is dropped after passing a reference to the next event,
/// 
/// If this component exists on the executing entity, then the runtime will send it to this oneshot channel,
/// 
#[derive(Debug, Component)]
#[storage(VecStorage)]
pub struct Yielding(pub Sender<ThunkContext>, pub ThunkContext);

impl Yielding {
    /// Returns a new yielding component and receiver,
    /// /
    pub fn new(tc: ThunkContext) -> (Yielding, Receiver<ThunkContext>) {
        let (tx, rx) = oneshot::channel();
        (Yielding(tx, tc), rx)
    }
}