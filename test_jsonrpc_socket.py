#!/usr/bin/env python3
"""
Test Toadstool JSON-RPC socket with both raw and HTTP-wrapped formats
"""
import socket
import json
import sys
import time

def test_raw_jsonrpc(socket_path):
    """Test raw JSON-RPC (newline-delimited)"""
    print("\n🧪 Testing RAW JSON-RPC format...")
    print(f"   Socket: {socket_path}")
    
    try:
        sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        sock.settimeout(5.0)
        sock.connect(socket_path)
        
        # Send raw JSON-RPC request (what biomeOS sends)
        request = json.dumps({
            "jsonrpc": "2.0",
            "method": "toadstool.health",
            "params": {},
            "id": 1
        }) + "\n"
        
        print(f"   Request: {request.strip()}")
        sock.sendall(request.encode())
        
        # Receive response
        response = sock.recv(4096).decode().strip()
        print(f"   Response: {response}")
        
        # Parse and validate
        result = json.loads(response)
        assert result.get("jsonrpc") == "2.0", "Invalid jsonrpc version"
        assert "result" in result, "Missing result field"
        assert result["result"].get("healthy") == True, "Not healthy"
        
        sock.close()
        print("   ✅ RAW JSON-RPC test PASSED")
        return True
        
    except socket.timeout:
        print("   ❌ TIMEOUT - No response received")
        return False
    except Exception as e:
        print(f"   ❌ ERROR: {e}")
        return False

def test_http_wrapped_jsonrpc(socket_path):
    """Test HTTP-wrapped JSON-RPC (for compatibility)"""
    print("\n🧪 Testing HTTP-WRAPPED JSON-RPC format...")
    print(f"   Socket: {socket_path}")
    
    try:
        sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        sock.settimeout(5.0)
        sock.connect(socket_path)
        
        # Send HTTP-wrapped JSON-RPC request
        body = json.dumps({
            "jsonrpc": "2.0",
            "method": "toadstool.version",
            "params": {},
            "id": 2
        })
        
        request = (
            f"POST / HTTP/1.1\r\n"
            f"Content-Type: application/json\r\n"
            f"Content-Length: {len(body)}\r\n"
            f"\r\n"
            f"{body}"
        )
        
        print(f"   Request: POST / HTTP/1.1 (Content-Length: {len(body)})")
        sock.sendall(request.encode())
        
        # Receive response
        response = sock.recv(4096).decode()
        print(f"   Response: {response[:100]}...")
        
        # Parse HTTP response
        headers, body = response.split("\r\n\r\n", 1)
        result = json.loads(body.strip())
        
        assert result.get("jsonrpc") == "2.0", "Invalid jsonrpc version"
        assert "result" in result, "Missing result field"
        assert "version" in result["result"], "Missing version"
        
        sock.close()
        print("   ✅ HTTP-WRAPPED JSON-RPC test PASSED")
        return True
        
    except socket.timeout:
        print("   ❌ TIMEOUT - No response received")
        return False
    except Exception as e:
        print(f"   ❌ ERROR: {e}")
        return False

def test_query_capabilities(socket_path):
    """Test capabilities query"""
    print("\n🧪 Testing toadstool.query_capabilities...")
    print(f"   Socket: {socket_path}")
    
    try:
        sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        sock.settimeout(5.0)
        sock.connect(socket_path)
        
        request = json.dumps({
            "jsonrpc": "2.0",
            "method": "toadstool.query_capabilities",
            "params": {},
            "id": 3
        }) + "\n"
        
        print(f"   Request: toadstool.query_capabilities")
        sock.sendall(request.encode())
        
        response = sock.recv(8192).decode().strip()
        result = json.loads(response)
        
        print(f"   Response: {json.dumps(result, indent=2)[:200]}...")
        
        assert result.get("jsonrpc") == "2.0", "Invalid jsonrpc version"
        assert "result" in result, "Missing result field"
        
        sock.close()
        print("   ✅ query_capabilities test PASSED")
        return True
        
    except Exception as e:
        print(f"   ❌ ERROR: {e}")
        return False

if __name__ == "__main__":
    import os
    
    # Default socket path
    runtime_dir = os.environ.get("XDG_RUNTIME_DIR", "/run/user/1000")
    socket_path = os.path.join(runtime_dir, "toadstool-default.jsonrpc.sock")
    
    if len(sys.argv) > 1:
        socket_path = sys.argv[1]
    
    print("=" * 80)
    print("🍄 Toadstool JSON-RPC Socket Test")
    print("=" * 80)
    print(f"\nSocket: {socket_path}")
    
    # Check if socket exists
    if not os.path.exists(socket_path):
        print(f"\n❌ Socket does not exist: {socket_path}")
        print("   Start toadstool daemon first: cargo run --release -- daemon")
        sys.exit(1)
    
    # Run tests
    results = []
    results.append(("Raw JSON-RPC", test_raw_jsonrpc(socket_path)))
    time.sleep(0.1)
    results.append(("HTTP-wrapped", test_http_wrapped_jsonrpc(socket_path)))
    time.sleep(0.1)
    results.append(("Capabilities", test_query_capabilities(socket_path)))
    
    # Summary
    print("\n" + "=" * 80)
    print("📊 TEST SUMMARY")
    print("=" * 80)
    passed = sum(1 for _, result in results if result)
    total = len(results)
    
    for name, result in results:
        status = "✅ PASS" if result else "❌ FAIL"
        print(f"   {name:20s} {status}")
    
    print(f"\n   Total: {passed}/{total} tests passed")
    
    if passed == total:
        print("\n🎉 ALL TESTS PASSED - JSON-RPC socket is working!")
        sys.exit(0)
    else:
        print("\n⚠️  Some tests failed - see details above")
        sys.exit(1)
