use atlier::system::Value;
use reality::{
    wire::{Frame, FrameIndex, WireObject},
    Keywords,
};
use specs::{shred::ResourceId, Entity};

use crate::prelude::NodeCommand;

/// Type alias for a vector of node commands,
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
        encoder.interner.add_ident("journal");

        let frames_len = _encoder.frames.len() + 1;
        let frame = Frame::add(
            "journal",
            &Value::Int(frames_len as i32),
            &mut encoder.blob_device,
        );
        encoder.frames.push(frame);

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
        let journal = frames.get(0).expect("should have a starting frame");
        assert_eq!(journal.name(interner), Some("journal".to_string()));

        match journal
            .read_value(interner, blob_device)
            .expect("should have a value")
        {
            Value::Int(commands) => {
                assert_eq!(commands as usize, frames.len());
            }
            _ => {
                panic!("Root attribute should be an integer");
            }
        }

        let mut journal = Journal::default();

        let command_frames = &frames[1..];
        let index = NodeCommand::build_index(interner, command_frames);
        let mut index = index.iter().map(|(_, v)| v).flatten().cloned().collect::<Vec<_>>();
        index.sort_by(|a, b| a.start.cmp(&b.start));
        for range in index {
            let frames = &command_frames[range];
            let start =
                frames[0].get_entity(protocol.as_ref(), false);
            let command = NodeCommand::decode(protocol, interner, blob_device, frames);
            journal.push((start, command));
        }

        journal
    }

    fn build_index(
        interner: &reality::wire::Interner,
        frames: &[reality::wire::Frame],
    ) -> reality::wire::FrameIndex {
        let mut frame_index = FrameIndex::default();

        for (idx, f) in frames.iter().enumerate().filter(|(_, f)| {
            f.name(interner) == Some("journal".to_string())
                && f.keyword() == Keywords::Add
                && f.attribute() == Some(reality::Attributes::Int)
        }) {
            match f.read_value(interner, &Default::default()) {
                Some(Value::Int(len)) => {
                    let range = idx..idx + (len as usize);
                    frame_index.insert(format!("journal-{}", idx), vec![range]);
                }
                _ => {}
            }
        }

        frame_index
    }

    fn resource_id() -> specs::shred::ResourceId {
        ResourceId::new::<Journal>()
    }
}
