use std::net::SocketAddr;
use tokio::net::UdpSocket;
use std::sync::Arc;
use tokio::sync::Semaphore;
use bytes::BytesMut;
use dashmap::DashMap;
use tokio::io;
// 
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> io::Result<()> {
    let proxy_socket = UdpSocket::bind("127.0.0.1:5000").await?;
    println!("UDP proxy listening on 127.0.0.1:5000");

    let server_addr: SocketAddr = "127.0.0.1:6000".parse().unwrap();
    let client_map = Arc::new(DashMap::<SocketAddr, SocketAddr>::new());
    let proxy_socket = Arc::new(proxy_socket);

    // Shared buffer pool
    let buf_pool = Arc::new(Semaphore::new(128));

    loop {
        let proxy_socket = Arc::clone(&proxy_socket);
        let client_map = Arc::clone(&client_map);
        let server_addr = server_addr.clone();
        let buf_pool = Arc::clone(&buf_pool);

        tokio::spawn(async move {
            let _permit = buf_pool.acquire().await.unwrap(); // Acquire buffer
            let mut buf = BytesMut::with_capacity(64 * 1024);

            if let Ok((len, client_addr)) = proxy_socket.recv_from(&mut buf).await {
                client_map.insert(client_addr, server_addr);
                if let Err(e) = proxy_socket.send_to(&buf[..len], server_addr).await {
                    eprintln!("Error forwarding to server: {}", e);
                }
            }
        });
    }
}
