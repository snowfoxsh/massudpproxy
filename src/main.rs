use tokio::net::UdpSocket;
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::sync::Mutex;
use std::sync::Arc;
use std::io;

#[tokio::main]
async fn main() -> io::Result<()> {
    // Bind the proxy to port 5000
    let proxy_socket = UdpSocket::bind("127.0.0.1:5000").await?;
    println!("UDP proxy listening on 127.0.0.1:5000");

    // Remote server address
    let server_addr: SocketAddr = "127.0.0.1:6000".parse().unwrap();

    // Map to track client <-> server address associations
    let client_map = Arc::new(Mutex::new(HashMap::<SocketAddr, SocketAddr>::new()));
    let proxy_socket = Arc::new(proxy_socket);

    let mut buf = vec![0u8; 1024];

    loop {
        // Receive a packet from a client
        let (len, client_addr) = proxy_socket.recv_from(&mut buf).await?;
        println!("Received {} bytes from client {}", len, client_addr);

        // Forward the packet to the server
        proxy_socket.send_to(&buf[..len], server_addr).await?;
        println!("Forwarded {} bytes to server {}", len, server_addr);

        // Receive a response from the server
        let (len, server_response_addr) = proxy_socket.recv_from(&mut buf).await?;
        println!(
            "Received {} bytes from server {}",
            len, server_response_addr
        );

        // Forward the server's response back to the client
        proxy_socket.send_to(&buf[..len], client_addr).await?;
        println!("Forwarded {} bytes back to client {}", len, client_addr);
    }
}
