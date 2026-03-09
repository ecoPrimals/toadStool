# Deploy Graph — Capability Routing

ToadStool showcase demo: Capability-based routing via deploy.capability_call to route compute to barraCuda.

## What It Demonstrates

- **Architecture**: toadStool decides WHERE; barraCuda decides WHAT
- **Capability Call**: `deploy.capability_call` routing to barraCuda's compute.sock
- **Routing Decision**: Resource check, substrate capabilities, target selection
- **Graph Status**: `deploy.graph_status` for active compute graph
- **Socket Check**: Connection attempt to barraCuda's compute.sock

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

- Banner: "ToadStool Showcase: Deploy Graph — Capability Routing"
- Architecture demarcation
- Capability call request
- Routing decision tree
- Graph status request/response
- Socket check (available or simulated)
- Summary: "Deploy graph routing demonstrated — toadStool WHERE, barraCuda WHAT"

## Prerequisites

- Rust 1.82+
- Primals optional (demo shows simulated responses when sockets unavailable)
