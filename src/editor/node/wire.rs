use std::{ops::Deref, sync::Arc};

use atlier::system::{Attribute, Value};
use reality::{
    wire::{Data, Encoder, Frame, FrameBuilder, FrameIndex, WireObject},
    BlockIndex, Elements,
};
use specs::{shred::ResourceId, Component, Entity, WorldExt};
use tracing::{event, Level};

use crate::{
    prelude::Appendix,
    state::{AttributeGraph, AttributeIndex},
};

use super::NodeCommand;

impl WireObject for NodeCommand {
    fn encode<BlobImpl>(&self, world: &specs::World, encoder: &mut reality::wire::Encoder<BlobImpl>)
    where
        BlobImpl: std::io::Read + std::io::Write + std::io::Seek + Clone,
    {
        let appendix = world.read_resource::<Arc<Appendix>>();

        match self {
            NodeCommand::Activate(entity) => {
                let frame = encode_node_command(0x10, *entity, appendix.deref().clone(), encoder);
                encoder.frames.push(frame);
            }
            NodeCommand::Reset(entity) => {
                let frame = encode_node_command(0x20, *entity, appendix.deref().clone(), encoder);
                encoder.frames.push(frame);
            }
            NodeCommand::Pause(entity) => {
                let frame = encode_node_command(0x30, *entity, appendix.deref().clone(), encoder);
                encoder.frames.push(frame);
            }
            NodeCommand::Resume(entity) => {
                let frame = encode_node_command(0x40, *entity, appendix.deref().clone(), encoder);
                encoder.frames.push(frame);
            }
            NodeCommand::Cancel(entity) => {
                let frame = encode_node_command(0x50, *entity, appendix.deref().clone(), encoder);
                encoder.frames.push(frame);
            }
            NodeCommand::Spawn(entity) => {
                let frame = encode_node_command(0x60, *entity, appendix.deref().clone(), encoder);
                encoder.frames.push(frame);
            }
            NodeCommand::Update(graph) => {
                let entity = world.entities().entity(graph.entity_id());
                let frame = encode_node_command(0x70, entity, appendix.deref().clone(), encoder);
                encoder.frames.push(frame);
                let mut index = graph.clone();
                let index = index.index();
                let symbol = index.root().name().to_string();
                encoder.interner.add_ident(&symbol);

                let frame = Frame::add(
                    index.root().name(),
                    index.root().value(),
                    &mut encoder.blob_device,
                );
                encoder.frames.push(frame);
                for (name, values) in graph.values() {
                    for value in values {
                        encoder.interner.add_ident(&name);
                        match &value {
                            Value::Symbol(symbol) => {
                                encoder.interner.add_ident(symbol);
                            }
                            Value::Complex(complex) => {
                                for c in complex.iter() {
                                    encoder.interner.add_ident(c);
                                }
                            }
                            _ => {}
                        }
                        let frame = Frame::define(&name, &symbol, &value, &mut encoder.blob_device)
                            .with_parity(entity);
                        encoder.frames.push(frame);
                    }
                }
                let frame = encode_node_command(0x71, entity, appendix.deref().clone(), encoder);
                encoder.frames.push(frame);
            }
            NodeCommand::Custom(name, entity) => {
                let frame = encode_node_command(0x80, *entity, appendix.deref().clone(), encoder);
                encoder.frames.push(frame);

                let frame = Frame::add(
                    "name",
                    &Value::Symbol(name.to_string()),
                    &mut encoder.blob_device,
                )
                .with_parity(*entity);

                encoder.interner.add_ident(name);
                encoder.frames.push(frame);
            }
        }
    }

    fn decode(
        protocol: &reality::wire::Protocol,
        interner: &reality::wire::Interner,
        blob_device: &std::io::Cursor<Vec<u8>>,
        frames: &[reality::wire::Frame],
    ) -> Self {
        match frames.get(0) {
            Some(frame) => {
                let entity = frame.get_entity(protocol.as_ref(), true);
                match frame.op() {
                    0x10 => NodeCommand::Activate(entity),
                    0x20 => NodeCommand::Reset(entity),
                    0x30 => NodeCommand::Pause(entity),
                    0x40 => NodeCommand::Resume(entity),
                    0x50 => NodeCommand::Cancel(entity),
                    0x60 => NodeCommand::Spawn(entity),
                    0x70 => {
                        let mut attributes = vec![];
                        for attr in frames.iter().skip(1) {
                            if attr.op() == 0x71 {
                                break;
                            }
                            match (
                                attr.name(interner),
                                attr.symbol(interner),
                                attr.read_value(interner, blob_device),
                            ) {
                                (Some(name), None, Some(value)) => {
                                    attributes.push(Attribute::new(entity.id(), &name, value));
                                }
                                (Some(name), Some(symbol), Some(value)) => {
                                    let mut attr = Attribute::new(
                                        entity.id(),
                                        format!("{symbol}::{name}"),
                                        Value::Empty,
                                    );

                                    attr.edit_as(value);
                                    attributes.push(attr);
                                }
                                _ => {}
                            }
                        }

                        eprintln!("{:#?}", attributes);

                        if let Some(index) = BlockIndex::index(attributes).first() {
                            NodeCommand::Update(AttributeGraph::new(index.clone()))
                        } else {
                            panic!("Could not get graph")
                        }
                    }
                    0x80 => {
                        if let Some(Value::Symbol(name)) = frames
                            .get(1)
                            .and_then(|name| name.read_value(interner, blob_device))
                        {
                            NodeCommand::Custom(name.to_string(), entity)
                        } else {
                            panic!("Name is required, {:?}", frames.get(1))
                        }
                    }
                    _ => {
                        panic!("Unrecognized start frame")
                    }
                }
            }
            None => {
                panic!("Trying to decode w/o any frames")
            }
        }
    }

    fn build_index(
        _: &reality::wire::Interner,
        frames: &[reality::wire::Frame],
    ) -> reality::wire::FrameIndex {
        let mut index = FrameIndex::default();
        let mut pos = 0;
        for (idx, frame) in frames.iter().enumerate() {
            match frame.op() {
                0x10 | 0x20 | 0x30 | 0x40 | 0x50 | 0x60 => {
                    let range = pos..pos + 1;
                    index.insert(format!("{idx}"), vec![range]);
                    pos += 1;
                }
                0x70 => {
                    if let Some(epos) = frames[idx..].iter().position(|p| p.op() == 0x71) {
                        let range = pos..pos + epos + 1;
                        index.insert(format!("{idx}"), vec![range]);
                        pos += epos + 1;
                    }
                }
                0x80 => {
                    let range = pos..pos + 2;
                    index.insert(format!("{idx}"), vec![range]);
                    pos += 2;
                }
                _ => {}
            }
        }
        index
    }

    fn resource_id() -> specs::shred::ResourceId {
        ResourceId::new::<<NodeCommand as Component>::Storage>()
    }
}

/// Encodes a node command into a frame,
///
fn encode_node_command<BlobImpl>(
    op: u8,
    entity: Entity,
    appendix: Arc<Appendix>,
    encoder: &mut Encoder<BlobImpl>,
) -> Frame
where
    BlobImpl: std::io::Read + std::io::Write + std::io::Seek + Clone,
{
    let mut frame = FrameBuilder::default();
    frame.write(Data::Operation(op), None::<&mut BlobImpl>).ok();
    if let Some(name) = appendix.name(&entity) {
        event!(Level::TRACE, "Encoding {}", name);
        encoder.interner.add_ident(name);
        frame
            .write(
                Elements::Identifier(name.to_string()),
                None::<&mut BlobImpl>,
            )
            .ok();
    }
    if let Some(control_symbol) = appendix.control_symbol(&entity) {
        event!(Level::TRACE, "Encoding {}", control_symbol);
        encoder.interner.add_ident(&control_symbol);
        frame
            .write(
                Elements::Identifier(control_symbol.to_string()),
                None::<&mut BlobImpl>,
            )
            .ok();
    }
    let frame: Frame = frame.into();
    frame.with_parity(entity)
}

mod tests {
    use crate::prelude::{Project};

    #[test]
    #[tracing_test::traced_test]
    fn test_protocol() {
        std::fs::remove_dir_all(".test").ok();
        use std::{fs::File, path::PathBuf};
        use super::NodeCommand;
        use crate::prelude::{Appendix, Editor, Host};
        use crate::state::{AttributeGraph, AttributeIndex};
        use reality::wire::Protocol;
        use reality::wire::WireObject;
        use reality::Parser;
        use specs::WorldExt;
        use std::sync::Arc;

        let mut host = Host::load_content::<Test>(
            r#"
        ``` test
        + .engine 
        : .start setup
        : .start run
        : .exit
        ```

        ``` setup test
        + .runtime
        : .println Hello World
        ```

        ``` run test
        + .runtime
        : .println Goodbye World
        ```
        "#,
        );
        host.build_appendix();

        if let Some(appendix) = host.world_mut().remove::<Appendix>() {
            host.world_mut().insert(Arc::new(appendix));
        }

        let mut protocol = Protocol::new(Parser::new_with(host.into()));

        // Record node command as wire objects in a protocol,
        //
        protocol.encoder::<NodeCommand>(|world, encoder| {
            let entity = world.entities().entity(4);
            encoder.encode(&NodeCommand::Activate(entity), world);

            let entity = world.entities().entity(2);
            encoder.encode(&NodeCommand::Spawn(entity), world);

            let entity = world.entities().entity(2);
            encoder.encode(&NodeCommand::Spawn(entity), world);

            let entity = world.entities().entity(4);
            encoder.encode(&NodeCommand::Activate(entity), world);

            let entity = world.entities().entity(4);
            encoder.encode(
                &NodeCommand::Custom("test_node_command".to_string(), entity),
                world,
            );

            let entity = world.entities().entity(5);
            let mut graph = world
                .read_component::<AttributeGraph>()
                .get(entity)
                .expect("should have graph")
                .clone();
            graph.with_symbol("testvalue", "test test").with_binary(
                "testbin",
                b"vec![0x0a, 0x12]lorem ipsum testsetsetsetet".to_vec(),
            );
            encoder.encode(&NodeCommand::Update(graph.to_owned()), world);

            for frame in encoder.frames.iter() {
                eprintln!("{:#}", frame);
            }

            encoder.frame_index =
                NodeCommand::build_index(&encoder.interner, encoder.frames_slice());
        });

        fn write_stream(name: &'static str) -> impl FnOnce() -> File + 'static {
            move || {
                std::fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(name)
                    .ok()
                    .unwrap()
            }
        }
    
        fn read_stream(name: &'static str) -> impl FnOnce() -> File + 'static {
            move || {
                std::fs::OpenOptions::new()
                    .read(true)
                    .open(name)
                    .ok()
                    .unwrap()
            }
        }


        let test_dir = PathBuf::from(".test");
        std::fs::create_dir_all(&test_dir).expect("should be able to create dirs");

        // Test sending wire data,
        //
        protocol.send::<NodeCommand, File, _>(
            write_stream(".test/control"),
            write_stream(".test/frames"),
            write_stream(".test/blob"),
        );
        for command in protocol.decode::<NodeCommand>() {
            eprintln!("{:#?}", command);
        }

        // Test receiving wire object
        //
        let mut protocol = Protocol::empty();
        protocol.receive::<NodeCommand, _, _>(
            read_stream(".test/control"), 
            read_stream(".test/frames"), 
            read_stream(".test/blob")
        );
        for command in protocol.decode::<NodeCommand>() {
            eprintln!("{:#?}", command);
        }
    }



    #[derive(Default)]
    struct Test;

    impl Project for Test {
        fn interpret(_: &specs::World, _: &reality::Block) {}
    }
}
