use bytemuck::cast;
use specs::Component;
use specs::DenseVecStorage;
use specs::World;
use specs::WorldExt;
use std::hash::Hash;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::net::SocketAddr;
use tracing::event;
use tracing::Level;

use crate::plugins::Proxy;
use crate::plugins::ThunkContext;
use crate::AttributeGraph;

/// Address component, that compacts a connection between two blocks
///
/// Represents a uniform matrix of 64 x 64 u8 integers, using u64 integers,
/// that can be used in a XOR-Linked list, connecting
/// blocks of data defined by runmd. Provides methods to use
/// elements within the matrix, and the state of the matrix for fundamental
/// indexing values.
///
/// All apis are designed to be immutable, therefore if an api changes state, it will return a new block address.
///
/// An example usage is w/ the thunk_context.
///
/// If a thunk_context has enabled a socket, it can be converted into a block address.
///
/// If two thunk_contexts enable a socket, both now have a block_address. This address can then be used
/// to connect these two contexts, and in order for either side to communicate, all that is required is the connected
/// version of the block address. This is useful for a hop-by-hop communication, so that blocks can leave messages w/o
/// needing to maintain and poll a connection. A dispatcher can sit between and monitor these connections, in a stateless manner
/// and route messasges to the waiting sockets, w/o the contexts needing to maintain the overhead of polling a connection.
///
#[derive(Debug, Component, Default, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[storage(DenseVecStorage)]
pub struct BlockAddress {
    /// The hash code is the hash_code of the runmd block that initated this address to be created
    ///
    hash_code: u64,
    /// An entity id is typically a u32 int (provided by specs)
    /// The format of the entity block is
    /// a x a ^ b, where a, b represents a u32 int
    /// If a is closed for connections, the layout is
    /// a x a ^ 0
    /// If a is open for connections, the layout is
    /// a x a ^ 1
    /// If a is connected to another entity, then the layout is
    /// a x a ^ b
    ///
    entity_block: u64,
    /// An IP v4/v6 address is at most 3 u64 integers
    /// Similar to the entity block above, the ultimate layout is
    /// A
    /// x
    /// B, where A is ip_block(a - c) and B is ip_block(d - f)
    /// If A is closed for connections, the layout is
    /// A
    /// x
    /// A ^ 0
    /// If A is open for connections, the layout is
    /// A
    /// x
    /// A ^ 1
    /// If B is connected, the layout is
    /// A
    /// x
    /// A ^ B
    ///
    ip_block_a: u64,
    ip_block_b: u64,
    /// port
    ip_block_c: u64,
    /// The following integers are used to store the connection state for this socket addr
    ip_block_d: u64,
    ip_block_e: u64,
    ip_block_f: u64,
}

impl Into<[u8; 64]> for BlockAddress {
    fn into(self) -> [u8; 64] {
        let mut node = [0; 64];
        let hash_code = cast::<u64, [u8; 8]>(self.hash_code);
        let entity_block = cast::<u64, [u8; 8]>(self.entity_block);
        let ip_block_a = cast::<u64, [u8; 8]>(self.ip_block_a);
        let ip_block_b = cast::<u64, [u8; 8]>(self.ip_block_b);
        let ip_block_c = cast::<u64, [u8; 8]>(self.ip_block_c);
        let ip_block_d = cast::<u64, [u8; 8]>(self.ip_block_d);
        let ip_block_e = cast::<u64, [u8; 8]>(self.ip_block_e);
        let ip_block_f = cast::<u64, [u8; 8]>(self.ip_block_f);
        node[0] = hash_code[0];
        node[1] = hash_code[1];
        node[2] = hash_code[2];
        node[3] = hash_code[3];
        node[4] = hash_code[4];
        node[5] = hash_code[5];
        node[6] = hash_code[6];
        node[7] = hash_code[7];
        node[8] = entity_block[0];
        node[9] = entity_block[1];
        node[10] = entity_block[2];
        node[11] = entity_block[3];
        node[12] = entity_block[4];
        node[13] = entity_block[5];
        node[14] = entity_block[6];
        node[15] = entity_block[7];
        node[16] = ip_block_a[0];
        node[17] = ip_block_a[1];
        node[18] = ip_block_a[2];
        node[19] = ip_block_a[3];
        node[20] = ip_block_a[4];
        node[21] = ip_block_a[5];
        node[22] = ip_block_a[6];
        node[23] = ip_block_a[7];
        node[24] = ip_block_b[0];
        node[25] = ip_block_b[1];
        node[26] = ip_block_b[2];
        node[27] = ip_block_b[3];
        node[28] = ip_block_b[4];
        node[29] = ip_block_b[5];
        node[30] = ip_block_b[6];
        node[31] = ip_block_b[7];
        node[32] = ip_block_c[0];
        node[33] = ip_block_c[1];
        node[34] = ip_block_c[2];
        node[35] = ip_block_c[3];
        node[36] = ip_block_c[4];
        node[37] = ip_block_c[5];
        node[38] = ip_block_c[6];
        node[39] = ip_block_c[7];
        node[40] = ip_block_d[0];
        node[41] = ip_block_d[1];
        node[42] = ip_block_d[2];
        node[43] = ip_block_d[3];
        node[44] = ip_block_d[4];
        node[45] = ip_block_d[5];
        node[46] = ip_block_d[6];
        node[47] = ip_block_d[7];
        node[48] = ip_block_e[0];
        node[49] = ip_block_e[1];
        node[50] = ip_block_e[2];
        node[51] = ip_block_e[3];
        node[52] = ip_block_e[4];
        node[53] = ip_block_e[5];
        node[54] = ip_block_e[6];
        node[55] = ip_block_e[7];
        node[56] = ip_block_f[0];
        node[57] = ip_block_f[1];
        node[58] = ip_block_f[2];
        node[59] = ip_block_f[3];
        node[60] = ip_block_f[4];
        node[61] = ip_block_f[5];
        node[62] = ip_block_f[6];
        node[63] = ip_block_f[7];
        node
    }
}

impl BlockAddress {
    pub fn enable_proxy_mode(&mut self) {
        self.hash_code = 0;
    }

    /// Creates a proxy for the current address
    pub async fn create_proxy(&self, world: &World) -> Option<Self> {
        let dest = self.clone().open();

        let entity = dest.entity();
        let entity = world.entities().entity(entity);

        match world.read_component::<ThunkContext>().get(entity) {
            // The original hash code must match the hash code this address was created at
            Some(ref tc) if tc.as_ref().hash_code() == self.hash_code => {
                let proxy_entity = world.entities().create();
                let mut hosting = tc.to_owned().clone();
                hosting.as_mut().set_parent_entity(proxy_entity);

                if let Some(_) = hosting.enable_socket().await {
                    if let Some(mut proxy_address) = hosting.to_block_address() {
                        // this signifies that it is in proxy mode, so the state is always considered transient
                        proxy_address.hash_code = 0;
                        let proxy_address = proxy_address.open();
                        if let Some((from, _)) = proxy_address.connect(&dest) {
                            match world.write_component().insert(proxy_entity, from.clone()) {
                                Ok(_) => {
                                    event!(
                                        Level::DEBUG,
                                        "added block address component for proxy {:?}",
                                        proxy_entity
                                    );
                                    match world
                                        .write_component()
                                        .insert(proxy_entity, Proxy::from((hosting, from.clone())))
                                    {
                                        Ok(_) => {
                                            event!(
                                                Level::DEBUG,
                                                "added proxy component for {:?}",
                                                proxy_entity
                                            );

                                            return Some(from);
                                        }
                                        Err(err) => event!(
                                            Level::ERROR,
                                            "could not add proxy component, {err}"
                                        ),
                                    }
                                }
                                Err(err) => event!(
                                    Level::ERROR,
                                    "could not add block address for proxy, {err}"
                                ),
                            }
                        }
                    }
                
                }
            
            }
            _ => {

            }
        }

        None
    }
}

impl BlockAddress {
    /// Returns a new block address for an attribute graph, uses the current entity set in the attribute graph
    /// as the entity for this address
    ///
    pub fn new(graph: impl AsRef<AttributeGraph>) -> BlockAddress {
        let hash_code = graph.as_ref().hash_code();

        let mut new_address = BlockAddress {
            hash_code,
            entity_block: 0,
            ip_block_a: 0,
            ip_block_b: 0,
            ip_block_c: 0,
            ip_block_d: 0,
            ip_block_e: 0,
            ip_block_f: 0,
        };
        new_address.set_entity_block([graph.as_ref().entity(), 0]);
        new_address
    }

    /// Returns a new address with the socket address set
    ///
    /// Caveat: Returns the address in a "closed" state, meaning trying to connect w/ this address will
    /// yield nothing
    ///
    pub fn with_socket_addr(&self, addr: SocketAddr) -> Self {
        let mut next = self.clone();
        next.set_socket_addr(addr);
        next
    }

    /// Get's the socket addr from the block address
    ///
    pub fn socket_addr(&self) -> Option<SocketAddr> {
        if self.is_unspecified_ip() {
            return Some(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                self.port(),
            ));
        }

        if self.ip_block_b == 0 {
            // v4
            Some(SocketAddr::new(IpAddr::V4(self.ip_addr_v4()), self.port()))
        } else {
            // v6
            Some(SocketAddr::new(IpAddr::V6(self.ip_addr_v6()), self.port()))
        }
    }

    /// Returns the owning entity
    ///
    pub fn entity(&self) -> u32 {
        let [entity, ..] = self.entity_block();
        entity
    }

    /// Returns an open version of this address
    ///
    pub fn open(&self) -> Self {
        let mut opened = self.clone();
        opened.hash_code = self.hash_code;
        let [a, ..] = cast::<u64, [u32; 2]>(self.entity_block);
        opened.set_entity_block([a, a]);

        opened.ip_block_a = self.ip_block_a;
        opened.ip_block_b = self.ip_block_b;
        opened.ip_block_c = self.ip_block_c;
        opened.ip_block_d = self.ip_block_a;
        opened.ip_block_e = self.ip_block_b;
        opened.ip_block_f = self.ip_block_c;
        opened
    }

    /// Returns true if the socket is ready to create a connection with another addr
    ///
    pub fn is_opened(&self) -> bool {
        let [a, b] = cast::<u64, [u32; 2]>(self.entity_block);
        a == b
            && self.ip_block_a == self.ip_block_d
            && self.ip_block_b == self.ip_block_e
            && self.ip_block_c == self.ip_block_f
    }

    /// Returns the block address for each side of the connection between two open block addresses
    ///
    /// If a connection cannot be made between the two addresses, returns nothing
    ///
    pub fn connect(&self, other: &BlockAddress) -> Option<(Self, Self)> {
        if self.is_opened() && other.is_opened() {
            let mut left = self.clone();
            let mut right = other.clone();

            // link entities
            let [a, b] = cast::<u64, [u32; 2]>(self.entity_block);
            let [c, d] = cast::<u64, [u32; 2]>(other.entity_block);
            left.set_entity_block([a, b ^ c]);
            right.set_entity_block([c, d ^ a]);

            match (self.socket_addr(), other.socket_addr()) {
                (Some(SocketAddr::V4(v4_addr)), Some(SocketAddr::V4(other_v4_addr))) => {
                    // link ip addresses
                    let [a0, a1, a2, a3] = v4_addr.ip().octets();
                    let [b0, b1, b2, b3] = other_v4_addr.ip().octets();
                    let [c0, c1, c2, c3] = [a0 ^ b0, a1 ^ b1, a2 ^ b2, a3 ^ b3];
                    let mut connected_ips = cast::<[u8; 8], u64>([c3, c2, c0, c1, 0, 0, 0, 0]);

                    // link port addresses
                    let port_a = v4_addr.port();
                    let port_b = other_v4_addr.port();
                    let connected_ports = cast::<[u16; 4], u64>([port_a ^ port_b, 0, 0, 0]);

                    if connected_ips == 0 { 
                        // this means the ip addresses are equal
                        let [ d, c ,b ,a ] = v4_addr.ip().octets();
                        connected_ips = cast::<[u8; 8], u64>([a, b, c, d, 0, 0, 0, 0]);
                    }

                    // set connections on return values
                    left.ip_block_d = connected_ips;
                    right.ip_block_d = connected_ips;
                    left.ip_block_e = 0;
                    right.ip_block_e = 0;
                    left.ip_block_f = connected_ports;
                    right.ip_block_f = connected_ports;
                }
                (Some(SocketAddr::V4(_)), Some(SocketAddr::V6(_))) => {
                    panic!("Ip address must be the same format");

                    // TODO: It might be too ambitious to try and use mixed ip formats
                    // so for now commenting this out

                    // let [ a0, a1, a2, a3 ] = v4_addr.ip().octets();
                    // let [ b0, b1, b2, b3, b4, b5, b6, b7 ] = other_v6_addr.ip().segments();

                    // let [b0_a, b0_b] = cast::<u16, [u8; 2]>(b0);
                    // let [b1_a, b1_b] = cast::<u16, [u8; 2]>(b1);
                    // let [b2_a, b2_b] = cast::<u16, [u8; 2]>(b2);
                    // let [b3_a, b3_b] = cast::<u16, [u8; 2]>(b3);

                    // let [c0, c1, c2, c3] = [
                    //     a0 ^ b0_a,
                    //     a1 ^ b0_b,
                    //     a2 ^ b1_a,
                    //     a3 ^ b1_b
                    // ];
                    // let connected_ips_a = cast::<[u8; 8], u64>([
                    //     c0, c1, c2, c3, b2_a, b2_b, b3_a, b3_b
                    // ]);
                    // let connected_ips_b = cast::<[u16; 4], u64>([b4, b5, b6, b7]);

                    // left.ip_block_d = connected_ips_a;
                    // right.ip_block_d = connected_ips_a;
                    // left.ip_block_e = connected_ips_b;
                    // right.ip_block_e = connected_ips_b;

                    // let port_a = v4_addr.port();
                    // let port_b = other_v6_addr.port();
                    // let connected_ports = cast::<[u16; 4], u64>([port_a ^ port_b, 0, 0, 0]);
                    // left.ip_block_f = connected_ports;
                    // right.ip_block_f = connected_ports;
                }
                (Some(SocketAddr::V6(_)), Some(SocketAddr::V4(_))) => {
                    panic!("Ip address must be the same format");

                    // TODO: It might be too ambitious to try and use mixed ip formats
                    // so for now commenting this out
                    // let [ a0, a1, a2, a3, a4, a5, a6, a7 ] = v6_addr.ip().segments();
                    // let [ b0, b1, b2, b3 ] = other_v4_addr.ip().octets();
                    // let [a0_a, a0_b] = cast::<u16, [u8; 2]>(a0);
                    // let [a1_a, a1_b] = cast::<u16, [u8; 2]>(a1);
                    // let [a2_a, a2_b] = cast::<u16, [u8; 2]>(a2);
                    // let [a3_a, a3_b] = cast::<u16, [u8; 2]>(a3);

                    // let [c0, c1, c2, c3] = [
                    //     b0 ^ a0_a,
                    //     b1 ^ a0_b,
                    //     b2 ^ a1_a,
                    //     b3 ^ a1_b
                    // ];
                    // let connected_ips_a = cast::<[u8; 8], u64>([
                    //     c0, c1, c2, c3, a2_a, a2_b, a3_a, a3_b
                    // ]);
                    // let connected_ips_b = cast::<[u16; 4], u64>([a4, a5, a6, a7]);

                    // left.ip_block_d = connected_ips_a;
                    // right.ip_block_d = connected_ips_a;
                    // left.ip_block_e = connected_ips_b;
                    // right.ip_block_e = connected_ips_b;

                    // let port_a = v6_addr.port();
                    // let port_b = other_v4_addr.port();
                    // let connected_ports = cast::<[u16; 4], u64>([port_a ^ port_b, 0, 0, 0]);
                    // left.ip_block_f = connected_ports;
                    // right.ip_block_f = connected_ports;
                }
                (Some(SocketAddr::V6(v6_addr)), Some(SocketAddr::V6(other_v6_addr))) => {
                    let [a0, a1, a2, a3, a4, a5, a6, a7] = v6_addr.ip().segments();
                    let [b0, b1, b2, b3, b4, b5, b6, b7] = other_v6_addr.ip().segments();

                    let [c0, c1, c2, c3, c4, c5, c6, c7] = [
                        a0 ^ b0,
                        a1 ^ b1,
                        a2 ^ b2,
                        a3 ^ b3,
                        a4 ^ b4,
                        a5 ^ b5,
                        a6 ^ b6,
                        a7 ^ b7,
                    ];

                    let mut connected_ip_a = cast::<[u16; 4], u64>([c0, c1, c2, c3]);
                    let mut connected_ip_b = cast::<[u16; 4], u64>([c4, c5, c6, c7]);

                    if connected_ip_a == 0 && connected_ip_b == 0  { // this means the ips are the same
                        connected_ip_a = cast::<[u16; 4], u64>([a3, a2, a1, a0]);
                        connected_ip_b = cast::<[u16; 4], u64>([a7, a6, a5, a4]);
                    }

                    left.ip_block_d = connected_ip_a;
                    right.ip_block_d = connected_ip_a;
                    left.ip_block_e = connected_ip_b;
                    right.ip_block_e = connected_ip_b;

                    // link port addresses
                    let port_a = v6_addr.port();
                    let port_b = other_v6_addr.port();
                    let connected_ports = cast::<[u16; 4], u64>([port_a ^ port_b, 0, 0, 0]);
                    left.ip_block_f = connected_ports;
                    right.ip_block_f = connected_ports;
                }
                _ => {}
            }

            Some((left, right))
        } else {
            None
        }
    }

    /// Returns the connected entity, and it's socket addr if it exists
    ///
    pub fn connected(&self) -> Option<(u32, SocketAddr)> {
        // If this is open, then returns None, as it is not connected
        if self.is_opened() {
            event!(Level::TRACE, "not connected");
            None
        } else {
            if let (e, Some(a)) = (self.connected_entity(), self.connected_address()) {
                Some((e, a))
            } else {
                event!(Level::WARN, "does not have connected entity and address");
                None
            }
        }
    }

    /// If this block address was created as part of a proxy, the hash_code is set to 0
    ///
    pub fn is_proxy_address(&self) -> bool {
        self.hash_code == 0
    }
}

/// Private functions for assembling and decomposing this block address
///
impl BlockAddress {
    /// If connected, returns the connected entity
    ///  
    pub fn connected_entity(&self) -> u32 {
        let [a, b] = self.entity_block();
        b ^ a
    }

    /// If connected, returns the connected socket address
    ///  
    pub fn connected_address(&self) -> Option<SocketAddr> {
        if self.ip_block_d == 0 && self.ip_block_e == 0 && self.ip_block_f == 0 {
            None
        } else if self.ip_block_a == self.ip_block_d
            && self.ip_block_b == self.ip_block_e
            && self.ip_block_c == self.ip_block_f
        {
            None
        } else if self.is_unspecified_ip() {
            None
        } else {
            if self.is_ipv6() {
                let mut other_ip_a = self.ip_block_a ^ self.ip_block_d;
                let mut other_ip_b = self.ip_block_b ^ self.ip_block_e;
                let other_ip_c = self.ip_block_c ^ self.ip_block_f;

                if other_ip_a == 0 && other_ip_b == 0 { 
                    // this means the ip addresses are equal
                    other_ip_a = self.ip_block_a;
                    other_ip_b = self.ip_block_b;
                }

                let [port, ..] = cast::<u64, [u16; 4]>(other_ip_c);
                let [d, c, b, a] = cast::<u64, [u16; 4]>(other_ip_a);
                let [h, g, f, e] = cast::<u64, [u16; 4]>(other_ip_b);
                Some(SocketAddr::new(
                    IpAddr::V6(Ipv6Addr::new(a, b, c, d, e, f, g, h)),
                    port,
                ))
            } else {
                let mut other_ip_a = self.ip_block_a ^ self.ip_block_d;
                let other_ip_c = self.ip_block_c ^ self.ip_block_f;

                if other_ip_a == 0 { 
                    // this means the ip addresses are equal
                    other_ip_a = self.ip_block_a;
                }

                let [d, c, b, a, ..] = cast::<u64, [u8; 8]>(other_ip_a);
                let [port, ..] = cast::<u64, [u16; 4]>(other_ip_c);
                Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(a, b, c, d)), port))
            }
        }
    }

    /// Sets the ip address for the block from a socket addr
    ///
    fn set_socket_addr(&mut self, addr: SocketAddr) {
        match addr.ip() {
            std::net::IpAddr::V4(ip_v4) => {
                self.set_ip_v4(ip_v4.octets(), addr.port());
            }
            std::net::IpAddr::V6(ip_v6) => {
                self.set_ip_v6(ip_v6.segments(), addr.port());
            }
        }
    }

    /// Returns the values of the entity block
    ///
    fn entity_block(&self) -> [u32; 2] {
        cast::<u64, [u32; 2]>(self.entity_block)
    }

    /// Sets the state of the entity block
    ///
    fn set_entity_block(&mut self, connection: [u32; 2]) {
        self.entity_block = cast::<[u32; 2], u64>(connection);
    }

    /// Sets an ip_v4 address
    ///
    fn set_ip_v4(&mut self, ip: [u8; 4], port: u16) {
        self.ip_block_a = cast::<[u8; 8], u64>([ip[3], ip[2], ip[1], ip[0], 0, 0, 0, 0]);
        self.ip_block_b = 0;
        self.ip_block_c = cast::<[u16; 4], u64>([port, 0, 0, 0]);
    }

    /// Sets an ip_v6 address
    ///
    fn set_ip_v6(&mut self, ip: [u16; 8], port: u16) {
        self.ip_block_a = cast::<[u16; 4], u64>([ip[3], ip[2], ip[1], ip[0]]);

        self.ip_block_b = cast::<[u16; 4], u64>([ip[7], ip[6], ip[5], ip[4]]);

        self.ip_block_c = cast::<[u16; 4], u64>([port, 0, 0, 0]);
    }

    /// Returns the ip_v4 address
    ///
    fn ip_addr_v4(&self) -> Ipv4Addr {
        let [d, c, b, a, ..] = cast::<u64, [u8; 8]>(self.ip_block_a);
        Ipv4Addr::new(a, b, c, d)
    }

    /// Returns the ip_v6 address
    ///
    fn ip_addr_v6(&self) -> Ipv6Addr {
        let [d, c, b, a] = cast::<u64, [u16; 4]>(self.ip_block_a);
        let [h, g, f, e] = cast::<u64, [u16; 4]>(self.ip_block_b);
        Ipv6Addr::new(a, b, c, d, e, f, g, h)
    }

    /// Returns true if the ip address set for this block is an ipv6 address, which uses
    /// at least uses self.ip_block_b,
    ///
    /// Caveat: doesn't include the case that the address is set to host only
    ///
    fn is_ipv6(&self) -> bool {
        self.ip_block_a != 0 && self.ip_block_b != 0 && !self.is_unspecified_ip()
    }

    /// Returns true if the address is set to 0.0.0.0
    ///
    /// Note: an address set at 0.0.0.0 has a special meaning, a server binding to 0.0.0.0 is required
    /// if the server plans on serving traffic for a particular host name
    ///
    fn is_unspecified_ip(&self) -> bool {
        self.ip_block_a == 0 && self.ip_block_b == 0 && self.ip_block_c != 0
    }

    /// Returns the port
    ///
    fn port(&self) -> u16 {
        let [port, ..] = cast::<u64, [u16; 4]>(self.ip_block_c);
        port
    }
}

#[test]
fn test_block_address() {
    let graph = AttributeGraph::from(100);
    let mut addr = BlockAddress::new(&graph);
    let ip_v4 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 50871);
    let ip_v6 = SocketAddr::new(
        IpAddr::V6(Ipv6Addr::new(127, 0, 127, 0, 127, 0, 0, 1)),
        50871,
    );

    assert_eq!(addr.entity(), 100);
    assert_eq!(addr.hash_code, graph.hash_code());

    addr.set_ip_v4([127, 0, 0, 1], 50871);
    assert_eq!(addr.ip_addr_v4(), Ipv4Addr::new(127, 0, 0, 1));
    assert_eq!(addr.port(), 50871);
    assert_eq!(addr.socket_addr(), Some(ip_v4));

    addr.set_ip_v6([127, 0, 127, 0, 127, 0, 0, 1], 50871);
    assert_eq!(
        addr.ip_addr_v6(),
        Ipv6Addr::new(127, 0, 127, 0, 127, 0, 0, 1)
    );
    assert_eq!(addr.port(), 50871);
    assert_eq!(addr.socket_addr(), Some(ip_v6));

    addr.set_socket_addr(ip_v4);
    assert_eq!(addr.ip_addr_v4(), Ipv4Addr::new(127, 0, 0, 1));
    assert_eq!(addr.port(), 50871);
    assert_eq!(addr.socket_addr(), Some(ip_v4));

    addr.set_socket_addr(ip_v6);
    assert_eq!(
        addr.ip_addr_v6(),
        Ipv6Addr::new(127, 0, 127, 0, 127, 0, 0, 1)
    );
    assert_eq!(addr.port(), 50871);
    assert_eq!(addr.socket_addr(), Some(ip_v6));

    let opened = addr.open();
    assert!(opened.is_opened());
}

#[test]
fn test_block_connection() {
    let ip_v4_a = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 50871);
    let ip_v4_b = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)), 58237);
    let ip_v6_a = SocketAddr::new(
        IpAddr::V6(Ipv6Addr::new(127, 0, 127, 0, 127, 0, 0, 1)),
        50212,
    );
    let ip_v6_b = SocketAddr::new(
        IpAddr::V6(Ipv6Addr::new(127, 0, 127, 0, 127, 0, 0, 1)),
        42313,
    );

    let a = AttributeGraph::from(100);
    let b = AttributeGraph::from(1421);
    let addr_a = BlockAddress::new(&a);
    let addr_b = BlockAddress::new(&b);

    let addr_a_ip_v4 = addr_a.with_socket_addr(ip_v4_a).open();
    let addr_b_ip_v4 = addr_b.with_socket_addr(ip_v4_b).open();

    assert_eq!(addr_a_ip_v4.socket_addr(), Some(ip_v4_a));
    assert_eq!(addr_b_ip_v4.socket_addr(), Some(ip_v4_b));

    if let Some((from, to)) = addr_a_ip_v4.connect(&addr_b_ip_v4) {
        if let Some((to_e, to_addr)) = from.connected() {
            assert_eq!(to_e, 1421);
            assert_eq!(to_addr, ip_v4_b);
            assert_eq!(to.hash_code, b.hash_code());
            eprintln!("{:?}, {:?}, {}", to_e, to_addr, to.hash_code);
        }

        if let Some((from_e, from_addr)) = to.connected() {
            assert_eq!(from_e, 100);
            assert_eq!(from.hash_code, a.hash_code());
            assert_eq!(from_addr, ip_v4_a);
            eprintln!("{:?}, {:?}, {}", from_e, from_addr, from.hash_code);
        }
    }

    let addr_a_ip_v6 = addr_a.with_socket_addr(ip_v6_a).open();
    let addr_b_ip_v6 = addr_b.with_socket_addr(ip_v6_b).open();

    assert_eq!(addr_a_ip_v6.socket_addr(), Some(ip_v6_a));
    assert_eq!(addr_b_ip_v6.socket_addr(), Some(ip_v6_b));

    if let Some((from, to)) = addr_a_ip_v6.connect(&addr_b_ip_v6) {
        if let Some((to_e, to_addr)) = from.connected() {
            assert_eq!(to_e, 1421);
            assert_eq!(to_addr, ip_v6_b);
            assert_eq!(to.hash_code, b.hash_code());
            eprintln!("{:?}, {:?}, {}", to_e, to_addr, to.hash_code);
        }

        if let Some((from_e, from_addr)) = to.connected() {
            assert_eq!(from_e, 100);
            assert_eq!(from_addr, ip_v6_a);
            assert_eq!(from.hash_code, a.hash_code());
            eprintln!("{:?}, {:?}, {}", from_e, from_addr, from.hash_code);
        }
    }
}

#[test]
fn test_proxy_mode() {
    use specs::Builder;
    
    let mut test_world = World::new();
    test_world.register::<BlockAddress>();
    test_world.register::<ThunkContext>();
    test_world.register::<Proxy>();

    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let mut main = ThunkContext::default();
        let main_entity = test_world.create_entity().build();
        main.as_mut().set_parent_entity(main_entity);
        if let Some(_) = main.enable_socket().await {
        } else {
            assert!(false, "Could not create main socket");
        }
        test_world
            .write_component()
            .insert(main_entity, main.clone())
            .ok();
        test_world.maintain();

        if let Some(main_block_address) = main.to_block_address() {
            if let Some(proxy_address) = main_block_address.create_proxy(&test_world).await {
                if let Some((dest_entity, dest_address)) = proxy_address.connected() {
                    
                    eprintln!("Proxy:       \t{:#?},\t\t\t{:#?},\t{:#?}", proxy_address.hash_code, proxy_address.entity(), proxy_address.socket_addr().expect("proxy should have an address"));
                    eprintln!("Destination: \t{:#?},\t{:#?},\t{:#?}", main_block_address.hash_code, dest_entity, dest_address);
                }
            } else {
                assert!(false, "Could not enable proxy");
            }
        } else {
            assert!(false, "Could not create block address");
        }
    });
}
