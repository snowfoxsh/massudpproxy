mod router;

use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use futures::future::join_all;
use log::{debug, error};
use tokio::{io, task};
use tokio::net::UdpSocket;
use crate::router::Router;


// consider better error handling here
async fn bind_sockets(socket_addrs: HashSet<SocketAddr>) -> Vec<Arc<UdpSocket>> {
    let tasks: Vec<_> = socket_addrs
        .into_iter()
        .map(|addr| task::spawn(async move {
            debug!("attempting to bind to socket: {}", addr);
            match UdpSocket::bind(addr).await {
                Ok(socket) => Ok(Arc::new(socket)),
                Err(error) => Err((addr, error)),
            }
        }))
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

#[tokio::main]
async fn main() -> io::Result<()> {
    env_logger::init();
    
    // create our router manager
    let router = Router::new().route(
        "127.0.0.1:5000".parse().unwrap(),
        "127.0.0.1:6000".parse().unwrap(),
    );

    println!("{:?}", router.routes());
    
    // todo: improve error handling it will ignore errors right now
    let sockets = bind_sockets(router.required_sockets()).await;
    
    debug!("sockets: {:?}", sockets);
    Ok(())
}

// map
// inputs -> outputs
// inputs:x-y --1:1-> outputs:x-y
// ^^^ i need to handle multiple rules sets for these

// create a table of where things need to go

