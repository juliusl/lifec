use std::{fs::File, path::PathBuf, sync::Arc};

use reality::{BlockObject, BlockProperties};
use specs::RunNow;
use tracing::{event, Level};

use crate::{
    engine::{Performance, Profilers, Cleanup},
    guest::Guest,
    host::EventHandler,
    prelude::{
        Appendix, Editor, EventRuntime, Host, Listener, Plugin, Project, RunmdFile, Sequencer,
    },
};

#[derive(Default)]
pub struct TestHost;

impl Project for TestHost {
    fn interpret(_: &specs::World, _: &reality::Block) {}
}

impl Listener for TestHost {
    fn create(_: &specs::World) -> Self {
        TestHost::default()
    }

    fn on_runmd(&mut self, _: &crate::prelude::RunmdFile) {}

    fn on_status_update(&mut self, _: &crate::prelude::StatusUpdate) {}

    fn on_operation(&mut self, _: crate::prelude::Operation) {}

    fn on_error_context(&mut self, _: &crate::prelude::ErrorContext) {}

    fn on_completed_event(&mut self, e: &specs::Entity) {
        event!(Level::DEBUG, "Guest plugin -- {}", e.id());
    }

    fn on_start_command(&mut self, _: &crate::prelude::Start) {}
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
                host.link_sequences();
                host.enable_listener::<TestHost>();
                host.build_appendix();
                if let Some(appendix) = host.world_mut().remove::<Appendix>() {
                    host.world_mut().insert(Arc::new(appendix));
                }
                let _ = host.prepare::<TestHost>();

                let test_dir = PathBuf::from(".world/test.io/test_host");
                std::fs::create_dir_all(&test_dir).expect("should be able to create dirs");
                let guest = Guest::new::<TestHost>(tc.entity().unwrap(), host, |host| {
                    EventRuntime::default().run_now(host.world());
                    Cleanup::default().run_now(host.world());
                    EventHandler::<TestHost>::default().run_now(host.world());

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
