use reality::wire::WireObject;
use specs::shred::ResourceId;

use super::Debugger;


impl WireObject for Debugger {
    fn encode<BlobImpl>(&self, world: &specs::World, encoder: &mut reality::wire::Encoder<BlobImpl>)
    where
        BlobImpl: std::io::Read + std::io::Write + std::io::Seek + Clone + Default,
    {
        todo!()
    }

    fn decode(
        protocol: &reality::wire::Protocol,
        interner: &reality::wire::Interner,
        blob_device: &std::io::Cursor<Vec<u8>>,
        frames: &[reality::wire::Frame],
    ) -> Self {
        todo!()
    }

    fn build_index(
        interner: &reality::wire::Interner,
        frames: &[reality::wire::Frame],
    ) -> reality::wire::FrameIndex {
        todo!()
    }

    fn resource_id() -> specs::shred::ResourceId {
        ResourceId::new::<Option<Debugger>>()
    }
}
