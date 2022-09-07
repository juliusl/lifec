use specs::Dispatcher;

use crate::{plugins::{Sequence, Engine}, host::Transport, Runtime};


/// Internal type for handling guest runtime services for
/// the protocol Host implementation 
/// 
/// This will be applied as entity 0 in the guest world 
/// 
pub struct ProtocolEngine {
    /// Dispatcher for the guest runtime this engine is operating
    /// 
    dispatcher: Option<Dispatcher<'static, 'static>>,

    /// Sequence this engine will execute in the guest runtime 
    /// 
    sequence: Sequence,

    /// Runtime from the host 
    /// 
    runtime: Runtime,
}

impl ProtocolEngine {
    /// Creates a new protocol engine 
    /// 
    pub fn new(runtime: Runtime, dispatcher: Dispatcher<'static, 'static>) -> Self {
        let dispatcher = Some(dispatcher);
        
        Self { dispatcher, runtime, sequence: Sequence::default() }
    }

    /// Takes the engine's guest runtime dispatcher, 
    /// This activates the engine.
    /// 
    pub fn activate(&mut self) -> Option<Dispatcher<'static, 'static>> {
        self.dispatcher.take()
    }
}
