
/// A node is a packet of 64 bytes of data, that 
/// is used for storing addresses for a resource element of the protocol.
/// 
/// *context* In the specification for ip addresses, they refer to the listener of the
/// address as a node, This protocol reuses this concept.
/// 
pub struct Node {
    identity: [u8; 64],
    blob_len: usize,
}
