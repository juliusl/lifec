use std::{net::SocketAddr, sync::Arc};

use serde::{Serialize, Deserialize};
use specs::{Component, System, WriteStorage, Entities, Join, Read};
use specs::storage::DenseVecStorage;
use tokio::net::UdpSocket;
use tracing::{event, Level};

use crate::AttributeIndex;
use crate::plugins::{ThunkContext,  EventRuntime};

use super::{NetworkEvent, BlockAddress};

/// Component for running proxy network systems
/// 
#[derive(Component, Default, Clone)]
#[storage(DenseVecStorage)]
pub struct Proxy(
    /// Clone of the upstream context this property is hosting
    Option<ThunkContext>,
    /// A connected block address, pointing to the upstream receiver
    Option<BlockAddress>
);

/// Format of the message sent upstream when a proxy receives a message
/// 
#[derive(Serialize, Deserialize)]
pub struct ProxiedMessage {
    /// Source of the message being sent
    src: SocketAddr,
    /// The data that was sent
    data: Vec<u8>,
}

impl Proxy {
    /// Returns the underlying udp socket
    /// 
    pub fn udp_socket(&self) -> Option<Arc<UdpSocket>> {
        self.0.clone().and_then(|tc| tc.socket().clone())
    }

    /// Handles receibing and proxying the next message to the upstream socket
    /// 
    /// Returns the bytes sent as well as the entity_id of the upstream entity
    /// 
    pub async fn proxy_next_message(&self, buffer: &mut [u8]) -> Option<NetworkEvent> {
        if let Some(NetworkEvent::Received(read, src)) = self.receive(buffer).await {
            let data = buffer[..read].to_vec();
            let proxied_message = ProxiedMessage {
                src,
                data
            };
    
            let proxied_message = serde_json::ser::to_vec(
                &proxied_message
            ).ok().unwrap_or_default();

            if let Some(NetworkEvent::Proxied(upstream_entity, sent)) = self.send_upstream(&proxied_message.to_vec()).await {
                if sent != proxied_message.len() {
                    todo!("the entire message wasn't pushed")
                }
    
                Some(NetworkEvent::Proxied(upstream_entity, sent))
            } else {
                None
            }
        } else {
            None 
        }
    }

    /// Receive the next message for this proxy,
    /// 
    /// Returns the the number of bytes read, and the src address
    /// 
    pub async fn receive(&self, received: &mut [u8]) -> Option<NetworkEvent> {
        if let Some(socket) = self.udp_socket() {
            socket.recv_from(received).await.ok().and_then(|(s, addr)| Some(NetworkEvent::Received(s, addr)))
        } else {
            None 
        }
    }

    /// Sends a message to the upstream address,
    /// 
    /// Returns the upstream entity id, and len of message sent upstream
    /// 
    pub async fn send_upstream(&self, message: &[u8]) -> Option<NetworkEvent> {
        if let Proxy(Some(_), Some(address)) = self {
            if let (
                Some((upstream_entity, upstream_address)),
                Some(socket)
             ) = ( address.connected() , self.udp_socket() ){
                if let Some(sent) = socket.send_to(message, upstream_address).await.ok() {
                    Some( NetworkEvent::Proxied(upstream_entity, sent) )
                } else {
                    None
                }
            } else {
                None 
            }
        } else {
            None 
        }
    }
}

impl From<(ThunkContext, BlockAddress)> for Proxy {
    fn from((tc, block_addr): (ThunkContext, BlockAddress)) -> Self {
        Self(Some(tc), Some(block_addr))
    }
}

#[test]
fn test_socket_proxies() {
    use specs::Builder;
    use crate::plugins::BlockAddress; 
    use crate::plugins::NetworkEvent;
    use specs::World; 
    use specs::WorldExt;

    let mut test_world = World::new();
    test_world.register::<ThunkContext>();
    test_world.register::<BlockAddress>();
    test_world.register::<Proxy>();

    let runtime = tokio::runtime::Runtime::new().unwrap();

    let mut a = ThunkContext::default();
    let entity_a = test_world
        .create_entity()
        .with(a.clone())
        .build();
    //a.state() .set_parent_entity(entity_a);

    let mut b = ThunkContext::default();
    let entity_b = test_world
        .create_entity()
        .with(b.clone())
        .build();
    //b.state().set_parent_entity(entity_b);
    test_world.maintain();

    a = a.enable_async(entity_a, runtime.handle().clone());
    b = b.enable_async(entity_b, runtime.handle().clone());

    test_world.write_component().insert(entity_a, a.clone()).ok();
    test_world.write_component().insert(entity_b, b.clone()).ok();
    test_world.maintain();

    // Test some basic expectations about sockets in general
    runtime.block_on(async {
        if let (Some(sock_a), Some(sock_b)) = (a.enable_socket().await, b.enable_socket().await) {
            let sock_a_addr = sock_a.local_addr().ok().unwrap();
            let sock_b_addr = sock_b.local_addr().ok().unwrap();
            
            let sent = sock_a.send_to(b"hello world", sock_b_addr).await.expect("sent");
            let mut received = [0; 1024];
            let received = &mut received;

            let (read, addr) = sock_b.recv_from(received).await.expect("read");
            eprintln!("{:?}, {:?}, {:?}", read, addr, String::from_utf8(received[..read].to_vec()).ok().and_then(|r| Some(r.trim().to_string())));
            assert_eq!(read, sent);
            assert_eq!(addr, sock_a_addr);

            sock_b.send_to(&received[..read], addr).await.ok();

            let (read, addr) = sock_a.recv_from(received).await.expect("received");
            assert_eq!(read, sent);
            assert_eq!(addr, sock_b_addr);

            eprintln!("{:?}, {:?}, {:?}", read, addr, String::from_utf8(received[..read].to_vec()));

        } else {
            assert!(false, "failed");
        }
    });
    test_world.write_component().insert(entity_a, a.clone()).ok();
    test_world.write_component().insert(entity_b, b.clone()).ok();
    test_world.maintain();

    // Test reuse, context-switching works
    runtime.block_on(async {
        if let (Some(sock_a), Some(sock_b)) = (a.socket(), b.socket()) {
            let sock_a_addr = sock_a.local_addr().ok().unwrap();
            let sock_b_addr = sock_b.local_addr().ok().unwrap();
            
            let sent = sock_a.send_to(b"hello world", sock_b_addr).await.expect("sent");
            let mut received = [0; 1024];
            let received = &mut received;

            let (read, addr) = sock_b.recv_from(received).await.expect("read");
            eprintln!("{:?}, {:?}, {:?}", read, addr, String::from_utf8(received[..read].to_vec()).ok().and_then(|r| Some(r.trim().to_string())));
            assert_eq!(read, sent);
            assert_eq!(addr, sock_a_addr);

            sock_b.send_to(&received[..read], addr).await.ok();

            let (read, addr) = sock_a.recv_from(received).await.expect("received");
            assert_eq!(read, sent);
            assert_eq!(addr, sock_b_addr);

            eprintln!("{:?}, {:?}, {:?}", read, addr, String::from_utf8(received[..read].to_vec()));

        } else {
            assert!(false, "failed");
        }
    });

    // Test proxy end-to-end
    runtime.block_on(async {
        let block_a = a.to_block_address().expect("should exist");
        let block_b = b.to_block_address().expect("should exist");

        let proxy_a = block_a.create_proxy(&test_world).await.expect("created");
        let proxy_b = block_b.create_proxy(&test_world).await.expect("created");
        test_world.maintain();
    
        let proxy_a_entity = test_world.entities().entity(proxy_a.entity());
        let proxy_b_entity = test_world.entities().entity(proxy_b.entity());

        let proxy_a_impl = test_world.read_component::<Proxy>().get(proxy_a_entity).expect("retrieved").clone();
        let proxy_b_impl = test_world.read_component::<Proxy>().get(proxy_b_entity).expect("retrieved").clone();

        if let (Some(sock_a), Some(sock_b)) = (proxy_a_impl.udp_socket(), proxy_b_impl.udp_socket()) {
            let sock_a_addr = sock_a.local_addr().ok().unwrap();
            let sock_b_addr = sock_b.local_addr().ok().unwrap();
            
            let proxy_b_rcv = runtime.spawn(async move {
                let mut received = [0; 1024];
                let received = &mut received;

                // Checks that `b` can receive the message
                let upstream_b = b.socket().unwrap();
                let (read, addr) = upstream_b.recv_from(received).await.expect("read");
                
                 assert_eq!(addr, sock_b_addr);
        
                upstream_b.send_to(&received[..read], sock_a_addr).await.ok();
            });
        
            let proxy_a_recv = runtime.spawn(async move { 
                let mut received = [0; 1024];
                let received = &mut received;

                // Checks that `a` can receive the message
                let upstream_a = a.socket().unwrap();
                let (_, addr) = upstream_a.recv_from(received).await.expect("read");

                assert_eq!(addr, sock_a_addr, "received a message from the proxy");

            });

            // Write to b
            let _ = sock_a.send_to(b"hello world", sock_b_addr).await.expect("sent");
            let mut received = [0; 1024];
            let received = &mut received;

            // Receives the message for b from b's proxy
            if let Some(NetworkEvent::Proxied(upstream, _)) = proxy_b_impl.proxy_next_message(received).await {
                eprintln!("{} -> upstream {}", proxy_b_entity.id(), upstream);
                proxy_b_rcv.await.ok();
            } else {
                assert!(false)
            }

            // Receives the message for a from a's proxy
            if let Some(NetworkEvent::Proxied(upstream, _)) = proxy_a_impl.proxy_next_message(received).await {
                eprintln!("{} -> upstream {}", proxy_a_entity.id(), upstream);
                proxy_a_recv.await.ok();
            } else {
                assert!(false)
            }
        } else {
            assert!(false, "failed");
        }
    })
}

/// Proxy runtime looks for thunk contexts that have a bool attribute `enable_proxy_socket` enabled,
/// and do not alreay have a Proxy component installed.
/// 
/// If enabled, this system will create a socket for the context if it doesn't already exist, and then create
/// a proxy_entity to host the new proxy component.
/// 
#[derive(Default)]
pub struct ProxyRuntime;

impl<'a> System<'a> for ProxyRuntime {
    type SystemData = (
        Entities<'a>,
        Read<'a, tokio::runtime::Runtime, EventRuntime>,
        WriteStorage<'a, ThunkContext>,
        WriteStorage<'a, BlockAddress>,
        WriteStorage<'a, Proxy>,
    );

    fn run(&mut self, (entities, tokio_runtime, mut contexts, mut block_addresses, mut proxies): Self::SystemData) {
        for (entity, context) in (&entities, &mut contexts).join() {
            
            // Enables a proxy for the socket, if a socket doesn't already exist, one is created
            if !proxies.contains(entity) && context.is_enabled("enable_proxy_socket") {
                if context.socket_address().is_none() {
                    tokio_runtime.block_on(async {
                        context.enable_socket().await;
                        if let Some(block_address) = context.to_block_address() {
                            match block_addresses.insert(entity, block_address) {
                                Ok(_) => event!(Level::TRACE, "inserted block_address for {:?}", entity),
                                Err(err) => event!(Level::ERROR, "could not insert block_address component {err}"),
                            }
                        }
                    });
                }

                if let Some(block_address) = context.to_block_address() {
                    tokio_runtime.block_on(async {
                        let proxy_entity = entities.create();
                        let mut proxy_context = context.clone();
                        //proxy_context.as_mut().set_parent_entity(proxy_entity);
                        proxy_context.enable_socket().await;
                        if let Some(mut proxy_address) = proxy_context.to_block_address() {
                            proxy_address.enable_proxy_mode();
                            if let Some((from, _)) = proxy_address.open().connect(&block_address.open()) {
                                let proxy = Proxy::from((proxy_context, from.clone()));
                                match proxies.insert(proxy_entity, proxy) {
                                    Ok(_) => event!(Level::TRACE, "inserted proxy for {:?}", proxy_entity),
                                    Err(err) => event!(Level::ERROR, "could not insert proxy component {err}"),
                                }

                                match block_addresses.insert(proxy_entity, from) {
                                    Ok(_) => event!(Level::TRACE, "inserted block_address for {:?}", proxy_entity),
                                    Err(err) => event!(Level::ERROR, "could not insert block_address component {err}"),
                                }
                            }
                        }
                    });
                }
            }
        }
    }
}

#[test]
fn test_proxy_runtime() {
    use specs::World;
    use specs::WorldExt;
    use specs::DispatcherBuilder;
    use atlier::system::Extension;
    use crate::plugins::network::NetworkRuntime;
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
    test_dispatcher.dispatch(&test_world);
    
    let test_entity_a = test_world.entities().create();
    let mut tc = ThunkContext::default();
    tc.state().add_bool_attr("enable_proxy_socket", true);
    // tc.state().set_parent_entity(test_entity_a);
    test_world.write_component().insert(test_entity_a, tc.clone()).ok();

    let test_entity_b = test_world.entities().create();
    let mut tc = ThunkContext::default();
    tc.state().add_bool_attr("enable_proxy_socket", true);
    // tc.as_mut().set_parent_entity(test_entity_b);
    test_world.write_component().insert(test_entity_b, tc.clone()).ok();
    
    test_world.maintain();
    test_dispatcher.dispatch(&test_world);

    tokio_runtime.block_on(async {
        let a = test_world.read_component::<ThunkContext>().get(test_entity_a).expect("retrieved").clone();
        let b = test_world.read_component::<ThunkContext>().get(test_entity_b).expect("retrieved").clone();

        let addresses = test_world.read_component::<BlockAddress>();
        let addresses = addresses.join().filter(|a| a.is_proxy_address()).collect::<Vec<_>>();

        let proxy_a_entity_id = addresses.get(0).unwrap().entity();
        let proxy_b_entity_id = addresses.get(1).unwrap().entity();

        let proxy_a_entity = test_world.entities().entity(proxy_a_entity_id);
        let proxy_b_entity = test_world.entities().entity(proxy_b_entity_id);
        let proxies = test_world.read_component::<Proxy>();

        let proxy_a_impl = &proxies.get(proxy_a_entity).unwrap();
        let proxy_b_impl = &proxies.get(proxy_b_entity).unwrap();
    
        if let (Some(sock_a), Some(sock_b)) = (proxy_a_impl.udp_socket(), proxy_b_impl.udp_socket()) {
            let sock_a_addr = sock_a.local_addr().ok().unwrap();
            let sock_b_addr = sock_b.local_addr().ok().unwrap();
            
            let proxy_b_rcv = tokio_runtime.spawn(async move {
                let mut received = [0; 1024];
                let received = &mut received;
    
                // Checks that `b` can receive the message
                let upstream_b = b.socket().unwrap();
                let (read, addr) = upstream_b.recv_from(received).await.expect("read");
                
                 assert_eq!(addr, sock_b_addr);
        
                upstream_b.send_to(&received[..read], sock_a_addr).await.ok();
            });
        
            let proxy_a_recv = tokio_runtime.spawn(async move { 
                let mut received = [0; 1024];
                let received = &mut received;
    
                // Checks that `a` can receive the message
                let upstream_a = a.socket().unwrap();
                let (_, addr) = upstream_a.recv_from(received).await.expect("read");
    
                assert_eq!(addr, sock_a_addr, "received a message from the proxy");
    
            });
    
            // Write to b
            let _ = sock_a.send_to(b"hello world", sock_b_addr).await.expect("sent");
            let mut received = [0; 1024];
            let received = &mut received;
    
            // Receives the message for b from b's proxy
            if let Some(NetworkEvent::Proxied(upstream, _)) = proxy_b_impl.proxy_next_message(received).await {
                eprintln!("{} -> upstream {}", proxy_b_entity_id, upstream);
                proxy_b_rcv.await.ok();
            } else {
                assert!(false)
            }
    
            // Receives the message for a from a's proxy
            if let Some(NetworkEvent::Proxied(upstream, _)) = proxy_a_impl.proxy_next_message(received).await {
                eprintln!("{} -> upstream {}", proxy_a_entity_id, upstream);
                proxy_a_recv.await.ok();
            } else {
                assert!(false)
            }
        } else {
            assert!(false, "failed");
        }
    });
}