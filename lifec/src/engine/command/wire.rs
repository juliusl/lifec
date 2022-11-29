use reality::{
    wire::{Frame, FrameIndex, WireObject},
    Attribute, BlockIndex, Keywords, Value,
};
use specs::{shred::ResourceId, Component, WorldExt};
use tracing::{event, Level};

use crate::state::{AttributeGraph, AttributeIndex};

use super::NodeCommand;

impl WireObject for NodeCommand {
    fn encode<BlobImpl>(&self, world: &specs::World, encoder: &mut reality::wire::Encoder<BlobImpl>)
    where
        BlobImpl: std::io::Read + std::io::Write + std::io::Seek + Clone + Default,
    {
        encoder.interner.add_ident("node_command");
        encoder.interner.add_ident("activate");
        encoder.interner.add_ident("reset");
        encoder.interner.add_ident("pause");
        encoder.interner.add_ident("resume");
        encoder.interner.add_ident("cancel");
        encoder.interner.add_ident("spawn");
        encoder.interner.add_ident("update");
        encoder.interner.add_ident("swap");
        encoder.interner.add_ident("swap.from");
        encoder.interner.add_ident("swap.to");
        encoder.interner.add_ident("custom");

        match self {
            NodeCommand::Activate(e) => {
                let frame = Frame::extension("node_command", "activate").with_parity(*e);
                encoder.frames.push(frame);
            }
            NodeCommand::Reset(e) => {
                let frame = Frame::extension("node_command", "reset").with_parity(*e);
                encoder.frames.push(frame);
            }
            NodeCommand::Pause(e) => {
                let frame = Frame::extension("node_command", "pause").with_parity(*e);
                encoder.frames.push(frame);
            }
            NodeCommand::Resume(e) => {
                let frame = Frame::extension("node_command", "resume").with_parity(*e);
                encoder.frames.push(frame);
            }
            NodeCommand::Cancel(e) => {
                let frame = Frame::extension("node_command", "cancel").with_parity(*e);
                encoder.frames.push(frame);
            }
            NodeCommand::Spawn(e) => {
                let frame = Frame::extension("node_command", "spawn").with_parity(*e);
                encoder.frames.push(frame);
            }
            NodeCommand::Update(graph) => {
                let entity = world.entities().entity(graph.entity_id());
                let frame = Frame::extension("node_command", "update").with_parity(entity);
                encoder.frames.push(frame);

                let index = graph.clone();
                let index = index.index();
                let symbol = index.root().name().to_string();
                encoder.interner.add_ident(&symbol);

                let frame = Frame::add(
                    index.root().name(),
                    index.root().value(),
                    &mut encoder.blob_device,
                );

                match index.root().value() {
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
            }
            NodeCommand::Swap { owner, from, to } => {
                let frame = Frame::extension("node_command", "swap").with_parity(*owner);
                encoder.frames.push(frame);

                let frame = Frame::extension("node_command", "swap.from").with_parity(*from);
                encoder.frames.push(frame);

                let frame = Frame::extension("node_command", "swap.to").with_parity(*to);
                encoder.frames.push(frame);
            }
            NodeCommand::Custom(command, e) => {
                encoder.interner.add_ident(command);

                let frame = Frame::extension("node_command", "custom").with_parity(*e);
                encoder.frames.push(frame);
                let frame = Frame::extension("custom", command).with_parity(*e);
                encoder.frames.push(frame);
            }
        }
    }

    fn decode<BlobImpl>(
        protocol: &reality::wire::Protocol<BlobImpl>,
        interner: &reality::wire::Interner,
        blob_device: &BlobImpl,
        frames: &[reality::wire::Frame],
    ) -> Self
    where
        BlobImpl: std::io::Read + std::io::Write + std::io::Seek + Clone + Default,
    {
        match frames.get(0) {
            Some(frame) => {
                assert_eq!(frame.keyword(), Keywords::Extension);
                let entity =
                    frame.get_entity(protocol.as_ref(), protocol.assert_entity_generation());
                match frame
                    .symbol(interner)
                    .expect("should have a symbol")
                    .as_str()
                {
                    "activate" => NodeCommand::Activate(entity),
                    "reset" => NodeCommand::Reset(entity),
                    "pause" => NodeCommand::Pause(entity),
                    "resume" => NodeCommand::Resume(entity),
                    "cancel" => NodeCommand::Cancel(entity),
                    "spawn" => NodeCommand::Spawn(entity),
                    "swap" => {
                        let from = frames.get(1).expect("should have a from frame");
                        assert_eq!(
                            from.name(interner).expect("should have a name").as_str(),
                            "node_command"
                        );
                        assert_eq!(
                            from.symbol(interner).expect("should have a name").as_str(),
                            "swap.from"
                        );
                        let from =
                            from.get_entity(protocol.as_ref(), protocol.assert_entity_generation());

                        let to = frames.get(2).expect("should have a to frame");
                        assert_eq!(
                            to.name(interner).expect("should have a name").as_str(),
                            "node_command"
                        );
                        assert_eq!(
                            to.symbol(interner).expect("should have a name").as_str(),
                            "swap.to"
                        );
                        let to =
                            to.get_entity(protocol.as_ref(), protocol.assert_entity_generation());

                        NodeCommand::Swap {
                            owner: entity,
                            from,
                            to,
                        }
                    }
                    "update" => {
                        let mut attributes = vec![];
                        for attr in frames.iter().skip(1) {
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

                        if let Some(index) = BlockIndex::index(attributes.clone()).first() {
                            NodeCommand::Update(AttributeGraph::new(index.clone()))
                        } else {
                            event!(Level::ERROR, "{:#?}", attributes);
                            panic!("Could not get graph")
                        }
                    }
                    "custom" => {
                        let command = frames.get(1).expect("should have a command frame");
                        assert_eq!(command.name(interner), Some("custom".to_string()));

                        NodeCommand::Custom(
                            command.symbol(interner).expect("should have a symbol"),
                            entity,
                        )
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
        interner: &reality::wire::Interner,
        frames: &[reality::wire::Frame],
    ) -> reality::wire::FrameIndex {
        let mut index = FrameIndex::default();
        let mut pos = 0;
        for (idx, frame) in frames.iter().enumerate() {
            if frame.keyword() != Keywords::Extension {
                continue;
            }

            match frame
                .symbol(interner)
                .expect("should have a symbol")
                .as_str()
            {
                "activate" | "reset" | "pause" | "resume" | "cancel" | "spawn" => {
                    let range = pos..pos + 1;
                    index.insert(format!("{idx}"), vec![range]);
                    pos += 1;
                }
                "update" => {
                    if let Some(epos) = frames[idx + 1..]
                        .iter()
                        .position(|p| p.keyword() == Keywords::Extension)
                    {
                        let range = pos..pos + epos; // + 1 to include op 0x71
                        index.insert(format!("{idx}"), vec![range]);
                        pos += epos + 1;
                    } else {
                        let range = pos..frames.len(); // + 1 to include op 0x71
                        index.insert(format!("{idx}"), vec![range]);
                    }
                }
                "custom" => {
                    let range = pos..pos + 2;
                    index.insert(format!("{idx}"), vec![range]);
                    pos += 2;
                }
                "swap" => {
                    let range = pos..pos + 3;
                    index.insert(format!("{idx}"), vec![range]);
                    pos += 3;
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

mod tests {
    use crate::prelude::Project;

    #[test]
    #[tracing_test::traced_test]
    fn test_protocol() {
        std::fs::remove_dir_all(".test").ok();
        use super::NodeCommand;
        use crate::appendix::Appendix;
        use crate::prelude::Host;
        use crate::state::{AttributeGraph, AttributeIndex};
        use reality::wire::Protocol;
        use reality::wire::WireObject;
        use reality::Parser;
        use specs::WorldExt;
        use std::sync::Arc;
        use std::{fs::File, path::PathBuf};

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
        protocol.send::<NodeCommand, _, _>(
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
            read_stream(".test/blob"),
        );
        for command in protocol.decode::<NodeCommand>() {
            eprintln!("{:#?}", command);
        }

        protocol.ensure_encoder::<NodeCommand>().clear();

        assert!(protocol.decode::<NodeCommand>().is_empty());
    }

    #[derive(Default)]
    struct Test;

    impl Project for Test {
        fn interpret(_: &specs::World, _: &reality::Block) {}
    }
}
