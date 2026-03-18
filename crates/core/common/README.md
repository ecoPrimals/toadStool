# toadstool-common

**Common utilities and types for ToadStool**

This crate provides shared functionality used across the ToadStool universal compute platform, including error types, discovery mechanisms, and core utilities.

## Features

- **Error Handling**: Comprehensive 3-tier error system with structured error types
- **Error Codes**: Hierarchical error code system for programmatic handling
- **Primal Discovery**: Runtime capability-based service discovery
- **Core Types**: Shared data structures and utilities

## Error System

The error system provides a structured, 3-tier hierarchy:

```rust
use toadstool_common::error::{ToadStoolError, ToadStoolResult};

fn example() -> ToadStoolResult<String> {
    // Tier 1: Top-level domain errors
    Err(ToadStoolError::execution("Runtime failed"))
}
```

### Error Tiers

1. **Tier 1**: `ToadStoolError` - Top-level categorization
2. **Tier 2**: Domain-specific errors (`ExecutionError`, `ConfigError`, etc.)
3. **Tier 3**: Result type aliases for convenience

### Error Codes

```rust
use toadstool_common::error_codes::codes;
use toadstool_common::error::ToadStoolErrorExt;

let error = ToadStoolError::runtime("Failed to initialize")
    .with_code(codes::EXEC_RUNTIME_001);

println!("Error code: {}", error.error_code_str().unwrap());
println!("Remediation: {}", error.remediation().unwrap());
```

## Primal Discovery

Zero-configuration service discovery based on capabilities:

```rust
use toadstool_common::primal_discovery::PrimalDiscovery;

let discovery = PrimalDiscovery::new().await?;
let services = discovery.discover_by_capability("compute").await?;
```

## Architecture

This crate follows **deep debt solutions** and **modern idiomatic Rust**:

- ✅ Zero `.unwrap()` in production code
- ✅ Comprehensive error handling with `Result<T, E>`
- ✅ SAFETY documentation on all unsafe blocks
- ✅ Extensive testing (unit + integration)

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
toadstool-common = "0.1"
```

## Documentation

For full API documentation, run:

```bash
cargo doc --open --package toadstool-common
```

## License

AGPL-3.0-or-later (same as ToadStool parent project)

