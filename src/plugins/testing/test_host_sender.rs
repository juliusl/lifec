use std::{fs::File, path::PathBuf, sync::Arc};

use reality::{BlockObject, BlockProperties};
use specs::RunNow;
use tracing::{event, Level};

use crate::{
    debugger::Debugger,
    engine::Performance,
    guest::{Guest, Sender},
    host::EventHandler,
    prelude::{Appendix, Editor, Host, Plugin, RunmdFile, Sequencer, NodeStatus, Journal},
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

                    let test_dir = PathBuf::from(".world/test.io/test_host");
                    if guest.send_commands(test_dir) {
                        event!(Level::WARN, "Commands not sent, previous commands have not been read");
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
                            .join("test_host");

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

                        let remote_dir = work_dir.join("journal");
                        let control = remote_dir.join("control");
                        let frames = remote_dir.join("frames");
                        let blob = remote_dir.join("blob");
                        let journal_updated = if control.exists() && frames.exists() && blob.exists() {
                            protocol.clear::<Journal>();
                            protocol.receive::<Journal, _, _>(
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

                        performance_updated | status_updated | journal_updated
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
