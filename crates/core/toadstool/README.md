# toadstool

**ToadStool Universal Compute Platform - Core Library**

The main entry point for the ToadStool universal compute platform, providing unified access to execution environments, resource management, and security sandboxing.

## Overview

ToadStool enables **universal compute**: run workloads across Native, WASM, Container, Python, and GPU runtimes with a single unified API. Perfect for polyglot applications, edge computing, and privacy-preserving execution.

## Quick Start

```rust
use toadstool::{ToadStool, WorkloadRequest, RuntimeType};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize ToadStool
    let toadstool = ToadStool::new().await?;
    
    // Execute a workload
    let request = WorkloadRequest {
        runtime: RuntimeType::Native,
        code: include_bytes!("my_app"),
        // ...
    };
    
    let result = toadstool.execute(request).await?;
    println!("Result: {:?}", result);
    
    Ok(())
}
```

## Supported Runtimes

- **Native**: Compiled binaries (fastest)
- **WASM**: WebAssembly (portable, sandboxed)
- **Container**: Docker/Podman (isolated environments)
- **Python**: Python scripts (with dependency management)
- **GPU**: CUDA/OpenCL compute (high-performance)
- **Secure Enclave**: Zero-knowledge compute (privacy-preserving)

## Key Features

### Universal Execution

Run any workload type with the same API:

```rust
// Native binary
toadstool.execute_native(binary).await?;

// WASM module
toadstool.execute_wasm(wasm_bytes).await?;

// Python script
toadstool.execute_python(script, deps).await?;

// GPU compute
toadstool.execute_gpu(kernel).await?;
```

### Resource Management

Automatic resource allocation and monitoring:

```rust
let config = ResourceConfig {
    max_memory: "2GB",
    max_cpu: 2.0,
    timeout: "5m",
};

toadstool.execute_with_limits(request, config).await?;
```

### Security Sandboxing

Built-in sandboxing for untrusted code:

```rust
let sandbox = SandboxConfig {
    network: false,
    filesystem: ReadOnly("/data"),
    memory_limit: "512MB",
};

toadstool.execute_sandboxed(request, sandbox).await?;
```

### Primal Ecosystem Integration

ToadStool integrates seamlessly with the primal ecosystem:

- **NestGate**: Compression (88% savings)
- **BearDog**: Encryption (AES-256-GCM)
- **Songbird**: Secure communication (BTSP)

## Architecture

ToadStool follows **capability-based discovery**:

- ✅ **Zero Hardcoding**: Services discover each other at runtime
- ✅ **Self-Knowledge**: Each primal knows only itself
- ✅ **Capability-Based**: Services advertise their capabilities
- ✅ **Environment-Agnostic**: Works anywhere

## Examples

See the `examples/` directory for complete examples:

- `examples/hello_world.rs` - Basic execution
- `examples/multi_runtime.rs` - Using multiple runtimes
- `examples/secure_compute.rs` - Zero-knowledge execution
- `examples/gpu_compute.rs` - GPU-accelerated workloads

## Quality

This crate demonstrates **modern idiomatic Rust**:

- ✅ Zero `.unwrap()` in production
- ✅ Comprehensive error handling
- ✅ SAFETY docs on all unsafe blocks
- ✅ Extensive test coverage
- ✅ Deep solutions, not superficial wrappers

## Installation

```toml
[dependencies]
toadstool = "0.1"
```

### Feature Flags

```toml
[dependencies]
toadstool = { version = "0.1", features = ["full-ecosystem"] }
```

Available features:
- `websocket` - WebSocket support
- `full-ecosystem` - All ecosystem integrations
- `wgpu` - GPU compute via wgpu

## Documentation

```bash
cargo doc --open --package toadstool
```

## Contributing

See the main [ToadStool repository](https://github.com/your-org/toadstool) for contribution guidelines.

## License

AGPL-3.0-or-later

---

*Part of the ToadStool universal compute platform. Built with ❤️ for sovereignty and human dignity.*

