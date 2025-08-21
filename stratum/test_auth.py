#!/usr/bin/env python3

import socket
import json
import time

def test_authentication():
    """Test the authentication transformation"""
    print("Testing authentication transformation...")
    
    # Connect to the stratum server
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    try:
        sock.connect(('localhost', 3333))
        print("✅ Connected to Loka Stratum on port 3333")
        
        # Send mining.authorize with john.test
        auth_message = {
            "id": 1,
            "method": "mining.authorize",
            "params": ["john.test", "password"]
        }
        
        message_str = json.dumps(auth_message) + "\n"
        print(f"📤 Sending: {message_str.strip()}")
        
        sock.send(message_str.encode())
        
        # Wait a bit for processing
        time.sleep(1)
        
        print("✅ Authentication message sent successfully")
        print("🔍 Check the logs above to see if username was transformed to '37vuX2XMqtcrobGwxSZJSwJoYyjiH18SiQ.john_test'")
        
    except Exception as e:
        print(f"❌ Error: {e}")
    finally:
        sock.close()

if __name__ == "__main__":
    test_authentication()