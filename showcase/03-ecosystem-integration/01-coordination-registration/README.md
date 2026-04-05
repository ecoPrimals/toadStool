# SongBird Capability Registration

ToadStool showcase demo: Registering toadStool's compute capabilities with songBird for cross-tower discovery.

## What It Demonstrates

- **ToadStool Capabilities**: All capabilities toadStool advertises (compute, gpu, wasm, container, science, shader, ecology, discovery, deploy, hardware_transport, orchestration, ai_local)
- **Registration Request**: JSON-RPC `coordination.register` request toadStool sends to songBird
- **SongBird Discovery**: Socket check at `$XDG_RUNTIME_DIR/biomeos/coordination.sock`
- **Cross-Tower Scenario**: How songBird federates capability registration across towers (tarpc multiplexing)
- **Health Registration**: `discovery.primal_health` request songBird periodically calls

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

- Banner: "ToadStool Showcase: SongBird Capability Registration"
- ToadStool capabilities list
- Registration request (formatted JSON)
- SongBird socket found/not-found
- Cross-tower scenario explanation
- Health registration request/response
- Summary: "SongBird registration demonstrated — N capabilities registered"

## Prerequisites

- Rust 1.85+
- songBird optional (demo shows simulated responses when socket unavailable)
