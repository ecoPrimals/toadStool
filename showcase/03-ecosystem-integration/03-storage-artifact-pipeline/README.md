# NestGate Artifact Storage

ToadStool showcase demo: Storing and retrieving compute artifacts (models, shader binaries) via nestGate.

## What It Demonstrates

- **Storage Flow**: Compile/complete → store in nestGate → retrieve cached artifacts
- **Store Artifact**: JSON-RPC `storage.artifact.store` with metadata
- **Retrieve Artifact**: JSON-RPC `storage.artifact.retrieve` by artifact_id
- **Artifact Types**: Compiled shaders, model weights, benchmark results, job outputs
- **NestGate Socket Check**: Socket at `$XDG_RUNTIME_DIR/biomeos/storage.sock`
- **ZFS Integration**: Automatic compression, deduplication, snapshot-based versioning

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

- Banner: "ToadStool Showcase: NestGate Artifact Storage"
- Storage flow explanation
- Store artifact request/response
- Retrieve artifact request/response
- Artifact types list
- NestGate socket found/not-found
- ZFS integration explanation
- Summary: "NestGate artifact storage demonstrated — persistent compute artifacts"

## Prerequisites

- Rust 1.85+
- nestGate optional (demo shows simulated responses when socket unavailable)
