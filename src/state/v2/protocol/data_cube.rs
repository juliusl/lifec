use crate::plugins::BlockAddress;

use super::node::Node;

/// A data cube is the initial cube processed in the tesseract protocol,
/// 
/// In the data cube processing phase, the goal is to establish a stable/consistent
/// environment, (soft-start), before the runtime starts.
/// 
pub struct DataCube {
    /// The block address of the transmission side of the runtime
    block_address: BlockAddress,
    /// Additional nodes
    nodes: [Node; 7],
}

