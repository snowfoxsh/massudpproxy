import socket
import threading
import time

# Define multiple proxy-server routes for the test
ROUTES = [
    (("127.0.0.1", 5000), ("127.0.0.1", 6000)),  # Proxy:5000 -> Server:6000
    (("127.0.0.1", 5001), ("127.0.0.1", 6001)),  # Proxy:5001 -> Server:6001
]

# Base client port for uniqueness, we'll increment for each route
CLIENT_BASE_PORT = 7000

# Number of packets to send per client
PACKETS_PER_CLIENT = 5

def mock_server(server_addr):
    """Mock server listening on server_addr and responding to every packet."""
    server_socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    server_socket.bind(server_addr)
    print(f"Mock Server started on {server_addr}")

    while True:
        data, addr = server_socket.recvfrom(1024)
        print(f"Mock Server {server_addr}: Received '{data.decode()}' from {addr}")
        # Respond to the client (which is actually the proxy in a real scenario)
        server_socket.sendto(b"Hello from server", addr)


def test_client(proxy_addr, server_addr, client_addr, packets_to_send=1):
    """Client sends multiple packets to the proxy and waits for responses."""
    client_socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    client_socket.bind(client_addr)
    client_socket.settimeout(5)  # add timeout to avoid hanging if no response

    # Send multiple packets to the proxy
    for i in range(packets_to_send):
        msg = f"Hello from client {client_addr} packet#{i}".encode()
        client_socket.sendto(msg, proxy_addr)
        print(f"Client {client_addr}: Sent '{msg.decode()}' to proxy {proxy_addr}")

        # Wait for response
        data, addr = client_socket.recvfrom(1024)
        resp = data.decode()
        print(f"Client {client_addr}: Received '{resp}' from {addr}")
        # Verify the response
        assert resp == "Hello from server", f"Test failed: Unexpected response '{resp}'"

    print(f"Client {client_addr}: Test passed for route {proxy_addr} -> {server_addr}")
    client_socket.close()


if __name__ == "__main__":
    # Start mock servers for each server address
    for _, server_addr in ROUTES:
        threading.Thread(target=mock_server, args=(server_addr,), daemon=True).start()

    # Give servers a moment to start
    time.sleep(0.5)

    # Run clients concurrently
    client_threads = []
    for i, (proxy_addr, server_addr) in enumerate(ROUTES):
        client_port = CLIENT_BASE_PORT + i
        client_addr = ("127.0.0.1", client_port)
        t = threading.Thread(target=test_client, args=(proxy_addr, server_addr, client_addr, PACKETS_PER_CLIENT))
        t.start()
        client_threads.append(t)

    # Wait for all client threads to finish
    for t in client_threads:
        t.join()

    print("All tests completed successfully.")
