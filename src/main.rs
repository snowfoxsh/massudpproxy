mod router;
mod configure;
mod port_range;
mod args;

use bytes::BytesMut;
use dashmap::DashMap;
use futures::future::join_all;
use log::{debug, error, info};
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use clap::Parser;
use tokio::net::UdpSocket;
use tokio::sync::Semaphore;
use tokio::{io, task};
use crate::args::Cli;
use crate::configure::Config;

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
/// - `router` gives static routes from local address to remote.
/// - `client_map` tracks client <-> server pairs.
/// - `buf_pool` is a semaphore for controlling buffer usage.
async fn forward_task(
    socket: Arc<UdpSocket>,
    router: Arc<DashMap<SocketAddr, SocketAddr>>,
    client_map: Arc<DashMap<SocketAddr, SocketAddr>>,
    buf_pool: Arc<Semaphore>,
) -> io::Result<()> {
    let local_addr = socket.local_addr()?;
    info!("Listening on {}", local_addr);

    loop {
        // acquire a buffer permit
        let _permit = buf_pool.acquire().await.unwrap();
        let mut buf = BytesMut::with_capacity(64 * 1024);
        // ensure the buffer has some initial capacity
        buf.resize(64 * 1024, 0);

        let (len, src_addr) = match socket.recv_from(&mut buf).await {
            Ok(res) => res,
            Err(e) => {
                error!("Error receiving from {}: {}", local_addr, e);
                continue;
            }
        };

        buf.truncate(len);

        // determine if this is a client->server or server->client packet
        // if incoming packet source isn't one of our known routes,
        // this likely is client->server traffic.

        // check if local_addr matches a known route
        if let Some(server_addr) = router.get(&local_addr) {
            // we have a defined route: local_addr -> server_addr.value()

            // is this packet coming from the server side or the client side?
            // ff src_addr == server_addr, then its server->client
            // otherwise it's client->server
            if src_addr == *server_addr {
                // server->client direction
                if let Some(client_addr) = client_map.get(&src_addr) {
                    // forward to client
                    if let Err(e) = socket.send_to(&buf, *client_addr).await {
                        error!("Error forwarding server->client: {}", e);
                    }
                } else {
                    // no client mapping found; ignore or log
                    debug!("No client found for server {} response", src_addr);
                }
            } else {
                // client->server direction
                // store the mapping both ways:
                // client_addr -> server_addr
                client_map.insert(src_addr, *server_addr);

                // also store server_addr -> client_addr for reverse lookup
                client_map.insert(*server_addr, src_addr);

                // forward to server
                if let Err(e) = socket.send_to(&buf, *server_addr).await {
                    error!("Error forwarding client->server: {}", e);
                }
            }
        } else {
            // no route found for this local_addr, ignore or log
            debug!("No route configured for local_addr: {}", local_addr);
        }
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    
    let config = Config::load_file(cli.config_file).await?;
    let router = config.router();
    
    debug!("found routes: {:?}", router.routes());
    
    // bind all required sockets
    let sockets = bind_sockets(router.required_sockets()).await;
    debug!("bound sockets: {:?}", sockets);

    info!("udp proxy server starting");
    // shared state
    let router_map = router.routes(); // Arc<DashMap<SocketAddr, SocketAddr>>
    let client_map = Arc::new(DashMap::<SocketAddr, SocketAddr>::new());
    let buf_pool = Arc::new(Semaphore::new(config.buffer_pool_permits)); // Buffer pool

    // spawn a forwarding task for each socket
    for socket in sockets {
        let router_map = Arc::clone(&router_map);
        let client_map = Arc::clone(&client_map);
        let buf_pool = Arc::clone(&buf_pool);
        tokio::spawn(async move {
            if let Err(e) = forward_task(socket, router_map, client_map, buf_pool).await {
                error!("Forward task error: {}", e);
            }
        });
    }

    // the main task can now wait forever
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}
