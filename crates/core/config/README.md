# toadstool-config

**Zero-hardcoding hierarchical configuration system for ToadStool**

Provides a sophisticated, layered configuration management system with environment-based overrides, validation, and hot-reloading support.

## Features

- **Zero Hardcoding**: All configuration is externalized and discoverable
- **Hierarchical Merging**: Defaults → File → Environment → Runtime overrides
- **Type Safety**: Strongly-typed configuration with validation
- **Hot Reloading**: Watch for configuration changes (optional)
- **Environment Agnostic**: Works across dev, staging, production
- **Capability Discovery**: Integration with primal discovery system

## Quick Start

```rust
use toadstool_config::{Config, ConfigBuilder};

// Load configuration with hierarchical merging
let config = ConfigBuilder::new()
    .with_defaults()
    .with_file("config.toml")?
    .with_env_overrides()
    .build()?;

// Access configuration
let port = config.network.port;
let workers = config.execution.max_workers;
```

## Configuration Sources

Configuration is loaded and merged in order (later sources override earlier):

1. **Built-in Defaults** - Sensible fallbacks
2. **Configuration Files** - TOML/YAML/JSON support
3. **Environment Variables** - `TOADSTOOL_*` prefix
4. **Runtime Overrides** - Programmatic configuration

### Example Configuration

```toml
[network]
host = "0.0.0.0"
port = 3000

[execution]
max_workers = 4
timeout_seconds = 300

[discovery]
method = "mdns"  # or "file", "env"
enabled = true
```

## Environment Variables

Override any configuration via environment:

```bash
export TOADSTOOL_NETWORK_PORT=8080
export TOADSTOOL_EXECUTION_MAX_WORKERS=8
export TOADSTOOL_DISCOVERY_METHOD=file
```

## Validation

Configuration is validated at load time:

```rust
use toadstool_config::Config;

let config = Config::load("config.toml")?;
// Validation happens automatically:
// - Port in valid range (1-65535)
// - Paths exist and are accessible
// - Resource limits are reasonable
```

## Architecture Principles

This crate embodies ToadStool's **zero-hardcoding** principle:

- ✅ **No hardcoded URLs/ports** - All externalized
- ✅ **Self-knowledge only** - Primals discover each other at runtime
- ✅ **Capability-based** - Services advertise what they can do
- ✅ **Environment-agnostic** - Same code, different config

## Implementation Quality

- ✅ Zero `.unwrap()` calls in production
- ✅ Comprehensive error handling
- ✅ Extensive validation
- ✅ Well-tested (unit + integration)

## Usage

```toml
[dependencies]
toadstool-config = "0.1"
```

## Documentation

```bash
cargo doc --open --package toadstool-config
```

## License

AGPL-3.0-or-later

