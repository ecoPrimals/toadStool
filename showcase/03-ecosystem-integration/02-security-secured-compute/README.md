# BearDog-Secured Compute

ToadStool showcase demo: Signed workload submission using bearDog-provided security tokens.

## What It Demonstrates

- **Security Flow**: Client → bearDog (auth) → toadStool (submit with token) → bearDog (validate) → execute
- **Authentication**: JSON-RPC `security.authenticate` request to bearDog
- **Secured Workload Submission**: `compute.submit` with bearer token
- **Zero-Trust Validation**: toadStool validates token with bearDog before execution
- **BearDog Socket Check**: Socket at `$XDG_RUNTIME_DIR/biomeos/security.sock`
- **Standalone Fallback**: Without bearDog, toadStool operates in standalone mode (no auth)

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

- Banner: "ToadStool Showcase: BearDog-Secured Compute"
- Security flow explanation
- Authentication request/response
- Secured workload submission request
- Zero-trust validation request/response
- BearDog socket found/not-found
- Standalone fallback explanation
- Summary: "BearDog-secured compute demonstrated — zero-trust validation"

## Prerequisites

- Rust 1.85+
- bearDog optional (demo shows simulated responses when socket unavailable)
