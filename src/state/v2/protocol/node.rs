
/// A node is basically an extent, w/ 64 byte limit used for the identity, and the current
/// length of the blob this node represents.
/// 
/// # Background 
/// 
/// In the specification for ip addresses, they refer to the listener of the
/// address as a node, this is also referred to as a "machine" or "host" in network terms.
/// 
/// These days the actual host for an application has been abstracted to the point
/// that resources the application requires can be distributed across multiple network and hardware boundaries.
/// 
/// Because of this, it's useful to have a type that can be used in many different contexts as a general purpose
/// target for transfering data between these boundaries.
/// 
/// For example, a block address can contain a hash_code u64 value, a link between two entities,
/// and a link for either ipv4 or ipv6 socket addresses (including the port number). This can all fit within
/// the 64 byte identity limit. 
/// 
/// Another example, is a block blob id in azure can be a maximum of 64 bytes long before being base64 encoded. When a 
/// block blob list is returned each member will contain this id and the number of bytes put into the block. 
/// 
/// Both of these examples can be converted into a `Node` struct and subsequently can be used within a protocol to represent
/// point A and point B of a blob transfer w/o leaking any underlying implementation details. 
/// 
/// So long as a system exists that can convert the Node into streams, then two systems that provide nodes can transfer data
/// w/o needing to understand implementation details of the other side.
/// 
pub struct Node {
    /// 64 bytes representing the identity of this node
    identity: [u8; 64],
    /// The current length of the blob this node points to
    blob_len: usize,
}