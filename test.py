import socket
import threading
import time

# Configuration
SEND_FROM_ADDRESS = ("127.0.0.1", 7000)  # Source address for sending
SEND_TO_ADDRESS = ("127.0.0.1", 5000)    # Rust socket bound address
RECEIVE_FROM_ADDRESS = ("127.0.0.1", 6000)  # Destination for forwarded packets

# Message to send
MESSAGE = "Hello, testing route from Python!"

# Socket timeout
TIMEOUT = 2


def send_data(send_from_addr, send_to_addr, message):
    """Send data from a specific address to the target address."""
    sender_socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sender_socket.bind(send_from_addr)  # Bind to the specified source address
    sender_socket.sendto(message.encode(), send_to_addr)
    print(f"Sent: '{message}' from {send_from_addr} to {send_to_addr}")
    sender_socket.close()


def receive_data(receive_from_addr):
    """Receive data on the specified address."""
    receiver_socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    receiver_socket.bind(receive_from_addr)  # Bind to the specified address
    receiver_socket.settimeout(TIMEOUT)

    try:
        data, addr = receiver_socket.recvfrom(1024)
        print(f"Received: '{data.decode()}' from {addr}")
    except socket.timeout:
        print(f"No response received within {TIMEOUT} seconds.")
    finally:
        receiver_socket.close()


if __name__ == "__main__":
    # Start a thread to listen for forwarded data
    receiver_thread = threading.Thread(
        target=receive_data, args=(RECEIVE_FROM_ADDRESS,), daemon=True
    )
    receiver_thread.start()

    # Give the listener a moment to start
    time.sleep(1)

    # Send data to the Rust socket
    send_data(SEND_FROM_ADDRESS, SEND_TO_ADDRESS, MESSAGE)

    # Wait for the receiver thread to finish
    receiver_thread.join()
