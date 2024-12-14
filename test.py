import socket
import threading

# Proxy and server addresses
PROXY_ADDR = ("127.0.0.1", 5000)
SERVER_ADDR = ("127.0.0.1", 6000)
CLIENT_ADDR = ("127.0.0.1", 7000)


def mock_server():
    """Mock server that listens for packets from the proxy and sends a response."""
    server_socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    server_socket.bind(SERVER_ADDR)

    while True:
        data, addr = server_socket.recvfrom(1024)
        print(f"Mock Server: Received '{data.decode()}' from {addr}")
        server_socket.sendto(b"Hello from server", addr)


def test_client():
    """Simulates a client sending data to the proxy and receiving a response."""
    client_socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    client_socket.bind(CLIENT_ADDR)

    # Send a message to the proxy
    client_socket.sendto(b"Hello from client", PROXY_ADDR)
    print(f"Client: Sent 'Hello from client' to proxy {PROXY_ADDR}")

    # Wait for a response from the proxy (forwarded from the server)
    data, addr = client_socket.recvfrom(1024)
    print(f"Client: Received '{data.decode()}' from {addr}")

    # Verify the response
    assert data.decode() == "Hello from server", "Test failed: Unexpected response"
    print("Test passed!")


if __name__ == "__main__":
    # Start the mock server in a separate thread
    server_thread = threading.Thread(target=mock_server, daemon=True)
    server_thread.start()

    # Run the client test
    test_client()
