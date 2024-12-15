import socket
import threading
import time

# Define routes for testing
ROUTES = [
    (("127.0.0.1", 5000), ("127.0.0.1", 6000)),
    (("127.0.0.1", 5001), ("127.0.0.1", 6001)),
]

# Local addresses to use for clients
LOCAL_ADDRESSES = ["127.0.0.2", "127.0.0.3"]

# Port range for testing
START_PORT = 7000
END_PORT = 7010

# Number of packets to send per client
PACKETS_PER_CLIENT = 5

# Timeout for receiving responses
RECV_TIMEOUT = 2

# Dropped ports log
dropped_ports = threading.Lock()
dropped_ports_list = []

def mock_server(server_addr):
    """Mock server listening on server_addr and responding to every packet."""
    server_socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    server_socket.bind(server_addr)
    print(f"Mock Server started on {server_addr}")

    while True:
        try:
            data, addr = server_socket.recvfrom(1024)
            print(f"Mock Server {server_addr}: Received '{data.decode()}' from {addr}")
            # Respond to the client (proxy in a real scenario)
            server_socket.sendto(b"Hello from server", addr)
        except Exception as e:
            print(f"Mock Server {server_addr}: Error - {e}")


def test_client(proxy_addr, server_addr, local_addr, port_range, packets_to_send):
    """Client sends packets to a range of ports and logs dropped ports."""
    for port in port_range:
        client_socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        client_socket.bind((local_addr, port))
        client_socket.settimeout(RECV_TIMEOUT)

        dropped = False
        for i in range(packets_to_send):
            msg = f"Hello from {local_addr}:{port} packet#{i}".encode()
            client_socket.sendto(msg, proxy_addr)
            print(f"Client {local_addr}:{port}: Sent '{msg.decode()}' to proxy {proxy_addr}")

            try:
                data, addr = client_socket.recvfrom(1024)
                resp = data.decode()
                print(f"Client {local_addr}:{port}: Received '{resp}' from {addr}")
            except socket.timeout:
                print(f"Client {local_addr}:{port}: No response for packet#{i}")
                dropped = True
                break

        if dropped:
            with dropped_ports:
                dropped_ports_list.append(port)

        client_socket.close()


if __name__ == "__main__":
    # Start mock servers for each server address
    for _, server_addr in ROUTES:
        threading.Thread(target=mock_server, args=(server_addr,), daemon=True).start()

    # Give servers a moment to start
    time.sleep(0.5)

    # Create port range
    port_range = range(START_PORT, END_PORT)

    # Run clients concurrently for each local address
    client_threads = []
    for local_addr in LOCAL_ADDRESSES:
        for proxy_addr, server_addr in ROUTES:
            t = threading.Thread(
                target=test_client,
                args=(proxy_addr, server_addr, local_addr, port_range, PACKETS_PER_CLIENT),
            )
            t.start()
            client_threads.append(t)

    # Wait for all client threads to finish
    for t in client_threads:
        t.join()

    # Log dropped ports
    if dropped_ports_list:
        print(f"\nDropped Ports: {sorted(set(dropped_ports_list))}")
    else:
        print("\nNo ports were dropped.")

    print("All tests completed successfully.")
