use crate::{prelude::NodeCommand, state::AttributeIndex};
use reality::{
    wire::{FrameIndex, WireObject},
};
use specs::{shred::ResourceId, Entity, WorldExt};

/// Struct for storing executed node commands,
///
#[derive(Debug, Default, Clone, Hash, PartialEq, Eq)]
pub struct Journal(pub Vec<(Entity, NodeCommand)>);

impl Journal {
    /// Pushes an entry,
    ///
    pub fn push(&mut self, entry: (Entity, NodeCommand)) {
        self.0.push(entry);
    }

    /// Returns an iterator over entries,
    ///
    pub fn iter(&self) -> impl Iterator<Item = &(Entity, NodeCommand)> {
        self.0.iter()
    }

    /// Clears the journal,
    ///
    pub fn clear(&mut self) {
        self.0.clear()
    }
}

impl WireObject for Journal {
    fn encode<BlobImpl>(&self, world: &specs::World, encoder: &mut reality::wire::Encoder<BlobImpl>)
    where
        BlobImpl: std::io::Read + std::io::Write + std::io::Seek + Clone + Default,
    {
        let mut _encoder = reality::wire::Encoder::new();
        for (_, c) in self.0.iter() {
            c.encode(world, &mut _encoder);
        }
        encoder.interner = _encoder.interner.clone();

        for f in _encoder.frames.iter() {
            encoder.frames.push(f.clone());
        }
    }

    fn decode(
        protocol: &reality::wire::Protocol,
        interner: &reality::wire::Interner,
        blob_device: &std::io::Cursor<Vec<u8>>,
        frames: &[reality::wire::Frame],
    ) -> Self {
        let mut journal = Journal::default();

        let command_frames = &frames[..];
        let index = NodeCommand::build_index(interner, command_frames);
        let mut index = index
            .iter()
            .map(|(_, v)| v)
            .flatten()
            .cloned()
            .collect::<Vec<_>>();
        index.sort_by(|a, b| a.start.cmp(&b.start));
        for range in index {
            let frames = &command_frames[range];
            let command = NodeCommand::decode(protocol, interner, blob_device, frames);
            let entity = {
                match &command {
                    NodeCommand::Activate(e)
                    | NodeCommand::Reset(e)
                    | NodeCommand::Pause(e)
                    | NodeCommand::Resume(e)
                    | NodeCommand::Cancel(e)
                    | NodeCommand::Spawn(e) => *e,
                    NodeCommand::Update(g) => { 
                        let e = g.clone().entity_id();
                        protocol.as_ref().entities().entity(e)
                    },
                    NodeCommand::Swap { owner, .. } => *owner,
                    NodeCommand::Custom(_, e) => *e,
                }
            };

            journal.push((entity, command));
        }

        journal
    }

    fn build_index(
        _: &reality::wire::Interner,
        frames: &[reality::wire::Frame],
    ) -> reality::wire::FrameIndex {
        let mut frame_index = FrameIndex::default();

        frame_index.insert("journal".to_string(), vec![0..frames.len()]);

        frame_index
    }

    fn resource_id() -> specs::shred::ResourceId {
        ResourceId::new::<Journal>()
    }
}

mod tests {
    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test() {
        use crate::prelude::{Appendix, Journal, NodeCommand};
        use reality::wire::{Protocol, WireObject};
        use specs::WorldExt;
        use std::sync::Arc;

        let mut protocol = Protocol::empty();

        protocol.as_mut().insert(Arc::new(Appendix::default()));

        // Record node command as wire objects in a protocol,
        //
        let frame_count = protocol.encoder::<Journal>(|world, encoder| {
            let mut journal = Journal::default();
            let entity_4 = world.entities().entity(4);
            let entity_2 = world.entities().entity(2);
            journal.push((entity_4, NodeCommand::Activate(entity_4)));
            journal.push((entity_2, NodeCommand::Spawn(entity_2)));
            journal.push((entity_4, NodeCommand::Activate(entity_4)));
            journal.push((entity_2, NodeCommand::Activate(entity_2)));
            journal.encode(world, encoder);
            encoder.frame_index = Journal::build_index(&encoder.interner, encoder.frames_slice());
        });

        let (control_client, control_server) = tokio::io::duplex(64 * frame_count);
        let (frame_client, frame_server) = tokio::io::duplex(64 * frame_count);
        let (blob_client, blob_server) = tokio::io::duplex(64 * frame_count);

        let read = tokio::spawn(async move {
            protocol
                .send_async::<Journal, _, _>(
                    || std::future::ready(control_client),
                    || std::future::ready(frame_client),
                    || std::future::ready(blob_client),
                )
                .await;
        });

        let write = tokio::spawn(async {
            let mut receiver = Protocol::empty();
            receiver
                .receive_async::<Journal, _, _>(
                    || std::future::ready(control_server),
                    || std::future::ready(frame_server),
                    || std::future::ready(blob_server),
                )
                .await;

            let journal = receiver.decode::<Journal>();
            journal
        });

        let (_, journal) = tokio::join!(read, write);

        let journal = journal.expect("should be okay");
        eprintln!("{:#?}", journal);
    }
}
