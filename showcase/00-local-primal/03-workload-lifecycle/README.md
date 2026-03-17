# Workload Lifecycle

ToadStool showcase demo: workload lifecycle simulation (submit → status → result → cancel).

## What It Demonstrates

- **compute.submit**: JSON-RPC 2.0 request with job_type, data, gpu_hint
- **compute.status**: Check job status (running)
- **compute.result**: Retrieve completed result data
- **compute.cancel**: Cancel a different job

All requests/responses use the JSON-RPC 2.0 format (jsonrpc, method, params, id).

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

- Banner: "ToadStool Showcase: Workload Lifecycle"
- compute.submit: request + response with job_id (UUID)
- compute.status: request + response (status: "running")
- compute.result: request + response (status: "completed", result data)
- compute.cancel: request + response (status: "cancelled")
- Summary: "Full workload lifecycle demonstrated"

## Prerequisites

- Rust 1.85+

## ToadStool Capabilities Shown

| Crate | Capability |
|-------|------------|
| toadstool-common | Types, UUID generation (via uuid crate) |
| serde_json | JSON-RPC 2.0 request/response construction |
| uuid | Job ID generation |

This demo simulates the data structures and flow used by toadStool's compute orchestration — no server required.
