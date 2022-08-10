mod control_cube;
pub use control_cube::ControlCube;

mod data_cube;
pub use data_cube::DataCube;

/// # Tesseract protocol - runtime start-up protocol
/// 
/// This protocol is to procedurely begin a runtime. A runtime can host a number of different,
/// applications, but in general a runtime is a collection of engines, and an engine is a sequence of events.
/// The goal of the protocol is to predictably start a runtime in a multitude of different environments, and produce 
/// consistent results. The protocol can be built on top of existing technologies, in order to support multi-environments.
/// 
/// ## Elements of the protocol
/// 
/// A "cube" is a collection of 8 x 64 byte "nodes", that can each be backed
/// by a persistent 4 GB blob. At max a single cube should be able to store
/// 32 GB's total. (It's called tesseract because the protocol can be viewed as a collection of cubes)
/// 
/// To put this in terms of a container, roughly,
/// a single "node" <=> a single blob descriptor, (manifest, config, index, blob, etc)
/// a single "cube" <=> a single blob, (layer tar.gz, .json, etc)
/// 
/// ## Future-proofing strategy
/// 
/// Currently, to start the protocol, a datacube and controlcube are required.
/// 
/// To expand the protocol, additional cube-types may be defined and inserted into the
/// sequence of cube processing. For example, to create a breaking change for the protocol, a version cube could
/// be added before the datacube, and this would force the protocol to diverge from the initial datacube -> controlcube.
/// Vice-versa, to make the protocol forward compatible when adding new features, new cube-types can be inserted after the initial,
/// data cube -> control cube sequence.
/// 
pub struct Protocol;

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

/// Cube is a layout of 8 nodes and their data 
/// 
/// When serialized, the node_ids will have a max size of 512 bytes,
/// and their blob_lens will have a max size of 64 bytes, 
/// 
/// Therefore when a cube is transmitted, it will be a sequence of a 
/// 512 byte frame, followed by a 64 byte frame, 
/// 
/// # Background
/// 
/// Current design limits for this protocol include a max blob length of 4 GB's per node, meaning a single cube
/// can represent at most 32 GB's worth of data.
/// 
pub struct Cube {
    /// Node identity data, max 512 bytes
    node_ids: [[u8; 64]; 8],
    /// Blob length data from nodes, max 64 bytes
    blob_lens: [usize; 8],
}
