
use atlier::system::{Attribute, Value};
use reality::{
    wire::{Encoder, Frame, FrameIndex, WireObject},
    BlockIndex, Keywords,
};
use specs::{shred::ResourceId, Component, WorldExt};
use tracing::{event, Level};

use crate::{
    prelude::EventStatus,
    state::{AttributeGraph, AttributeIndex}, engine::EngineStatus,
};

use super::{NodeCommand, NodeStatus};

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
                },
                crate::engine::EngineStatus::Active(e) => {
                    let frame = Frame::extension("engine_status", "active").with_parity(*e);
                    encoder.frames.push(frame);
                },
                crate::engine::EngineStatus::Disposed(e) => {
                    let frame = Frame::extension("engine_status", "disposed").with_parity(*e);
                    encoder.frames.push(frame);
                },
            },
            NodeStatus::Event(event) => match event {
                EventStatus::Scheduled(e) => {
                    let frame = Frame::extension("event_status", "scheduled").with_parity(*e);
                    encoder.frames.push(frame);
                },
                EventStatus::New(e) => {
                    let frame = Frame::extension("event_status", "new").with_parity(*e);
                    encoder.frames.push(frame);
                },
                EventStatus::InProgress(e) => {
                    let frame = Frame::extension("event_status", "in_progress").with_parity(*e);
                    encoder.frames.push(frame);
                }
                EventStatus::Paused(e) => {
                    let frame = Frame::extension("event_status", "paused").with_parity(*e);
                    encoder.frames.push(frame);
                },
                EventStatus::Ready(e) => {
                    let frame = Frame::extension("event_status", "ready").with_parity(*e);
                    encoder.frames.push(frame);
                },
                EventStatus::Completed(e) => {
                    let frame = Frame::extension("event_status", "completed").with_parity(*e);
                    encoder.frames.push(frame);
                },
                EventStatus::Cancelled(e) => {
                    let frame = Frame::extension("event_status", "cancelled").with_parity(*e);
                    encoder.frames.push(frame);
                },
                EventStatus::Inactive(e) => {
                    let frame = Frame::extension("event_status", "inactive").with_parity(*e);
                    encoder.frames.push(frame);
                }
                EventStatus::Disposed(e) => {
                    let frame = Frame::extension("event_status", "disposed").with_parity(*e);
                    encoder.frames.push(frame);
                },
            },
            NodeStatus::Profiler(e) => {
                let frame = Frame::extension("node_status", "profiler").with_parity(*e);
                encoder.frames.push(frame);
            },
            NodeStatus::Custom(e) => {
                let frame = Frame::extension("node_status", "custom").with_parity(*e);
                encoder.frames.push(frame);
            },
            NodeStatus::Empty => {},
        }
    }

    fn decode(
        protocol: &reality::wire::Protocol,
        interner: &reality::wire::Interner,
        _: &std::io::Cursor<Vec<u8>>,
        frames: &[Frame],
    ) -> Self {
      let frame = frames.get(0).expect("should hav a frame");
      let entity = frame.get_entity(protocol.as_ref(), protocol.assert_entity_generation());
      match (frame.name(interner).unwrap_or_default().as_str(), frame.symbol(interner).unwrap_or_default().as_str()) {
        ("engine_status", status) => NodeStatus::Engine(match status {
            "active" => EngineStatus::Active(entity),
            "inactive" => EngineStatus::Inactive(entity),
            "disposed" => EngineStatus::Disposed(entity),
            _ => panic!("unknown engine status symbol")
        }),
        ("event_status", status) => NodeStatus::Event(match status{
            "scheduled" => EventStatus::Scheduled(entity),
            "new" => EventStatus::New(entity),
            "in_progress" => EventStatus::InProgress(entity),
            "paused" => EventStatus::Paused(entity),
            "ready" => EventStatus::Ready(entity),
            "completed" => EventStatus::Completed(entity),
            "cancelled" => EventStatus::Cancelled(entity),
            "inactive" => EventStatus::Inactive(entity),
            "disposed" => EventStatus::Disposed(entity),
            _ =>  panic!("unknown engine status symbol")
        }),
        ("node_status", status) => match status {
            "profiler" => NodeStatus::Profiler(entity),
            "custom" => NodeStatus::Custom(entity),
            _ => NodeStatus::Empty
        },
        _ => NodeStatus::Empty
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

    fn decode(
        protocol: &reality::wire::Protocol,
        interner: &reality::wire::Interner,
        blob_device: &std::io::Cursor<Vec<u8>>,
        frames: &[reality::wire::Frame],
    ) -> Self {
        match frames.get(0) {
            Some(frame) => {
                assert_eq!(frame.keyword(), Keywords::Extension);
                let entity =
                    frame.get_entity(protocol.as_ref(), protocol.assert_entity_generation());
                match frame.symbol(interner).expect("should have a symbol").as_str() {
                    "activate" => NodeCommand::Activate(entity),
                    "reset" => NodeCommand::Reset(entity),
                    "pause" => NodeCommand::Pause(entity),
                    "resume" => NodeCommand::Resume(entity),
                    "cancel" => NodeCommand::Cancel(entity),
                    "spawn" => NodeCommand::Spawn(entity),
                    "swap" => {
                        let from = frames.get(1).expect("should have a from frame");
                        assert_eq!(from.name(interner).expect("should have a name").as_str(), "node_command");
                        assert_eq!(from.symbol(interner).expect("should have a name").as_str(), "swap.from");
                        let from = from.get_entity(protocol.as_ref(), protocol.assert_entity_generation());

                        let to = frames.get(2).expect("should have a to frame");
                        assert_eq!(to.name(interner).expect("should have a name").as_str(), "node_command");
                        assert_eq!(to.symbol(interner).expect("should have a name").as_str(), "swap.to");
                        let to = to.get_entity(protocol.as_ref(), protocol.assert_entity_generation());

                        NodeCommand::Swap { owner: entity, from, to }
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

                        NodeCommand::Custom(command.symbol(interner).expect("should have a symbol"), entity)
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

            match frame.symbol(interner).expect("should have a symbol").as_str() {
                "activate" | "reset" | "pause" | "resume" | "cancel" | "spawn" => {
                    let range = pos..pos + 1;
                    index.insert(format!("{idx}"), vec![range]);
                    pos += 1;
                }
                "update" => {
                    if let Some(epos) = frames[idx+1..].iter().position(|p| p.keyword() == Keywords::Extension) {
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
        use crate::prelude::{Appendix, Editor, Host};
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
