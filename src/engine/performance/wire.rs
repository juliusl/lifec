use std::io::Cursor;
use std::io::Write;

use reality::wire::FrameIndex;
use reality::wire::{Frame, WireObject};
use specs::Component;
use specs::WorldExt;
use specs::shred::ResourceId;

use super::Performance;

impl WireObject for Performance {
    fn encode<BlobImpl>(&self, _: &specs::World, encoder: &mut reality::wire::Encoder<BlobImpl>)
    where
        BlobImpl: std::io::Read + std::io::Write + std::io::Seek + Clone + Default,
    {
        let Performance {
            // 8 bytes
            bucket_ms,
            // 8 bytes
            total_samples,
            // 4 bytes
            from,
            // 4 bytes
            to,
            // 4 bytes per bucket
            buckets,
            // 16 bytes
            percentiles,
        } = self;

        // Start the frame by including static properties and bucket/percentile counts
        let mut performance_start = Cursor::<[u8; 63]>::new([0; 63]);
        performance_start
            .write(&bytemuck::cast::<u64, [u8; 8]>(*bucket_ms))
            .ok();
        performance_start
            .write(&bytemuck::cast::<u64, [u8; 8]>(*total_samples))
            .ok();
        performance_start
            .write(&bytemuck::cast::<u32, [u8; 4]>(from.id()))
            .ok();
        performance_start
            .write(&bytemuck::cast::<u32, [u8; 4]>(to.id()))
            .ok();
        performance_start
            .write(&bytemuck::cast::<usize, [u8; 8]>(buckets.len()))
            .ok();
        performance_start
            .write(&bytemuck::cast::<usize, [u8; 8]>(percentiles.len()))
            .ok();
        let mut performance = vec![Frame::instruction(0x10, &performance_start.into_inner())];

        let mut buckets = buckets.chunks(15);
        while let Some(chunk) = buckets.next() {
            let mut bucket_frame = Cursor::<[u8; 63]>::new([0; 63]);

            for c in chunk {
                bucket_frame.write(&bytemuck::cast::<f32, [u8; 4]>(*c)).ok();
            }

            performance.push(Frame::instruction(0x11, &bucket_frame.into_inner()));
        }

        let mut percentiles = percentiles.chunks(3);
        while let Some(chunk) = percentiles.next() {
            let mut percentile_frame = Cursor::<[u8; 63]>::new([0; 63]);

            for (p, pv) in chunk {
                percentile_frame
                    .write(&bytemuck::cast::<u64, [u8; 8]>(*p))
                    .ok();
                percentile_frame
                    .write(&bytemuck::cast::<u64, [u8; 8]>(*pv))
                    .ok();
            }

            performance.push(Frame::instruction(0x12, &percentile_frame.into_inner()));
        }

        for p in performance {
            encoder.frames.push(p);
        }
    }

    fn decode(
        protocol: &reality::wire::Protocol,
        _: &reality::wire::Interner,
        _: &std::io::Cursor<Vec<u8>>,
        frames: &[reality::wire::Frame],
    ) -> Self {
        let start = frames.get(0).expect("should have starting frame");

        assert_eq!(start.op(), 0x10);

        let data = start.data();
        let mut bucket_ms = [0; 8];
        bucket_ms.copy_from_slice(&data[0..8]);
        let bucket_ms = bytemuck::cast::<[u8; 8], u64>(bucket_ms);

        let mut total_samples = [0; 8];
        total_samples.copy_from_slice(&data[8..16]);
        let total_samples = bytemuck::cast::<[u8; 8], u64>(total_samples);

        let mut from = [0; 4];
        from.copy_from_slice(&data[16..20]);
        let from = bytemuck::cast::<[u8; 4], u32>(from);
        let from = protocol.as_ref().entities().entity(from);

        let mut to = [0; 4];
        to.copy_from_slice(&data[20..24]);
        let to = bytemuck::cast::<[u8; 4], u32>(to);
        let to = protocol.as_ref().entities().entity(to);

        let mut performance = Performance {
            bucket_ms,
            buckets: vec![],
            percentiles: vec![],
            total_samples,
            from,
            to,
        };
        {
            // todo parity
            // 24..32
            let bucket_len = [0; 8].copy_from_slice(&data[24..32]);
            // 32..40
            let percentile_len = [0; 8].copy_from_slice(&data[32..40]);

            for frame in frames {
                match frame.op() {
                    0x11 => {
                        // TODO add parity
                        let buckets = &frame.data()[..60];
                        for c in buckets.chunks_exact(4) {
                            let mut _c = [0; 4];
                            _c.copy_from_slice(c);
                            performance.buckets.push(bytemuck::cast::<[u8; 4], f32>(_c));
                        }
                    }
                    0x12 => {
                        let buckets = &frame.data()[..48];
                        for c in buckets.chunks_exact(16) {
                            let mut p = [0; 8];
                            p.copy_from_slice(&c[..8]);
                            let mut pv = [0; 8];
                            pv.copy_from_slice(&c[8..]);
                            
                            // TODO add parity
                            performance.percentiles.push((
                                bytemuck::cast::<[u8; 8], u64>(p),
                                bytemuck::cast::<[u8; 8], u64>(pv),
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }

        performance
    }

    fn build_index(
        _: &reality::wire::Interner,
        frames: &[reality::wire::Frame],
    ) -> reality::wire::FrameIndex 
    {
        let mut frame_index = FrameIndex::default();

        for (idx, frame) in frames.iter().enumerate() {
            if frame.op() == 0x10 {
                if let Some(end) = frames[idx + 1..].iter().position(|f| f.op() == 0x10) {
                    let range = idx..idx + end + 1;
                    
                    assert!(range.start < range.end, "{:?}, {:?}", range, frames);

                    frame_index.insert(format!("{idx}-performance"), vec![range]);
                } else {
                    let range = idx..frames.len();

                    assert!(range.start < range.end, "{:?}, {:?}", range, frames);

                    frame_index.insert(format!("{idx}-performance"), vec![range]);
                }
            }
        }

        frame_index
    }

    fn resource_id() -> specs::shred::ResourceId {
        ResourceId::new::<<Performance as Component>::Storage>()
    }
}
