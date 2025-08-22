#!/usr/bin/env python3
import socket
import json
import time

def test_http_connect_with_path():
    """Test HTTP CONNECT with path extraction"""
    
    # Connect to the server
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    try:
        print("🔌 Connecting to localhost:3333...")
        sock.connect(('localhost', 3333))
        
        # Send HTTP CONNECT request with path
        connect_request = "CONNECT 130.211.20.161:9200/order-1 HTTP/1.1\r\nHost: 130.211.20.161:9200\r\n\r\n"
        print(f"📤 Sending HTTP CONNECT with path: {repr(connect_request)}")
        sock.send(connect_request.encode())
        
        # Read HTTP response
        response = sock.recv(1024).decode()
        print(f"📥 HTTP Response: {repr(response)}")
        
        if "200 Connection established" in response:
            print("✅ HTTP tunnel established! Now sending Stratum messages...")
            
            # Send mining.subscribe
            subscribe_msg = {"id": 1, "method": "mining.subscribe", "params": ["test-client/1.0"]}
            sock.send((json.dumps(subscribe_msg) + "\n").encode())
            print(f"📤 Sent: {subscribe_msg}")
            
            # Read response
            response = sock.recv(1024).decode()
            print(f"📥 Stratum response: {response.strip()}")
            
            # Send mining.authorize
            time.sleep(0.1)
            auth_msg = {"id": 2, "method": "mining.authorize", "params": ["john.test", "xpassword"]}
            sock.send((json.dumps(auth_msg) + "\n").encode())
            print(f"📤 Sent: {auth_msg}")
            
            # Read response
            response = sock.recv(1024).decode()
            print(f"📥 Auth response: {response.strip()}")
            
        else:
            print("❌ HTTP tunnel failed!")
            
    except Exception as e:
        print(f"❌ Error: {e}")
    finally:
        sock.close()
        print("🔌 Connection closed")

if __name__ == "__main__":
    print("🧪 Testing HTTP CONNECT with path extraction")
    test_http_connect_with_path()