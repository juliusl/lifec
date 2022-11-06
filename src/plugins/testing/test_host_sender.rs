use std::{fs::File, path::PathBuf, sync::Arc};

use reality::{BlockObject, BlockProperties};
use specs::RunNow;

use crate::{
    debugger::Debugger,
    engine::Performance,
    guest::Guest,
    host::EventHandler,
    prelude::{Appendix, Editor, Host, NodeCommand, Plugin, RunmdFile, Sequencer, NodeStatus},
};

use super::TestHost;

#[derive(Default)]
pub struct TestHostSender;

impl Plugin for TestHostSender {
    fn symbol() -> &'static str {
        "test_host_sender"
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
                    : .timer 1s
                    ```
                    "#,
                ));

                let mut world = root
                    .compile::<TestHost>()
                    .expect("should be able to compile");
                world.insert(None::<Debugger>);
                let mut host = Host::from(world);
                host.prepare::<TestHost>();
                host.link_sequences();
                host.build_appendix();
                host.enable_listener::<()>();
                host.prepare::<TestHost>();
                if let Some(appendix) = host.world_mut().remove::<Appendix>() {
                    host.world_mut().insert(Arc::new(appendix));
                }
                let mut guest = Guest::new::<TestHost>(tc.entity().unwrap(), host, |guest| {
                    EventHandler::<()>::default().run_now(guest.protocol().as_ref());

                    if guest.encode_commands() {
                        let test_dir = PathBuf::from(".world/test.io/test_host");
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

                        if guest.update_protocol(|protocol| {
                            protocol.send::<NodeCommand, _, _>(
                                write_stream(".world/test.io/test_host/control"),
                                write_stream(".world/test.io/test_host/frames"),
                                write_stream(".world/test.io/test_host/blob"),
                            );

                            true
                        }) {}
                    }

                    let workspace = guest.workspace().clone();
                    if guest.update_protocol(move |protocol| {
                        fn read_stream<'a>(name: &'a PathBuf) -> impl FnOnce() -> File + 'a {
                            move || {
                                std::fs::OpenOptions::new()
                                    .read(true)
                                    .open(name)
                                    .ok()
                                    .unwrap()
                            }
                        }

                        let work_dir = workspace
                            .work_dir()
                            .join(".world/test.io/test_host");

                        let performance_dir = work_dir.join("performance");
                        let control = performance_dir.join("control");
                        let frames = performance_dir.join("frames");
                        let blob = performance_dir.join("blob");

                        let performance_updated = if control.exists() && frames.exists() && blob.exists() {
                            protocol.clear::<Performance>();
                            protocol.receive::<Performance, _, _>(
                                read_stream(&control),
                                read_stream(&frames),
                                read_stream(&blob),
                            );

                            std::fs::remove_file(control).ok();
                            std::fs::remove_file(frames).ok();
                            std::fs::remove_file(blob).ok();
                            true
                        } else {
                            false
                        };

                        let status_dir = work_dir.join("status");
                        let control = status_dir.join("control");
                        let frames = status_dir.join("frames");
                        let blob = status_dir.join("blob");
                        let status_updated = if control.exists() && frames.exists() && blob.exists() {
                            protocol.clear::<NodeStatus>();
                            protocol.receive::<NodeStatus, _, _>(
                                read_stream(&control),
                                read_stream(&frames),
                                read_stream(&blob),
                            );

                            std::fs::remove_file(control).ok();
                            std::fs::remove_file(frames).ok();
                            std::fs::remove_file(blob).ok();
                            true
                        } else {
                            false
                        };

                        // let remote_dir = work_dir.join("remote");
                        // let control = remote_dir.join("control");
                        // let frames = remote_dir.join("frames");
                        // let blob = remote_dir.join("blob");
                        // let remote_updated = if control.exists() && frames.exists() && blob.exists() {
                        //     protocol.receive::<Remote, _, _>(
                        //         read_stream(&control),
                        //         read_stream(&frames),
                        //         read_stream(&blob),
                        //     );

                        //     protocol.decode::<Remote>();

                        //     std::fs::remove_file(control).ok();
                        //     std::fs::remove_file(frames).ok();
                        //     std::fs::remove_file(blob).ok();
                        //     true
                        // } else {
                        //     false
                        // };

                        performance_updated | status_updated 
                    }) {
                        
                    }
                });

                guest.enable_remote();
                tc.enable_guest(guest);
                Some(tc)
            }
        })
    }
}

impl BlockObject for TestHostSender {
    fn query(&self) -> BlockProperties {
        BlockProperties::default()
    }

    fn parser(&self) -> Option<reality::CustomAttribute> {
        Some(Self::as_custom_attr())
    }
}
