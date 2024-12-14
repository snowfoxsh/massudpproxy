import socket
import threading
import time

def udp_server(server_host, server_port):
    """Simulates a UDP server that receives messages and sends a response."""
    server_sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    server_sock.bind((server_host, server_port))
    print(f"Server listening on {server_host}:{server_port}")

    while True:
        message, client_addr = server_sock.recvfrom(1024)
        print(f"Server received: {message.decode()} from {client_addr}")
        response = f"Echo: {message.decode()}"
        server_sock.sendto(response.encode(), client_addr)

def udp_client(proxy_host, proxy_port, message, timeout=1):
    """Simulates a UDP client that sends a message to the proxy and waits for a response."""
    client_sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    client_sock.settimeout(timeout)

    try:
        print(f"Client sending: {message} to {proxy_host}:{proxy_port}")
        client_sock.sendto(message.encode(), (proxy_host, proxy_port))
        response, _ = client_sock.recvfrom(1024)
        print(f"Client received: {response.decode()}")
    except socket.timeout:
        print("Client timed out waiting for a response.")
    finally:
        client_sock.close()

def main():
    server_host = "127.0.0.1"
    proxy_host = "127.0.0.1"
    start_port = 5000
    end_port = 5000
    server_base_port = 6000

    # Start a server for each port in the range
    for i in range(start_port, end_port + 1):
        server_port = server_base_port + (i - start_port)
        threading.Thread(target=udp_server, args=(server_host, server_port), daemon=True).start()

    # Give servers time to start
    time.sleep(1)

    # Start clients for each port in the range
    for i in range(start_port, end_port + 1):
        message = f"Hello from client on port {i}"
        threading.Thread(target=udp_client, args=(proxy_host, i, message), daemon=True).start()

    # Keep the main thread alive to observe the results
    time.sleep(5)

if __name__ == "__main__":
    main()
