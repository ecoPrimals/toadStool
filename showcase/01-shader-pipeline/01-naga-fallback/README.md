# Naga Shader Fallback

ToadStool showcase demo: WGSL shader compilation using naga (toadStool's built-in fallback when coralReef is absent).

## What It Demonstrates

- **WGSL Source**: A simple compute shader that doubles values in a storage buffer
- **Compilation Request**: JSON-RPC 2.0 `shader.compile.wgsl` over Unix socket
- **Naga Compilation**: When coralReef is unavailable, toadStool uses naga for WGSL → SPIR-V
- **Socket Check**: Graceful handling when toadStool server is not running

## How to Run

```bash
./demo.sh
```

Or manually:

```bash
cargo build --release
cargo run --release
```

## Expected Output

- Banner: "ToadStool Showcase: Naga Shader Fallback"
- WGSL shader source
- JSON-RPC compilation request
- Simulated naga response (status, backend, spirv_size_bytes, entry_points)
- Socket check: "Server not running" (or "Live server detected" if toadStool is running)
- Summary: "Naga fallback compilation demonstrated — no coralReef needed"

## Prerequisites

- Rust 1.85+
- toadStool server optional (demo works standalone with simulated responses)
