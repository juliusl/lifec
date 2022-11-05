use std::{fs::File, path::PathBuf, sync::Arc};

use reality::{BlockObject, BlockProperties};
use specs::RunNow;

use crate::{
    debugger::Debugger,
    engine::{Cleanup, Performance, Profilers},
    guest::Guest,
    host::EventHandler,
    prelude::{Appendix, Editor, EventRuntime, Host, Plugin, Project, RunmdFile, Sequencer},
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

                let world = root
                    .compile::<TestHost>()
                    .expect("should be able to compile");
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

                    guest.protocol().as_ref().system_data::<Profilers>().profile();

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
                });
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
