use std::net::SocketAddr;

use atlier::system::Extension;
use specs::{Component, Join, Read, WorldExt, WriteStorage};
use specs::{DenseVecStorage, Entities, ReadStorage, System};
use tokio::task::JoinHandle;
use tracing::{event, Level};

use crate::AttributeIndex;

use super::{CancelThunk, ErrorContext, Event, EventRuntime, ThunkContext};

mod proxy;
pub use proxy::ProxiedMessage;
pub use proxy::Proxy;
pub use proxy::ProxyRuntime;

mod address;
pub use address::BlockAddress;

mod udp;
pub use udp::UDP;

mod tcp;
pub use tcp::TCP;

/// Network runtime is similar to the event runtime, but instead fires events
/// when a network task has completed
///
#[derive(Default)]
pub struct NetworkRuntime;

/// Network events returned by network tasks
///
#[derive(Debug)]
pub enum NetworkEvent {
    /// This event is created when a proxy component has received bytes
    ///
    Received(
        /// The number of bytes received
        usize,
        /// The address these bytes were received from
        SocketAddr,
    ),
    /// This event is created when a proxy component has sent bytes
    /// to an upstream entity
    Proxied(
        /// upstream entity id to notify,
        ///
        /// This id belongs to the entity that spawned the proxy
        u32,
        /// bytes sent upstream,
        ///
        /// *note* usually udp sockets need to implement their own protocols,
        /// so this value can be used to figure out
        usize,
    ),
    Default,
}

/// Network task component, for managing network-related activities
///
#[derive(Component, Default)]
#[storage(DenseVecStorage)]
pub struct NetworkTask(
    /// Task join handle, that returns the created event
    Option<JoinHandle<NetworkEvent>>,
);

impl NetworkTask {
    /// Returns true if the underlying task is_finished
    ///
    pub fn is_ready(&self) -> bool {
        self.0
            .as_ref()
            .and_then(|t| Some(t.is_finished()))
            .unwrap_or_default()
    }

    /// Handles the underlying network task, by awaiting and returning the output
    ///
    pub async fn handle(&mut self) -> Option<NetworkEvent> {
        if let Some(task) = self.0.take() {
            match task.await {
                Ok(net_event) => Some(net_event),
                Err(err) => {
                    event!(Level::ERROR, "error handling task, {err}");
                    None
                }
            }
        } else {
            None
        }
    }
}

impl Extension for NetworkRuntime {
    fn configure_app_world(world: &mut specs::World) {
        world.register::<Event>();
        world.register::<ThunkContext>();
        world.register::<CancelThunk>();
        world.register::<ErrorContext>();
        world.register::<Proxy>();
        world.register::<BlockAddress>();
        world.register::<NetworkTask>();
    }

    fn configure_app_systems(dispatcher: &mut specs::DispatcherBuilder) {
        dispatcher.add(NetworkRuntime::default(), "network_runtime", &[]);

        dispatcher.add(ProxyRuntime::default(), "proxy_runtime", &["event_runtime"]);
    }
}

impl<'a> System<'a> for NetworkRuntime {
    type SystemData = (
        Entities<'a>,
        Read<'a, tokio::runtime::Runtime, EventRuntime>,
        ReadStorage<'a, ThunkContext>,
        WriteStorage<'a, NetworkTask>,
        WriteStorage<'a, Event>,
    );

    fn run(
        &mut self,
        (entities, tokio_runtime, contexts, mut network_tasks, mut events): Self::SystemData,
    ) {
        for (entity, task) in (&entities, &mut network_tasks).join() {
            if task.is_ready() {
                tokio_runtime.block_on(async {
                    if let Some(next) = task.handle().await {
                        match next {
                            NetworkEvent::Proxied(upstream_id, sent) => {
                                let upstream = entities.entity(upstream_id);
                                if let (Some(upstream_event), Some(upstream_context)) =
                                    (events.get_mut(upstream), contexts.get(upstream))
                                {
                                    event!(
                                        Level::TRACE,
                                        "proxied message\n{sent} bytes\n{} -> {upstream_id}\n{}",
                                        entity.id(),
                                        upstream_context.block().name()
                                    );
                                    let mut upstream_context = upstream_context.clone();
                                    upstream_context
                                        .state_mut()
                                        .with_int("received", sent as i32);
                                    upstream_event.activate();
                                }
                            }
                            _ => {}
                        }
                    }
                });
            }
        }
    }
}

#[test]
fn test_network_systems() {
    use specs::DispatcherBuilder;
    use specs::World;
    let mut test_world = World::new();
    let test_world = &mut test_world;
    let mut test_dispatcher = DispatcherBuilder::new();
    let tokio_runtime = tokio::runtime::Runtime::new().unwrap();
    EventRuntime::configure_app_world(test_world);
    EventRuntime::configure_app_systems(&mut test_dispatcher);
    NetworkRuntime::configure_app_world(test_world);
    NetworkRuntime::configure_app_systems(&mut test_dispatcher);

    let mut test_dispatcher = test_dispatcher.build();
    test_dispatcher.setup(test_world);

    let test_entity = test_world.entities().create();
    test_world.maintain();
    test_dispatcher.dispatch(&test_world);

    // Test inserting a task that has completed, gets handled
    test_world
        .write_component()
        .insert(
            test_entity,
            NetworkTask(Some(tokio_runtime.spawn(async {
                eprintln!("called");
                NetworkEvent::Proxied(0, 0)
            }))),
        )
        .ok();
    test_world.maintain();
    test_dispatcher.dispatch(&test_world);
    test_world.maintain();

    assert!(test_world
        .read_component::<NetworkTask>()
        .get(test_entity)
        .and_then(|t| t.0.as_ref())
        .is_none());
}
