use std::{fs::File, sync::Arc, path::PathBuf};

use reality::{BlockObject, BlockProperties};
use specs::RunNow;

use crate::{
    guest::Guest,
    host::EventHandler,
    prelude::{
        Appendix, Editor, Host, NodeCommand, Plugin, Project, Sequencer,
    },
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
                    EventHandler::<TestHost>::default().run_now(host.world());

                    if host.encode_commands() {
                        let test_dir = PathBuf::from(".test");
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
                            protocol.send::<NodeCommand, _, _>(
                                write_stream(".test/control"),
                                write_stream(".test/frames"),
                                write_stream(".test/blob"),
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

impl BlockObject for TestHostSender {
    fn query(&self) -> BlockProperties {
        BlockProperties::default()
    }

    fn parser(&self) -> Option<reality::CustomAttribute> {
        Some(Self::as_custom_attr())
    }
}