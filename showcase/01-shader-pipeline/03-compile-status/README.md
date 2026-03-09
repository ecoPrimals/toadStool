# Compilation Status Polling

ToadStool showcase demo: Async shader compilation status polling pattern.

## What It Demonstrates

- **Submit Compilation**: `shader.compile.wgsl` request that returns a compilation_id (UUID)
- **Status Polling**: `shader.compile.status` with `{ "compilation_id": "<uuid>" }`
- **Polling Cycles**: Simulated 3 polls (compiling 0.3 → 0.7 → completed 1.0)
- **Capabilities**: `shader.compile.capabilities` response (backends, input_formats, target_formats)

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

- Banner: "ToadStool Showcase: Compilation Status Polling"
- Submit compilation request and returned compilation_id
- 3 polling cycles with 200ms delay between each
- Capabilities response (backends, input_formats, target_formats)
- Summary: "Async compilation polling demonstrated — 3 status checks"

## Prerequisites

- Rust 1.82+
