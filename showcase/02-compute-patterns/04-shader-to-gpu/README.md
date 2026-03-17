# Shader to GPU — The Compute Triangle

ToadStool showcase demo: The full compute triangle — coralReef compiles shader → toadStool dispatches → barraCuda executes.

## What It Demonstrates

- **Step 1**: WGSL compute shader for vector addition
- **Step 2**: coralReef compilation (shader.compile.wgsl) via toadStool proxy
- **Step 3**: toadStool routing (resource check, substrate selection, deploy.capability_call)
- **Step 4**: barraCuda execution on GPU via wgpu
- **Full Pipeline**: End-to-end flow from shader to result
- **Live Status**: Socket availability for toadStool, coralReef, barraCuda

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

- Banner: "ToadStool Showcase: The Compute Triangle"
- ASCII art showing coralReef → toadStool → barraCuda
- WGSL shader source
- Compile request/response
- Dispatch request
- Execute result with timing and throughput
- Full pipeline summary (6 steps)
- Live status for each primal socket
- Summary: "Full compute triangle demonstrated — compile, dispatch, execute"

## Prerequisites

- Rust 1.85+
- Primals optional (demo shows simulated responses when sockets unavailable)
