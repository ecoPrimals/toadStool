# biomeOS Integration - Build & Test Guide

## Quick Start

### Build the Server
```bash
cd /home/strandgate/Development/ecoPrimals/phase1/toadStool
export PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1
cargo build --release --bin toadstool-server
```

### Run the Server
```bash
export RUST_LOG=info
export TOADSTOOL_FAMILY=default
./target/release/toadstool-server
```

### Test JSON-RPC Connection

#### Using netcat
```bash
# In another terminal
nc 127.0.0.1 9944
{"jsonrpc":"2.0","method":"toadstool.query_capabilities","id":1}
```

#### Using Python
```python
import socket
import json

sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.connect(('127.0.0.1', 9944))

# Query capabilities
request = {
    "jsonrpc": "2.0",
    "method": "toadstool.query_capabilities",
    "params": {},
    "id": 1
}
sock.sendall((json.dumps(request) + '\n').encode())
response = json.loads(sock.recv(4096).decode())
print(f"Capabilities: {response}")

sock.close()
```

#### Using cURL (if HTTP wrapper added)
```bash
curl -X POST http://127.0.0.1:9944 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "toadstool.health",
    "id": 1
  }'
```

## Available JSON-RPC Methods

### 1. `toadstool.query_capabilities`
Returns compute capabilities, units, and available resources.

```json
{
  "jsonrpc": "2.0",
  "method": "toadstool.query_capabilities",
  "id": 1
}
```

### 2. `toadstool.submit_workload`
Submit a compute workload.

```json
{
  "jsonrpc": "2.0",
  "method": "toadstool.submit_workload",
  "params": {
    "workload_id": "job-001",
    "workload_type": "cpu_compute",
    "data": "YmFzZTY0X2VuY29kZWRfZGF0YQ==",
    "metadata": {},
    "priority": "Normal",
    "requirements": {
      "cpu_cores": 4,
      "memory_bytes": 1073741824,
      "timeout_secs": 300
    }
  },
  "id": 2
}
```

### 3. `toadstool.query_status`
Query workload status.

```json
{
  "jsonrpc": "2.0",
  "method": "toadstool.query_status",
  "params": {
    "workload_id": "job-001"
  },
  "id": 3
}
```

### 4. `toadstool.cancel_workload`
Cancel a running workload.

```json
{
  "jsonrpc": "2.0",
  "method": "toadstool.cancel_workload",
  "params": {
    "workload_id": "job-001"
  },
  "id": 4
}
```

### 5. `toadstool.list_workloads`
List all workloads.

```json
{
  "jsonrpc": "2.0",
  "method": "toadstool.list_workloads",
  "params": {
    "filter": null
  },
  "id": 5
}
```

### 6. `toadstool.health`
Health check.

```json
{
  "jsonrpc": "2.0",
  "method": "toadstool.health",
  "id": 6
}
```

### 7. `toadstool.version`
Get version information.

```json
{
  "jsonrpc": "2.0",
  "method": "toadstool.version",
  "id": 7
}
```

## Environment Variables

- `TOADSTOOL_FAMILY` - Family identifier (default: "default")
- `RUST_LOG` - Log level (trace, debug, info, warn, error)
- `PYO3_USE_ABI3_FORWARD_COMPATIBILITY` - Python 3.13 compatibility

## Architecture

```
┌─────────────────────────────────────────┐
│        biomeOS Client (Python)          │
│    - JSON-RPC 2.0 client library        │
└───────────────┬─────────────────────────┘
                │ TCP (127.0.0.1:9944)
                ▼
┌─────────────────────────────────────────┐
│    ToadStool Server Daemon              │
│  ┌───────────────────────────────────┐  │
│  │   JSON-RPC 2.0 Server             │  │
│  │   - 7 methods                     │  │
│  │   - Type-safe Rust                │  │
│  └─────────────┬─────────────────────┘  │
│                ▼                         │
│  ┌───────────────────────────────────┐  │
│  │   WorkloadExecutor                │  │
│  │   - MockExecutor (temporary)      │  │
│  │   - Real executor (future)        │  │
│  └─────────────┬─────────────────────┘  │
│                ▼                         │
│  ┌───────────────────────────────────┐  │
│  │   ToadStool Runtime               │  │
│  │   - CPU, GPU, Neuromorphic        │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

## Status

✅ JSON-RPC 2.0 server - COMPLETE  
✅ 7 methods implemented - COMPLETE  
✅ Server daemon - COMPLETE  
✅ XDG socket path - COMPLETE (TCP fallback)  
✅ Songbird registration - COMPLETE (framework)  
🟡 tarpc server - IN PROGRESS (not needed for biomeOS)  

## Next Steps

1. **Build release binary**: `cargo build --release --bin toadstool-server`
2. **Test locally**: Use Python/netcat examples above
3. **Integrate with biomeOS**: Use JSON-RPC client library
4. **Expand capabilities**: Add GPU support, distributed coordination

## Support

See `BIOMEOS_PHASE1_COMPLETE.md` for detailed architecture and examples.

