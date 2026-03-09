# Hello Compute

ToadStool showcase demo: primal identity, build info, and compute capabilities.

## What It Demonstrates

- **Primal identity**: Canonical name from `toadstool_common::constants::PRIMAL_NAME`
- **Version info**: Demo crate version
- **Interned capabilities**: All capability constants (security, crypto, storage, compute, etc.)
- **Interned primal names**: Legacy primal identifiers (beardog, songbird, nestgate, squirrel, toadstool)
- **CPU info**: Core count and brand via `toadstool_sysmon`
- **Memory info**: Total, available, and used memory
- **Load average**: 1/5/15 minute system load

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

- Banner: "ToadStool Showcase: Hello Compute"
- Primal name: toadstool
- Version: 0.1.0
- List of 16 capabilities
- List of 5 primal names
- CPU cores and brand
- Memory stats (total/available/used)
- Load average (1/5/15 min)
- Summary: "toadStool is ready for compute orchestration"

## Prerequisites

- Rust 1.82+ (or 1.80+ per workspace)
- Linux (sysmon reads `/proc`)

## ToadStool Capabilities Shown

| Crate | Capability |
|-------|------------|
| toadstool-common | constants, interned_strings, format_bytes |
| toadstool-sysmon | cpu_count, cpu_brand, memory_info, load_average |
