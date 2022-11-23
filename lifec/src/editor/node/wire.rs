
use reality::wire::{WireObject, Encoder, Frame, FrameIndex, ResourceId};
use specs::Component;

use crate::{prelude::EventStatus, engine::EngineStatus};

use super::NodeStatus;

impl WireObject for NodeStatus {
    fn encode<BlobImpl>(&self, _: &specs::World, encoder: &mut Encoder<BlobImpl>)
    where
        BlobImpl: std::io::Read + std::io::Write + std::io::Seek + Clone + Default,
    {
        encoder.interner.add_ident("engine_status");
        encoder.interner.add_ident("event_status");
        encoder.interner.add_ident("node_status");
        encoder.interner.add_ident("custom");
        encoder.interner.add_ident("profiler");
        encoder.interner.add_ident("inactive");
        encoder.interner.add_ident("active");
        encoder.interner.add_ident("disposed");
        encoder.interner.add_ident("scheduled");
        encoder.interner.add_ident("new");
        encoder.interner.add_ident("in_progress");
        encoder.interner.add_ident("paused");
        encoder.interner.add_ident("ready");
        encoder.interner.add_ident("completed");
        encoder.interner.add_ident("cancelled");

        match self {
            NodeStatus::Engine(engine) => match engine {
                crate::engine::EngineStatus::Inactive(e) => {
                    let frame = Frame::extension("engine_status", "inactive").with_parity(*e);
                    encoder.frames.push(frame);
                }
                crate::engine::EngineStatus::Active(e) => {
                    let frame = Frame::extension("engine_status", "active").with_parity(*e);
                    encoder.frames.push(frame);
                }
                crate::engine::EngineStatus::Disposed(e) => {
                    let frame = Frame::extension("engine_status", "disposed").with_parity(*e);
                    encoder.frames.push(frame);
                }
            },
            NodeStatus::Event(event) => match event {
                EventStatus::Scheduled(e) => {
                    let frame = Frame::extension("event_status", "scheduled").with_parity(*e);
                    encoder.frames.push(frame);
                }
                EventStatus::New(e) => {
                    let frame = Frame::extension("event_status", "new").with_parity(*e);
                    encoder.frames.push(frame);
                }
                EventStatus::InProgress(e) => {
                    let frame = Frame::extension("event_status", "in_progress").with_parity(*e);
                    encoder.frames.push(frame);
                }
                EventStatus::Paused(e) => {
                    let frame = Frame::extension("event_status", "paused").with_parity(*e);
                    encoder.frames.push(frame);
                }
                EventStatus::Ready(e) => {
                    let frame = Frame::extension("event_status", "ready").with_parity(*e);
                    encoder.frames.push(frame);
                }
                EventStatus::Completed(e) => {
                    let frame = Frame::extension("event_status", "completed").with_parity(*e);
                    encoder.frames.push(frame);
                }
                EventStatus::Cancelled(e) => {
                    let frame = Frame::extension("event_status", "cancelled").with_parity(*e);
                    encoder.frames.push(frame);
                }
                EventStatus::Inactive(e) => {
                    let frame = Frame::extension("event_status", "inactive").with_parity(*e);
                    encoder.frames.push(frame);
                }
                EventStatus::Disposed(e) => {
                    let frame = Frame::extension("event_status", "disposed").with_parity(*e);
                    encoder.frames.push(frame);
                }
            },
            NodeStatus::Profiler(e) => {
                let frame = Frame::extension("node_status", "profiler").with_parity(*e);
                encoder.frames.push(frame);
            }
            NodeStatus::Custom(e) => {
                let frame = Frame::extension("node_status", "custom").with_parity(*e);
                encoder.frames.push(frame);
            }
            NodeStatus::Empty => {}
        }
    }

    fn decode<BlobImpl>(
        protocol: &reality::wire::Protocol<BlobImpl>,
        interner: &reality::wire::Interner,
        _: &BlobImpl,
        frames: &[Frame],
    ) -> Self
    where
        BlobImpl: std::io::Read + std::io::Write + std::io::Seek + Clone + Default,
    {
        let frame = frames.get(0).expect("should hav a frame");
        let entity = frame.get_entity(protocol.as_ref(), protocol.assert_entity_generation());
        match (
            frame.name(interner).unwrap_or_default().as_str(),
            frame.symbol(interner).unwrap_or_default().as_str(),
        ) {
            ("engine_status", status) => NodeStatus::Engine(match status {
                "active" => EngineStatus::Active(entity),
                "inactive" => EngineStatus::Inactive(entity),
                "disposed" => EngineStatus::Disposed(entity),
                _ => panic!("unknown engine status symbol"),
            }),
            ("event_status", status) => NodeStatus::Event(match status {
                "scheduled" => EventStatus::Scheduled(entity),
                "new" => EventStatus::New(entity),
                "in_progress" => EventStatus::InProgress(entity),
                "paused" => EventStatus::Paused(entity),
                "ready" => EventStatus::Ready(entity),
                "completed" => EventStatus::Completed(entity),
                "cancelled" => EventStatus::Cancelled(entity),
                "inactive" => EventStatus::Inactive(entity),
                "disposed" => EventStatus::Disposed(entity),
                _ => panic!("unknown engine status symbol"),
            }),
            ("node_status", status) => match status {
                "profiler" => NodeStatus::Profiler(entity),
                "custom" => NodeStatus::Custom(entity),
                _ => NodeStatus::Empty,
            },
            _ => NodeStatus::Empty,
        }
    }

    fn build_index(_: &reality::wire::Interner, frames: &[Frame]) -> FrameIndex {
        let mut frame_index = FrameIndex::default();

        let mut pos = 0;
        for (idx, _) in frames.iter().enumerate() {
            let range = pos..pos + 1; // + 1 to include op 0x71
            frame_index.insert(format!("{idx}-node-status"), vec![range]);
            pos += 1;
        }

        frame_index
    }

    fn resource_id() -> ResourceId {
        ResourceId::new::<<NodeStatus as Component>::Storage>()
    }
}
