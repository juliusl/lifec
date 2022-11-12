use atlier::system::{Attribute, Value};
use reality::{
    wire::{Frame, FrameIndex, ResourceId, WireObject},
    BlockIndex, BlockProperties, Keywords,
};
use specs::{Component, WorldExt};

use super::{AttributeGraph, AttributeIndex};

impl WireObject for AttributeGraph {
    fn encode<BlobImpl>(&self, world: &specs::World, encoder: &mut reality::wire::Encoder<BlobImpl>)
    where
        BlobImpl: std::io::Read + std::io::Write + std::io::Seek + Clone + Default,
    {
        encoder.interner.add_ident("graph");
        encoder.interner.add_ident("properties");
        encoder.interner.add_ident("control_values");

        let entity = world.entities().entity(self.entity_id());
        encoder.last_entity = Some(entity);

        let frame = Frame::extension("graph", "properties").with_parity(entity);
        encoder.frames.push(frame);

        let properties = self.resolve_properties();
        properties.encode(world, encoder);

        let frame = Frame::extension("graph", "control_values").with_parity(entity);
        encoder.frames.push(frame);
        for (name, value) in self.control_values() {
            encoder.interner.add_ident(name);
            let frame = Frame::add(name, value, &mut encoder.blob_device).with_parity(entity);
            encoder.frames.push(frame);
        }
    }

    fn decode(
        protocol: &reality::wire::Protocol,
        interner: &reality::wire::Interner,
        blob_device: &std::io::Cursor<Vec<u8>>,
        frames: &[reality::wire::Frame],
    ) -> Self {
        let properties_frame = frames.get(0).expect("should have a properties frame");
        let control_values_pos = frames
            .iter()
            .position(|f| {
                f.keyword() == Keywords::Extension
                    && f.name(interner).expect("should have name").as_str() == "graph"
                    && f.symbol(interner).expect("should have symbol").as_str() == "control_values"
            })
            .expect("should have a frame for control_values");

        let properties = BlockProperties::decode(
            protocol,
            interner,
            blob_device,
            &frames[1..control_values_pos],
        );

        let entity =
            properties_frame.get_entity(protocol.as_ref(), protocol.assert_entity_generation());
        let attr = Attribute::new(entity.id(), properties.name(), Value::Empty);
        let mut block_index = BlockIndex::new(&attr);
        *block_index.properties_mut() = properties;

        for frame in frames[control_values_pos..].iter() {
            if frame.keyword() == Keywords::Add {
                let name = frame.name(&interner).expect("should have a name");
                let value = frame
                    .read_value(&interner, &blob_device)
                    .expect("should have a value");
                block_index.add_control(name, value);
            }
        }

        AttributeGraph::new(block_index)
    }

    fn build_index(
        interner: &reality::wire::Interner,
        frames: &[reality::wire::Frame],
    ) -> reality::wire::FrameIndex {
        let mut frame_index = FrameIndex::default();

        let mut delimitters = vec![];

        for (idx, frame) in frames.iter().enumerate() {
            match frame.keyword() {
                reality::Keywords::Extension => {
                    let extension = (frame.name(interner), frame.symbol(interner));

                    match extension {
                        (Some(name), Some(symbol)) => match (name.as_str(), symbol.as_str()) {
                            ("graph", "properties") => {
                                delimitters.push(idx);
                            }
                            _ => {
                                continue;
                            }
                        },
                        _ => {
                            continue;
                        }
                    }
                }
                _ => {
                    continue;
                }
            }
        }

        for (start, end) in delimitters.iter().zip(delimitters.iter().skip(1)) {
            let range = *start..*end;
            frame_index.insert(format!("graph-{start}-{end}"), vec![range]);
        }

        if let Some(last) = delimitters.last() {
            let range = *last..frames.len();
            frame_index.insert(format!("graph-{last}-{}", frames.len()), vec![range]);
        }

        frame_index
    }

    fn resource_id() -> reality::wire::ResourceId {
        ResourceId::new::<<AttributeGraph as Component>::Storage>()
    }
}

mod tests {
    #[test]
    fn test_graph_wire_object() {
        use reality::wire::Protocol;
        use crate::state::{AttributeGraph, AttributeIndex};
        use reality::wire::WireObject;
        use reality::BlockProperties;
        use specs::WorldExt;
        
        let mut protocol = Protocol::empty();
        protocol.as_mut().register::<BlockProperties>();

        protocol.encoder::<AttributeGraph>(|world, encoder| {
            let mut graph = AttributeGraph::default();
            graph.with_symbol("test", "test_symbol");
            graph.with_bool("test_bool", false);
            graph.with_int("test_int", 10);
            graph.encode(world, encoder);

            let mut graph = AttributeGraph::default();
            graph.with_symbol("test", "test_symbol2");
            graph.with_bool("test_bool", true);
            graph.with_int("test_int", 40);
            graph.encode(world, encoder);

            let mut graph = AttributeGraph::default();
            graph.with_symbol("test", "test_symbol4");
            graph.with_bool("test_bool", true);
            graph.with_int("test_int", 60);
            graph.encode(world, encoder);

            let graph = AttributeGraph::build_index(&encoder.interner, &encoder.frames);
            eprintln!("{:#?}", graph);
            assert!(graph.len() == 3);

            encoder.frame_index = graph;
        });


        let graphs = protocol.decode::<AttributeGraph>();

        let graph = graphs.get(0).expect("Should have a graph");
        assert_eq!(graph.find_symbol("test").expect("should exist"), "test_symbol");
        assert_eq!(graph.find_bool("test_bool").expect("should exist"), false);
        assert_eq!(graph.find_int("test_int").expect("should exist"), 10);

        // Alphabetically this is in the second position, but this is the 3rd graph from above
        let graph = graphs.get(1).expect("Should have a graph");
        assert_eq!(graph.find_symbol("test").expect("should exist"), "test_symbol4");
        assert_eq!(graph.find_bool("test_bool").expect("should exist"), true);
        assert_eq!(graph.find_int("test_int").expect("should exist"), 60);

        let graph = graphs.get(2).expect("Should have a graph");
        assert_eq!(graph.find_symbol("test").expect("should exist"), "test_symbol2");
        assert_eq!(graph.find_bool("test_bool").expect("should exist"), true);
        assert_eq!(graph.find_int("test_int").expect("should exist"), 40);
    }
}
