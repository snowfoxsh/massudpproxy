use rlimit::{increase_nofile_limit, setrlimit, Resource};

mod args;
mod configure;
mod port_range;
mod router;

use crate::args::Cli;
use crate::configure::Config;
use bytes::BytesMut;
use clap::Parser;
use dashmap::DashMap;
use futures::future::join_all;
use log::{debug, error, info, warn};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::net::{SocketAddr, UdpSocket as _DontUseUdpSocket};
use std::ops::Deref;
use std::sync::Arc;
use tokio::net::{ToSocketAddrs, UdpSocket};
use tokio::sync::Semaphore;
use tokio::{io, task};

use dashmap::mapref::one::Ref;
use std::os::unix::io::AsRawFd;
use tokio::io::Interest;
// use crate::router::Router;

/// Sets the `IP_TRANSPARENT` option for a given `UdpSocket`
fn set_ip_transparent(socket: &UdpSocket) -> io::Result<()> {
    let fd = socket.as_raw_fd();
    let opt_val: libc::c_int = 1;

    unsafe {
        let result = libc::setsockopt(
            fd,
            libc::SOL_IP,
            libc::IP_TRANSPARENT,
            &opt_val as *const _ as *const libc::c_void,
            std::mem::size_of_val(&opt_val) as libc::socklen_t,
        );
        if result != 0 {
            return Err(io::Error::last_os_error());
        }
    }

    Ok(())
}

/// bind to all required sockets concurrently

/// bind to all required sockets concurrently
async fn bind_sockets(socket_addrs: HashSet<SocketAddr>) -> Vec<Arc<UdpSocket>> {
    let tasks: Vec<_> = socket_addrs
        .into_iter()
        .map(|addr| {
            let addr_clone = addr.clone();
            task::spawn(async move {
                debug!("attempting to bind to socket: {}", addr_clone);
                match UdpSocket::bind(addr_clone).await {
                    Ok(socket) => Ok(Arc::new(socket)),
                    Err(error) => Err((addr_clone, error)),
                }
            })
        })
        .collect();

    let results = join_all(tasks).await;
    let mut bound_sockets = Vec::new();

    for task_result in results {
        match task_result {
            Ok(Ok(socket)) => {
                bound_sockets.push(socket);
            }
            Ok(Err((addr, error))) => {
                error!("failed to bind to socket: {} > {}", addr, error);
            }
            Err(join_error) => {
                error!("bind task failed with error > {}", join_error);
            }
        }
    }

    bound_sockets
}

/// handles forwarding packets for a given socket
async fn forward_task(
    socket: Arc<UdpSocket>,
    forward_routes: Arc<HashMap<SocketAddr, SocketAddr>>,
    backward_routes: Arc<HashMap<SocketAddr, SocketAddr>>,
    buf_pool: Arc<Semaphore>,
) -> io::Result<()> {
    let local_addr = socket.local_addr()?;
    info!("Listening on {}", local_addr);

    loop {
        // Acquire a buffer permit
        let _permit = buf_pool.acquire().await.unwrap();
        let mut buf = BytesMut::with_capacity(64 * 1024);
        buf.resize(64 * 1024, 0);

        let (len, src_addr) = match socket.recv_from(&mut buf).await {
            Ok(res) => res,
            Err(e) => {
                warn!("Error receiving from {}: {}", local_addr, e);
                continue;
            }
        };

        // 16 entries

        debug!("Received ");

        buf.truncate(len);

        // Determine packet direction (local -> remote or remote -> local)
        if let Some(remote_addr) = forward_routes.get(&local_addr) {
            if src_addr == *remote_addr {
                // Packet is remote -> local
                if let Some(local_addr) = backward_routes.get(remote_addr) {
                    // Forward to the local address
                    if let Err(e) = socket.send_to(&buf, *local_addr).await {
                        warn!("Error forwarding remote -> local packet: {}", e);
                    }
                } else {
                    debug!(
                        "No matching local route for remote response from {}",
                        src_addr
                    );
                }
            } else {
                // Packet is local -> remote
                if let Err(e) = socket.send_to(&buf, *remote_addr).await {
                    error!("Error forwarding local -> remote packet: {}", e);
                }
            }
        } else {
            debug!("No route found for local address: {}", local_addr);
        }
    }
}

fn set_unlimited_resource() -> io::Result<()> {
    // think this should work
    info!(
        "increase_nofile_limit reported: {}",
        increase_nofile_limit(u64::MAX - 1)?
    );
    Ok(())
}

struct Connection {
    remote_sock: UdpSocket,
    send_to: SocketAddr,

    local_sock: Arc<UdpSocket>,
    receive_from: SocketAddr,
    // maybe store a ref to the buffer pool
    // we will need a list of valid return addresses
}

impl Connection {
    async fn new( local_sock: Arc<UdpSocket>,send_to: SocketAddr) -> io::Result<Connection> {
        // get the address of the local socket
        // tiny bit of unnecessary overhead here
        let receive_from = local_sock.local_addr()?;

        // todo: maybe specify a way in the config to send from particular socket
        // bind the output socket; we dont care where it comes from
        let remote_sock = UdpSocket::bind("0.0.0.0:0").await?;

        Ok(Connection {
            remote_sock,
            send_to,

            receive_from,
            local_sock,
        })
    }

    async fn send(&self, bytes: &[u8]) -> io::Result<usize> {
        self.remote_sock.send_to(bytes, self.send_to).await
    }
}

struct Socket {
    // Arc<T> because we need to share to timeout thread
    active: Arc<DashMap<SocketAddr, Arc<Connection>>>, // active connections to the socket
    routes: DashMap<SocketAddr, SocketAddr>,           // C:x -> S:y
    socket: Arc<UdpSocket>,
}

impl Socket {
    pub async fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<Socket> {
        let socket = UdpSocket::bind(addr).await?;
        let socket = Arc::new(socket);

        Ok(Self {
            socket,
            active: Arc::new(DashMap::new()),
            routes: DashMap::new(), // start with empty routing table
        })
    }

    pub fn add_route<A: Into<SocketAddr>>(&mut self, from_addr: A, to_addr: A) {
        // create the route
        self.routes.insert(from_addr.into(), to_addr.into());
    }

    pub fn route(mut self, from_addr: SocketAddr, to_addr: SocketAddr) -> Self {
        self.add_route(from_addr, to_addr);
        self
    }

    pub fn socket(&self) -> &UdpSocket {
        self.socket.as_ref()
    }

    pub fn solve_route(&self, addr: &SocketAddr) -> Option<Ref<'_, SocketAddr, SocketAddr>> {
        self.routes.get(addr)
    }

    pub async fn send(&self, sent_from: SocketAddr, bytes: &[u8]) -> io::Result<()> {
        // lookup the correct route
        // if: no route
        // then: drop the packet
        // find connection
        // if: no connection
        // then: create connection
        // send from connection socket

        // lookup the correct route
        let Some(send_to) = self.solve_route(&sent_from) else {
            // drop the packet
            debug!("DROP");
            return Ok(());
        };

        // let connection = self.active.entry(sent_from).or_insert_with(|| {
        //     let connection = Connection::new(sent_from, Arc::clone(&self.socket)).await?;
        // });

        // let connection =  self.active.get(send_to).unwrap_or_else(Connection::new(sent_from, Arc::clone(&self.socket)));

        // get a handle on the connection
        let connection: Arc<Connection> = if let Some(active) = self.active.get(&send_to) {
            active.value().clone()
        } else {
            let connection = Connection::new(Arc::clone(&self.socket), *send_to).await?;
            let connection: Arc<Connection> = Arc::new(connection);

            self.active.insert(sent_from, connection.clone());
            connection
        };

        // send the bytes to the server
        let sent_size = connection.send(bytes).await?;
        debug!("SENT; LOCATION: {:?}, LEN: {sent_size}", connection.receive_from);

        Ok(())
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // start logging
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Debug)
        .init();
    
    let socket = Socket::bind("127.0.0.1:5000").await?
        .route("127.0.0.1:7000".parse().unwrap(), "127.0.0.1:6000".parse().unwrap());

    let mut buf = BytesMut::with_capacity(2048); // this is slow

    loop {
        socket.socket().readable().await?;

        let udp_socket = socket.socket();
        // todo: cache buffer size to determine when to shrink
        let (length, from) = match udp_socket.try_recv_buf_from(&mut buf) {
            Ok((length, from)) => (length, from),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => continue,
            Err(e) => return Err(e),
        };

        debug!("RECV; FROM {:?}, LEN: {length}, DATA {:?}", from, &buf[..length]);

        let bytes = &buf[..length];
        socket.send(from, &bytes[..length]).await?;

        buf.resize(2048, 0x0); // shrink buffer incase it has been expanded
    }

    Ok(())
}

// map error case

// #[tokio::main(flavor = "multi_thread", worker_threads = 10)]
// async fn main() -> io::Result<()> {
//     // env_logger::init();
//     env_logger::Builder::new()
//         .filter_level(log::LevelFilter::Debug)
//         .init();
//
//     let cli = Cli::parse();
//
//     let config = Config::load_file(cli.config_file).await?;
//     let router = config.router();
//
//     // by default yes
//     if cli.unlimited_resource {
//         set_unlimited_resource()?;
//     }
//
//     debug!("found routes: forward: {}, backwards: {}", router.get_forward_routes().len(), router.get_backward_routes().len());
//
//     // bind all required sockets
//     let sockets = bind_sockets(Router::required_sockets(router.get_forward_routes())).await;
//     debug!("bound {} sockets", sockets.len());
//
//     info!("udp proxy server starting");
//     // shared state
//
//     // let router_map = Arc::new(router.to_routes()); // Own the rooter map. It is now immutable
//
//     let (forwards_routes, backwards_routes) = router.to_forward_backward_routes();
//     let (forwards_routes, backwards_routes) = (Arc::new(forwards_routes), Arc::new(backwards_routes));
//
//     // let client_map = Arc::new(DashMap::<SocketAddr, SocketAddr>::new());
//     let buf_pool = Arc::new(Semaphore::new(config.buffer_pool_permits)); // Buffer pool
//
//     // spawn a forwarding task for each socket
//     for socket in sockets {
//         let forwards_routes = Arc::clone(&forwards_routes);
//         let backwards_routes = Arc::clone(&forwards_routes);
//         // let client_map = Arc::clone(&client_map);
//         let buf_pool = Arc::clone(&buf_pool);
//         tokio::spawn(async move {
//             if let Err(e) = forward_task(socket, forwards_routes, backwards_routes, buf_pool).await {
//                 warn!("Forward task error: {}", e);
//             }
//         });
//     }
//
//     // the main task can now wait forever
//     loop {
//         tokio::time::sleep(std::time::Duration::from_secs(60)).await;
//     }
// }
// mb proposal to type labels
