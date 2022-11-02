use std::{fs::File, sync::Arc, path::PathBuf};

use reality::{wire::Protocol, BlockObject, BlockProperties};
use specs::RunNow;
use tracing::{event, Level};

use crate::{
    guest::Guest,
    host::EventHandler,
    prelude::{
        Appendix, Editor, EventRuntime, Host, Listener, NodeCommand, Plugin, Project, Sequencer, Plugins,
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
                let root = tc.workspace().expect("Should have a workspace");
                let world = TestHost::compile_workspace(root, [].iter(), None);
                let mut host = Host::from(world);
                host.link_sequences();
                host.enable_listener::<TestHost>();
                host.build_appendix();
                if let Some(appendix) = host.world_mut().remove::<Appendix>() {
                    host.world_mut().insert(Arc::new(appendix));
                }
                let _ = host.prepare::<TestHost>();

                let guest = Guest::new::<TestHost>(tc.entity().unwrap(), host, |host| {
                    EventRuntime::default().run_now(host.world());
                    EventHandler::<TestHost>::default().run_now(host.world());

                    let test_dir = PathBuf::from(".test");
                    std::fs::create_dir_all(&test_dir).expect("should be able to create dirs");
                    
                    if test_dir.join("control").exists() {
                        fn read_stream(name: &'static str) -> impl FnOnce() -> File + 'static {
                            move || {
                                match std::fs::OpenOptions::new()
                                    .read(true)
                                    .open(name) {
                                        Ok(file) => {
                                            file
                                        },
                                        Err(err) => {
                                            panic!("{name} {err}")
                                        },
                                    }
                            }
                        }
    
                        let mut protocol = Protocol::empty();
                        protocol.receive::<NodeCommand, _, _>(
                            read_stream(".test/control"),
                            read_stream(".test/frames"),
                            read_stream(".test/blob"),
                        );
                        let mut handled = false;
                        for command in protocol.decode::<NodeCommand>() {
                            host.world()
                                .system_data::<Plugins>()
                                .features()
                                .broker()
                                .try_send_node_command(command.clone(), None)
                                .ok();
                            handled = true;
                        }
                        if handled {
                            std::fs::remove_dir_all(".test").ok();
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
