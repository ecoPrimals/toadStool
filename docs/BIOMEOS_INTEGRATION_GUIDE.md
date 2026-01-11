# biomeOS Integration Guide - Collaborative Intelligence

**Last Updated**: January 11, 2026  
**Status**: Production Ready  
**ToadStool Version**: 2.2.0  
**API Version**: JSON-RPC 2.0

---

## Overview

This guide explains how to integrate biomeOS with ToadStool's Collaborative Intelligence Resource Planning API. ToadStool provides resource estimation, availability validation, and optimization suggestions for execution graphs - enabling biomeOS to make intelligent decisions about workload placement and resource allocation.

### What You'll Learn

- How to connect to ToadStool via Unix sockets
- How to construct execution graphs
- How to call the three collaborative intelligence methods
- How to interpret results and handle errors
- Best practices for production integration

---

## Table of Contents

1. [Architecture](#architecture)
2. [Connection Setup](#connection-setup)
3. [API Methods](#api-methods)
4. [Integration Patterns](#integration-patterns)
5. [Error Handling](#error-handling)
6. [Best Practices](#best-practices)
7. [Troubleshooting](#troubleshooting)

---

## Architecture

### Communication Protocol

ToadStool uses **JSON-RPC 2.0 over Unix sockets** for biomeOS integration:

- **Protocol**: JSON-RPC 2.0 (universal, language-agnostic)
- **Transport**: Unix socket (secure, low-latency, no TCP overhead)
- **Socket Path**: `/run/user/<uid>/toadstool-<family>.jsonrpc.sock`
- **Default**: `/run/user/1000/toadstool-default.jsonrpc.sock`

### Why Unix Sockets?

✅ **Security**: No network exposure, file permissions control access  
✅ **Performance**: Lower latency than TCP, zero network overhead  
✅ **Multi-Instance**: Each ToadStool instance has unique socket path  
✅ **Discovery**: Use Songbird for capability-based discovery

### Architecture Diagram

```
┌─────────────────┐                    ┌──────────────────┐
│                 │                    │                  │
│    biomeOS      │◄──── Unix Socket ─│   ToadStool      │
│  (Python/Rust)  │  JSON-RPC 2.0     │  (Rust Server)   │
│                 │                    │                  │
└─────────────────┘                    └──────────────────┘
        │                                      │
        │                                      │
        ▼                                      ▼
  Neural API Graph              Resource Planning Modules
  - Nodes & Edges               - Estimator
  - Dependencies                - Validator
  - Resources                   - Optimizer
```

---

## Connection Setup

### 1. Discover ToadStool via Songbird

Use Songbird to discover ToadStool instances with collaborative intelligence capability:

```python
import songbird_client

# Discover ToadStool instances
instances = songbird_client.discover_by_capability("collaborative_intelligence")

for instance in instances:
    print(f"Found ToadStool: {instance.name}")
    print(f"  Socket: {instance.location.socket_path}")
    print(f"  Capabilities: {instance.capabilities}")
```

### 2. Connect to Unix Socket

#### Python Example

```python
import socket
import json

class ToadStoolClient:
    def __init__(self, socket_path="/run/user/1000/toadstool-default.jsonrpc.sock"):
        self.socket_path = socket_path
        self.request_id = 0
    
    def _call_rpc(self, method: str, params: dict) -> dict:
        """Call JSON-RPC method via Unix socket."""
        self.request_id += 1
        
        # Create JSON-RPC 2.0 request
        request = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": self.request_id
        }
        
        # Connect to Unix socket
        sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        sock.connect(self.socket_path)
        
        # Send HTTP POST request
        request_body = json.dumps(request)
        http_request = f"""POST / HTTP/1.1\r
Host: toadstool\r
Content-Type: application/json\r
Content-Length: {len(request_body)}\r
\r
{request_body}"""
        
        sock.sendall(http_request.encode())
        
        # Read response
        response = b""
        while True:
            chunk = sock.recv(4096)
            if not chunk:
                break
            response += chunk
            # Simple check for end of response
            if b'\r\n\r\n' in response and b'}' in response:
                break
        
        sock.close()
        
        # Parse HTTP response
        response_str = response.decode('utf-8')
        body_start = response_str.find('\r\n\r\n') + 4
        body = json.loads(response_str[body_start:])
        
        # Check for JSON-RPC error
        if "error" in body:
            raise Exception(f"JSON-RPC Error: {body['error']}")
        
        return body["result"]
```

#### Rust Example

```rust
use tokio::net::UnixStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde_json::{json, Value};

pub struct ToadStoolClient {
    socket_path: String,
    request_id: u64,
}

impl ToadStoolClient {
    pub fn new(socket_path: impl Into<String>) -> Self {
        Self {
            socket_path: socket_path.into(),
            request_id: 0,
        }
    }
    
    pub async fn call_rpc(&mut self, method: &str, params: Value) -> Result<Value, Box<dyn std::error::Error>> {
        self.request_id += 1;
        
        // Create JSON-RPC 2.0 request
        let request = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": self.request_id
        });
        
        // Connect to Unix socket
        let mut stream = UnixStream::connect(&self.socket_path).await?;
        
        // Send HTTP POST request
        let request_body = serde_json::to_string(&request)?;
        let http_request = format!(
            "POST / HTTP/1.1\r\nHost: toadstool\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            request_body.len(),
            request_body
        );
        
        stream.write_all(http_request.as_bytes()).await?;
        
        // Read response
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await?;
        
        // Parse HTTP response
        let response_str = String::from_utf8(response)?;
        let body_start = response_str.find("\r\n\r\n").unwrap() + 4;
        let body: Value = serde_json::from_str(&response_str[body_start..])?;
        
        // Check for JSON-RPC error
        if let Some(error) = body.get("error") {
            return Err(format!("JSON-RPC Error: {}", error).into());
        }
        
        Ok(body["result"].clone())
    }
}
```

---

## API Methods

ToadStool provides three JSON-RPC methods for collaborative intelligence:

### 1. `resources.estimate`

**Purpose**: Estimate resource requirements for an execution graph

**Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "resources.estimate",
  "params": {
    "graph": {
      "id": "ml_training",
      "nodes": [...],
      "edges": [...]
    }
  },
  "id": 1
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "graph_id": "ml_training",
    "cpu_cores": 16,
    "memory_bytes": 68719476736,
    "gpu_memory_bytes": 17179869184,
    "storage_bytes": 107374182400,
    "network_bandwidth_mbps": 1000,
    "estimated_duration": {
      "secs": 3900,
      "nanos": 0
    },
    "max_parallelism": 4,
    "critical_path_length": 3,
    "node_estimates": {...},
    "warnings": []
  },
  "id": 1
}
```

### 2. `resources.validate_availability`

**Purpose**: Check if system has sufficient resources for the graph

**Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "resources.validate_availability",
  "params": {
    "graph": {
      "id": "ml_training",
      "nodes": [...],
      "edges": [...]
    }
  },
  "id": 2
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "graph_id": "ml_training",
    "available": false,
    "gaps": [
      {
        "resource_type": "gpu_memory",
        "required": 17179869184,
        "available": 8589934592,
        "shortage": 8589934592,
        "suggestion": "Need 8 GB more GPU memory. Consider model quantization or sharding."
      }
    ],
    "warnings": [
      {
        "resource_type": "memory",
        "message": "High memory utilization predicted: 85% of available memory"
      }
    ],
    "system_capabilities": {
      "total_cpu_cores": 32,
      "available_cpu_cores": 24,
      "total_memory_bytes": 137438953472,
      "available_memory_bytes": 103079215104,
      "total_gpu_memory_bytes": 8589934592,
      "available_gpu_memory_bytes": 8589934592,
      "gpu_count": 1,
      "gpu_types": ["NVIDIA RTX 3090"]
    }
  },
  "id": 2
}
```

### 3. `resources.suggest_optimizations`

**Purpose**: Get optimization suggestions to improve performance or reduce resource usage

**Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "resources.suggest_optimizations",
  "params": {
    "graph": {
      "id": "ml_training",
      "nodes": [...],
      "edges": [...]
    }
  },
  "id": 3
}
```

**Response**:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "graph_id": "ml_training",
    "bottlenecks": [
      {
        "bottleneck_type": "sequential_execution",
        "affected_nodes": ["preprocess", "transform", "load"],
        "severity": 0.7,
        "description": "Sequential chain with 3 nodes could benefit from parallelization",
        "time_impact_secs": 180
      }
    ],
    "opportunities": [
      {
        "optimization_type": "parallelization",
        "affected_nodes": ["transform_a", "transform_b"],
        "priority": 9,
        "estimated_speedup": 2.0,
        "description": "Nodes transform_a and transform_b can run in parallel",
        "implementation_hint": "Split data and process in parallel workers"
      },
      {
        "optimization_type": "gpu_acceleration",
        "affected_nodes": ["matrix_multiply"],
        "priority": 8,
        "estimated_speedup": 10.0,
        "description": "Matrix operations would benefit from GPU acceleration",
        "implementation_hint": "Move computation to GPU for 10x speedup"
      }
    ],
    "estimated_improvement": {
      "time_reduction_secs": 2700,
      "memory_reduction_bytes": 0,
      "cost_reduction_percent": 35.0
    },
    "priority_order": ["parallelization", "gpu_acceleration"]
  },
  "id": 3
}
```

---

## Integration Patterns

### Pattern 1: Pre-Execution Planning

Use collaborative intelligence before executing workloads:

```python
client = ToadStoolClient()

# 1. Convert biomeOS neural API graph to ToadStool format
toadstool_graph = convert_neural_api_to_toadstool(biomeos_graph)

# 2. Estimate resources
estimate = client.call_rpc("resources.estimate", {"graph": toadstool_graph})
print(f"Estimated duration: {estimate['estimated_duration']['secs']}s")
print(f"Required GPU memory: {estimate['gpu_memory_bytes'] / (1024**3):.2f} GB")

# 3. Validate availability
availability = client.call_rpc("resources.validate_availability", {"graph": toadstool_graph})

if not availability["available"]:
    # 4. Get optimization suggestions
    suggestions = client.call_rpc("resources.suggest_optimizations", {"graph": toadstool_graph})
    
    # 5. Apply high-priority optimizations
    for opp in suggestions["opportunities"]:
        if opp["priority"] >= 8:
            print(f"Applying optimization: {opp['description']}")
            apply_optimization(biomeos_graph, opp)
    
    # 6. Re-validate after optimizations
    availability = client.call_rpc("resources.validate_availability", {"graph": toadstool_graph})

if availability["available"]:
    # 7. Execute the workload
    execute_graph(biomeos_graph)
else:
    # 8. Queue for later or request more resources
    queue_for_later(biomeos_graph, availability["gaps"])
```

### Pattern 2: Interactive Graph Design (with petalTongue)

Real-time resource feedback while designing graphs:

```python
class GraphDesigner:
    def __init__(self):
        self.client = ToadStoolClient()
        self.current_graph = None
    
    def on_node_added(self, node):
        """Called when user adds a node in petalTongue."""
        self.current_graph.add_node(node)
        self._update_estimates()
    
    def on_edge_added(self, edge):
        """Called when user connects nodes in petalTongue."""
        self.current_graph.add_edge(edge)
        self._update_estimates()
    
    def _update_estimates(self):
        """Update resource estimates in real-time."""
        toadstool_graph = convert_to_toadstool(self.current_graph)
        
        # Get estimates
        try:
            estimate = self.client.call_rpc("resources.estimate", {"graph": toadstool_graph})
            
            # Update UI with estimates
            self.ui.update_cpu_estimate(estimate["cpu_cores"])
            self.ui.update_memory_estimate(estimate["memory_bytes"] / (1024**3))
            self.ui.update_duration_estimate(estimate["estimated_duration"]["secs"])
            
            # Check availability in background
            availability = self.client.call_rpc("resources.validate_availability", {"graph": toadstool_graph})
            
            if not availability["available"]:
                # Show warnings in UI
                for gap in availability["gaps"]:
                    self.ui.show_warning(f"⚠️ {gap['resource_type']}: {gap['suggestion']}")
            
            # Get optimization suggestions
            suggestions = self.client.call_rpc("resources.suggest_optimizations", {"graph": toadstool_graph})
            
            # Show suggestions as hints
            for opp in suggestions["opportunities"][:3]:  # Top 3
                self.ui.show_hint(f"💡 {opp['description']}")
        
        except Exception as e:
            self.ui.show_error(f"Error estimating resources: {e}")
```

### Pattern 3: Adaptive Execution

Adjust execution strategy based on resource availability:

```python
def execute_with_adaptation(graph):
    client = ToadStoolClient()
    toadstool_graph = convert_to_toadstool(graph)
    
    # Check availability
    availability = client.call_rpc("resources.validate_availability", {"graph": toadstool_graph})
    
    if availability["available"]:
        # Resources available - execute normally
        return execute_graph(graph, mode="normal")
    
    # Resources not available - try optimizations
    suggestions = client.call_rpc("resources.suggest_optimizations", {"graph": toadstool_graph})
    
    # Prioritize by estimated speedup and availability
    best_optimization = None
    for opp in suggestions["opportunities"]:
        if opp["optimization_type"] == "reduce_parallelism":
            # Reduces resource requirements
            best_optimization = opp
            break
        elif opp["optimization_type"] == "streaming":
            # Reduces memory requirements
            best_optimization = opp
            break
    
    if best_optimization:
        # Apply optimization and retry
        apply_optimization(graph, best_optimization)
        return execute_with_adaptation(graph)  # Recursive retry
    else:
        # No suitable optimizations - queue for later
        return queue_for_later(graph, availability["gaps"])
```

---

## Error Handling

### JSON-RPC Errors

```python
def call_with_error_handling(client, method, params):
    try:
        result = client.call_rpc(method, params)
        return result
    except socket.error as e:
        # Connection failed
        print(f"Cannot connect to ToadStool: {e}")
        print("Is ToadStool running? Check: systemctl --user status toadstool")
        return None
    except json.JSONDecodeError as e:
        # Invalid JSON in response
        print(f"Invalid JSON response: {e}")
        return None
    except Exception as e:
        # JSON-RPC error or other
        if "JSON-RPC Error" in str(e):
            # Parse error details
            error_msg = str(e)
            if "Invalid graph" in error_msg:
                print("Graph validation failed - check for cycles or missing nodes")
            elif "Method not found" in error_msg:
                print("ToadStool version mismatch - update to 2.2.0+")
            else:
                print(f"RPC error: {error_msg}")
        else:
            print(f"Unexpected error: {e}")
        return None
```

### Common Error Codes

| Code | Message | Solution |
|------|---------|----------|
| -32600 | Invalid Request | Check JSON-RPC 2.0 format |
| -32601 | Method not found | Update ToadStool to 2.2.0+ |
| -32602 | Invalid params | Check graph structure |
| -32603 | Internal error | Check ToadStool logs |
| -32700 | Parse error | Invalid JSON syntax |

### Validation Errors

```python
# Handle graph validation errors
availability = client.call_rpc("resources.validate_availability", {"graph": graph})

if "error" in availability:
    error_type = availability["error"]["data"]["error_type"]
    
    if error_type == "CycleDetected":
        print("Graph has cycles - check dependencies")
        cycle_path = availability["error"]["data"]["cycle_path"]
        print(f"Cycle: {' -> '.join(cycle_path)}")
    
    elif error_type == "InvalidEdge":
        print("Edge references non-existent node")
        print(f"Edge: {availability['error']['data']['from']} -> {availability['error']['data']['to']}")
    
    elif error_type == "EmptyGraph":
        print("Graph has no nodes")
```

---

## Best Practices

### 1. Cache Estimates for Similar Graphs

```python
import hashlib
import json

class CachedToadStoolClient:
    def __init__(self):
        self.client = ToadStoolClient()
        self.cache = {}
    
    def _graph_hash(self, graph):
        """Create hash of graph structure for caching."""
        graph_str = json.dumps(graph, sort_keys=True)
        return hashlib.sha256(graph_str.encode()).hexdigest()
    
    def estimate_with_cache(self, graph, ttl=300):
        """Estimate with caching (TTL in seconds)."""
        graph_hash = self._graph_hash(graph)
        
        if graph_hash in self.cache:
            cached_result, cached_time = self.cache[graph_hash]
            if time.time() - cached_time < ttl:
                return cached_result
        
        # Not in cache or expired - fetch new estimate
        result = self.client.call_rpc("resources.estimate", {"graph": graph})
        self.cache[graph_hash] = (result, time.time())
        return result
```

### 2. Batch Multiple Graphs

```python
def estimate_multiple_graphs(graphs):
    """Estimate resources for multiple graphs efficiently."""
    client = ToadStoolClient()
    results = []
    
    # Process in parallel (if ToadStool supports concurrent connections)
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as executor:
        futures = [
            executor.submit(client.call_rpc, "resources.estimate", {"graph": g})
            for g in graphs
        ]
        
        for future in concurrent.futures.as_completed(futures):
            try:
                result = future.result()
                results.append(result)
            except Exception as e:
                print(f"Error estimating graph: {e}")
                results.append(None)
    
    return results
```

### 3. Handle Warnings Proactively

```python
def check_warnings(availability):
    """Check and handle warnings from availability check."""
    for warning in availability.get("warnings", []):
        resource_type = warning["resource_type"]
        message = warning["message"]
        
        if resource_type == "memory" and "80%" in message:
            # High memory usage - consider streaming or chunking
            print("⚠️ High memory usage predicted - enabling streaming mode")
            enable_streaming_mode()
        
        elif resource_type == "cpu" and "90%" in message:
            # Very high CPU usage - reduce parallelism
            print("⚠️ Very high CPU usage - reducing parallelism")
            reduce_parallelism()
        
        elif resource_type == "gpu":
            # GPU warning - consider fallback to CPU
            print("⚠️ GPU resource warning - preparing CPU fallback")
            prepare_cpu_fallback()
```

### 4. Graceful Degradation

```python
def execute_with_fallback(graph):
    """Execute graph with graceful degradation."""
    client = ToadStoolClient()
    
    try:
        # Try full resource request
        availability = client.call_rpc("resources.validate_availability", {"graph": graph})
        
        if availability["available"]:
            return execute_graph(graph, mode="optimal")
        
        # Not available - try degraded mode
        print("Optimal resources not available - trying degraded mode")
        
        # Get optimizations that reduce resource usage
        suggestions = client.call_rpc("resources.suggest_optimizations", {"graph": graph})
        
        for opp in suggestions["opportunities"]:
            if opp["optimization_type"] in ["reduce_parallelism", "streaming", "quantization"]:
                apply_optimization(graph, opp)
                
                # Re-check availability
                availability = client.call_rpc("resources.validate_availability", {"graph": graph})
                if availability["available"]:
                    return execute_graph(graph, mode="degraded")
        
        # Still not available - queue for later
        return queue_for_later(graph)
    
    except Exception as e:
        print(f"Error in resource planning: {e}")
        # Fallback: execute with best-effort resource allocation
        return execute_graph(graph, mode="best_effort")
```

---

## Troubleshooting

### Problem: Cannot connect to Unix socket

**Solution**:
```bash
# Check if ToadStool is running
systemctl --user status toadstool

# Check socket exists
ls -la /run/user/1000/toadstool-*.sock

# Check permissions
# Socket should be owned by your user with 0600 permissions

# Check ToadStool logs
journalctl --user -u toadstool -f
```

### Problem: "Method not found" error

**Solution**: Update ToadStool to version 2.2.0 or later:
```bash
# Check ToadStool version
toadstool --version

# Update ToadStool
cd /path/to/toadStool
git pull
cargo build --release
systemctl --user restart toadstool
```

### Problem: Graph validation fails with unclear error

**Solution**: Enable debug logging in ToadStool:
```bash
# Set RUST_LOG environment variable
export RUST_LOG=toadstool_server=debug

# Restart ToadStool
systemctl --user restart toadstool

# Check logs for detailed validation errors
journalctl --user -u toadstool -f
```

### Problem: Estimates seem inaccurate

**Solution**: Provide more accurate duration estimates in your graphs:
```python
# Use historical data for duration estimates
node = {
    "id": "training",
    "operation": "gpu_compute",
    "requirements": {...},
    "duration": get_historical_duration("training", dataset_size),  # In seconds
    "metadata": {
        "dataset_size": str(dataset_size),
        "model_type": "transformer"
    }
}
```

---

## Conclusion

ToadStool's Collaborative Intelligence API enables biomeOS to make intelligent decisions about workload placement and resource allocation. The Unix socket + JSON-RPC 2.0 architecture provides secure, high-performance communication between primals.

### Key Takeaways

✅ **Use Unix sockets** for secure, low-latency communication  
✅ **Validate graphs** before execution to catch errors early  
✅ **Handle resource gaps** gracefully with optimizations or queuing  
✅ **Cache estimates** for similar graphs to reduce overhead  
✅ **Provide accurate durations** for better resource planning

### Next Steps

1. **Integrate** the `ToadStoolClient` into biomeOS
2. **Convert** neural API graphs to ToadStool format
3. **Test** with sample workloads
4. **Monitor** estimation accuracy and adjust
5. **Optimize** based on suggestions

For more information:
- [API Specification](../specs/COLLABORATIVE_INTELLIGENCE_RESOURCE_PLANNING.md)
- [Usage Examples](./COLLABORATIVE_INTELLIGENCE_EXAMPLES.md)
- [Implementation Tracker](../COLLABORATIVE_INTELLIGENCE_TRACKER.md)

Different orders of the same architecture. 🍄🐸

