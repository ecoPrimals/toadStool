# Capability-Based Discovery

ToadStool showcase demo: Runtime discovery of compute primals via capability-based socket lookup.

## What It Demonstrates

- **Socket Discovery**: Check for primal sockets at `$XDG_RUNTIME_DIR/biomeos/{name}.sock`
- **Discovery Protocol**: JSON-RPC `discovery.primals` request and response
- **Topology**: `discovery.topology` request showing nodes and edges of the compute triangle
- **Compute Triangle**: coralReef (compile) → toadStool (orchestrate) → barraCuda (execute)

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

- Banner: "ToadStool Showcase: Capability-Based Discovery"
- Socket discovery: found/not-found status for each primal (toadStool, barraCuda, coralReef, songBird, bearDog, nestGate)
- Discovery protocol request/response
- Topology request/response
- ASCII diagram of the compute triangle
- Summary: "N/6 primals discovered on this host"

## Prerequisites

- Rust 1.82+
- Primals optional (demo shows simulated responses when sockets unavailable)
