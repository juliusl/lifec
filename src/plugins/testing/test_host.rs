use std::{path::PathBuf, sync::Arc};

use reality::{BlockObject, BlockProperties};
use specs::{RunNow, WorldExt};

use crate::{
    debugger::Debugger,
    editor::{CommandDispatcher, EventNode},
    engine::{Cleanup, Profilers},
    guest::{Guest, RemoteProtocol, Monitor},
    host::EventHandler,
    prelude::{
        Appendix, Editor, EventRuntime, Host, Node, NodeStatus, Plugin, Project,
        RunmdFile, Sequencer, State,
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
                let mut guest = Guest::new::<TestHost>(tc.entity().unwrap(), host, |guest| {
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

                    let test_host_dir = PathBuf::from(".world/test.io/test_host");
                    guest.update_performance(&test_host_dir);
                    guest.update_status(&test_host_dir);
                    guest.update_journal(&test_host_dir);
                });

                guest.add_node(Node {
                    status: NodeStatus::Custom(tc.entity().unwrap()),
                    remote_protocol: Some(guest.subscribe()),
                    edit: Some(|n, ui| {
                        let mut opened = true;
                        imgui::Window::new("test")
                            .opened(&mut opened)
                            .build(ui, || {
                                ui.text("test window");

                                if let Some(rp) = n.remote_protocol.as_ref() {
                                    for mut a in rp
                                        .remote
                                        .borrow()
                                        .as_ref()
                                        .system_data::<State>()
                                        .event_nodes()
                                    {
                                        match a.status {
                                            NodeStatus::Event(status) => {
                                                a.event_buttons(ui, status);
                                            }
                                            _ => {}
                                        }
                                    }
                                }

                                if ui.button("test") {
                                    n.custom("test", n.status.entity());
                                }
                            });
                        opened
                    }),
                    ..Default::default()
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
