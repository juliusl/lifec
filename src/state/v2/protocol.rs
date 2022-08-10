
mod control_cube;
pub use control_cube::ControlCube;

mod data_cube;
pub use data_cube::DataCube;

mod node;
use node::Node;

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
pub trait Cube {
    fn nodes(&self) -> [Node; 8];
}

/// TODO: 
/// thunk_context - 
pub trait BlobProvider {
    /// A blob provider populates the blob_len field in the node, and returns
    /// 
    fn fetch(&self, node: Node) -> Node;
}
