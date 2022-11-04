use std::{fs::File, path::PathBuf, sync::Arc};

use reality::{BlockObject, BlockProperties};
use specs::RunNow;

use crate::{
    engine::{Performance, Profilers, Cleanup},
    guest::Guest,
    host::EventHandler,
    prelude::{
        Appendix, Editor, EventRuntime, Host, Plugin, Project, RunmdFile, Sequencer,
    }, debugger::Debugger,
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
                    : .timer 1s
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
                let guest = Guest::new::<TestHost>(tc.entity().unwrap(), host, |host| {
                    EventRuntime::default().run_now(host.world());
                    Cleanup::default().run_now(host.world());
                    EventHandler::<Debugger>::default().run_now(host.world());

                    host.world().system_data::<Profilers>().profile();

                    if host.encode_performance() {
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

                        if let Some(protocol) = host.protocol_mut() {
                            protocol.send::<Performance, _, _>(
                                write_stream(".world/test.io/test_host/performance/control"),
                                write_stream(".world/test.io/test_host/performance/frames"),
                                write_stream(".world/test.io/test_host/performance/blob"),
                            );
                        }
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
