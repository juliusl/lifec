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