# akida-driver

**Pure Rust driver for BrainChip Akida neuromorphic processors**

[![License](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)

## Overview

This crate provides direct, safe access to Akida AKD1000/AKD1500 neuromorphic processors via the kernel driver at `/dev/akida*`.

### Design Principles

- **Zero Mocks**: Production code only, mocks isolated to tests
- **Capability-Based**: Devices discovered at runtime, no hardcoding
- **Safe Rust**: Minimal unsafe, fully encapsulated and documented
- **Idiomatic**: Modern Rust patterns, ergonomic API
- **Observable**: Comprehensive tracing for debugging

## Features

- ✅ Runtime device discovery
- ✅ Capability querying via sysfs
- ✅ Direct DMA transfers (read/write)
- 🚧 Model loading (in progress)
- 🚧 Inference execution (in progress)

## Requirements

- Linux kernel with `akida_pcie` driver loaded
- Akida PCIe hardware installed
- `/dev/akida*` devices accessible

## Quick Start

```rust
use akida_driver::prelude::*;

fn main() -> Result<()> {
    // Discover devices at runtime
    let manager = DeviceManager::discover()?;
    println!("Found {} device(s)", manager.device_count());

    // Open first device
    let mut device = manager.open_first()?;
    
    // Perform I/O
    device.write(&data)?;
    device.read(&mut buffer)?;
    
    Ok(())
}
```

## Examples

```bash
# Enumerate all devices
cargo run --example enumerate_devices

# Test basic I/O
cargo run --example basic_io

# Query device information
cargo run --example device_info
```

## Architecture

```
┌─────────────────────────────────────┐
│      akida-driver (Pure Rust)       │
│                                     │
│  ┌──────────┐  ┌────────────────┐  │
│  │Discovery │  │ Device Handle  │  │
│  │(Runtime) │  │   (Safe I/O)   │  │
│  └──────────┘  └────────────────┘  │
└─────────────────────────────────────┘
           │              │
      [sysfs]        [read/write]
           │              │
           v              v
    ┌──────────┐   ┌─────────────┐
    │ PCIe Bus │   │ /dev/akida* │
    └──────────┘   └─────────────┘
                          │
                          v
                   ┌─────────────┐
                   │ akida_pcie  │
                   │ (C driver)  │
                   └─────────────┘
```

## Testing

```bash
# Run tests (requires hardware)
cargo test

# Run with tracing
RUST_LOG=akida_driver=trace cargo test -- --nocapture
```

## Status

**Current**: Phase 1 - Basic I/O working  
**Next**: Phase 2 - Protocol analysis and model loading

See `ecoPrimals/infra/wateringHole/fossilRecord/` for the fossilized pure Rust migration roadmap.

## License

Licensed under AGPL-3.0-only. See [LICENSE](../../../LICENSE) for details.

This copyleft license ensures all derivative works remain open source, preventing corporate extend-and-extinguish tactics.
