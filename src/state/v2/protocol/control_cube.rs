use crate::plugins::BlockAddress;

use super::Node;

/// The control cube is established in the protocol, 
/// after the datacube has completed its setup. 
/// 
/// The goal of the control cube is to finish bootstrapping, and notify
/// the listening runtime of any features that will require additional I/O.
/// 
/// If the control-flow is Empty, then the control cube will be used for, 
/// diagnostics and logging only. 
/// 
/// The listening runtime will monitor the control-cube, for the signal
/// to cancel, otherwise it will run to completion on its own.
/// 
pub struct ControlCube {
    /// Block address of the receiver
    block_address: BlockAddress,
    /// Control flow directive for the receiving runtime
    control_flow: ControlFlow,
    /// Extra node storage
    nodes: [Node; 6],
}

pub enum ControlFlow {
    Acknowledge(Features),
    Cancel,
}

pub enum Features {
    EnableDispatcher,
    EnableKVStore,
    Empty,
}