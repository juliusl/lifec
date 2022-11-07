use std::{fs::File, path::PathBuf, sync::Arc};

use reality::{BlockObject, BlockProperties};
use specs::{RunNow, WorldExt};

use crate::{
    debugger::Debugger,
    engine::{Performance, Profilers, Cleanup},
    guest::{Guest, RemoteProtocol},
    host::EventHandler,
    prelude::{
        Appendix, Editor, EventRuntime, Host, NodeStatus, Plugin, Project, RunmdFile, Sequencer,
        State, Journal
    },
};

#[derive(Default)]
pub struct TestHost;

impl Project for TestHost {
    fn interpret(_: &specs::World, _: &reality::Block) {}
}

impl Plugin for TestHost {
    fn symbol() -> &'static str {
        "test_host"
    }

    fn call(context: &mut crate::prelude::ThunkContext) -> Option<crate::prelude::AsyncContext> {
        context.task(|_| {
            let tc = context.clone();
            async {
                let mut root = tc.workspace().expect("Should have a workspace").clone();
                root.cache_file(&RunmdFile::new_src(
                    "listener",
                    r#"
                    ```
                    + .engine
                    : .start start
                    : .start cooldown
                    : .loop
                    ```

                    ``` start
                    + .runtime
                    : .watch test_host
                    : .create file
                    : .listen test_host
                    ```

                    ``` cooldown
                    + .runtime
                    : .timer 500ms
                    ```
                    "#,
                ));

                let mut world = root
                    .compile::<TestHost>()
                    .expect("should be able to compile");
                world.insert(None::<RemoteProtocol>);
                world.insert(None::<Debugger>);
                let mut host = Host::from(world);
                host.prepare::<TestHost>();
                host.link_sequences();
                host.build_appendix();
                host.enable_listener::<Debugger>();
                host.prepare::<TestHost>();
                if let Some(appendix) = host.world_mut().remove::<Appendix>() {
                    host.world_mut().insert(Arc::new(appendix));
                }

                let test_dir = PathBuf::from(".world/test.io/test_host");
                std::fs::create_dir_all(&test_dir).expect("should be able to create dirs");
                let guest = Guest::new::<TestHost>(tc.entity().unwrap(), host, |guest| {
                    EventRuntime::default().run_now(guest.protocol().as_ref());
                    Cleanup::default().run_now(guest.protocol().as_ref());
                    EventHandler::<Debugger>::default().run_now(guest.protocol().as_ref());

                    guest
                        .protocol()
                        .as_ref()
                        .system_data::<Profilers>()
                        .profile();

                    let nodes = guest
                        .protocol()
                        .as_ref()
                        .system_data::<State>()
                        .event_nodes();
                    for node in nodes {
                        guest
                            .protocol()
                            .as_ref()
                            .write_component()
                            .insert(node.status.entity(), node.status)
                            .expect("should be able to insert status");
                    }

                    if guest.encode_performance() {
                        let test_dir = PathBuf::from(".world/test.io/test_host/performance");
                        std::fs::create_dir_all(&test_dir).expect("should be able to create dirs");

                        fn write_stream(name: &'static str) -> impl FnOnce() -> File + 'static {
                            move || {
                                std::fs::OpenOptions::new()
                                    .create(true)
                                    .write(true)
                                    .open(name)
                                    .ok()
                                    .unwrap()
                            }
                        }

                        guest.update_protocol(|protocol| {
                            protocol.send::<Performance, _, _>(
                                write_stream(".world/test.io/test_host/performance/control"),
                                write_stream(".world/test.io/test_host/performance/frames"),
                                write_stream(".world/test.io/test_host/performance/blob"),
                            );
                            true
                        });
                    }

                    if guest.encode_status() {
                        let test_dir = PathBuf::from(".world/test.io/test_host/status");
                        std::fs::create_dir_all(&test_dir).expect("should be able to create dirs");

                        fn write_stream(name: &'static str) -> impl FnOnce() -> File + 'static {
                            move || {
                                std::fs::OpenOptions::new()
                                    .create(true)
                                    .write(true)
                                    .open(name)
                                    .ok()
                                    .unwrap()
                            }
                        }

                        guest.update_protocol(|protocol| {
                            protocol.send::<NodeStatus, _, _>(
                                write_stream(".world/test.io/test_host/status/control"),
                                write_stream(".world/test.io/test_host/status/frames"),
                                write_stream(".world/test.io/test_host/status/blob"),
                            );
                            true
                        });
                    }

                    let remote_dir = PathBuf::from(".world/test.io/test_host").join("journal");
                    let control = remote_dir.join("control");
                    let frames = remote_dir.join("frames");
                    let blob = remote_dir.join("blob");
                    let journal_exists = control.exists() && frames.exists() && blob.exists();
                   
                    if !journal_exists && guest.encode_journal() {
                        let test_dir = PathBuf::from(".world/test.io/test_host/journal");
                        std::fs::create_dir_all(&test_dir).expect("should be able to create dirs");

                        fn write_stream(name: &'static str) -> impl FnOnce() -> File + 'static {
                            move || {
                                std::fs::OpenOptions::new()
                                    .create(true)
                                    .write(true)
                                    .open(name)
                                    .ok()
                                    .unwrap()
                            }
                        }

                        guest.update_protocol(|protocol| {
                            protocol.send::<Journal, _, _>(
                                write_stream(".world/test.io/test_host/journal/control"),
                                write_stream(".world/test.io/test_host/journal/frames"),
                                write_stream(".world/test.io/test_host/journal/blob"),
                            );
                            true
                        });
                    }
                });

                // guest.add_node(Node {
                //     status: NodeStatus::Custom(tc.entity().unwrap()),
                //     edit: Some(|n, ui| {
                //         let mut opened = true;
                //         imgui::Window::new("test").opened(&mut opened).build(ui, ||{
                //             ui.text("test window");
                //             if ui.button("test") {
                //                 n.custom("test", n.status.entity());
                //             }
                //         });
                //         opened
                //     }),
                //     .. Default::default()
                // });

                tc.enable_guest(guest);

                Some(tc)
            }
        })
    }
}

impl BlockObject for TestHost {
    fn query(&self) -> reality::BlockProperties {
        BlockProperties::default()
    }

    fn parser(&self) -> Option<reality::CustomAttribute> {
        Some(Self::as_custom_attr())
    }
}
