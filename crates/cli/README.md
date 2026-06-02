# toadstool-cli

**ToadStool CLI - Universal Compute Command Center for Sovereign Science**

A powerful command-line interface for managing ToadStool workloads, monitoring resources, and orchestrating distributed compute.

## Installation

Production binaries are distributed via **plasmidBin** — the NUCLEUS binary depot:

```bash
# plasmidBin auto-harvests from CI on every push to main.
# Binary lands at /opt/toadstool/bin/toadstool (includes CLI).
```

For development builds:

```bash
cargo build -p toadstool-cli
# Binary: target/debug/toadstool
```

## Quick Start

```bash
# Initialize ToadStool
toadstool init

# Execute a workload
toadstool exec --runtime native ./my_app

# Execute WASM module
toadstool exec --runtime wasm module.wasm

# GPU compute (sovereign wgpu path; enable `gpu` feature)
toadstool exec --runtime gpu shader.wgsl

# Monitor resources
toadstool status

# List available runtimes
toadstool runtimes
```

## Commands

### Execution

```bash
# Execute with specific runtime
toadstool exec --runtime <RUNTIME> <PATH>

# With resource limits
toadstool exec --runtime native --memory 2GB --cpu 2.0 ./app

# With timeout
toadstool exec --runtime wasm --timeout 5m module.wasm

# Sandboxed execution
toadstool exec --runtime wasm --sandbox module.wasm
```

### Resource Management

```bash
# Show current resource usage
toadstool status

# Show detailed metrics
toadstool status --verbose

# Monitor in real-time
toadstool monitor

# List active workloads
toadstool ps
```

### Configuration

```bash
# Show current configuration
toadstool config show

# Edit configuration
toadstool config edit

# Validate configuration
toadstool config validate

# Reset to defaults
toadstool config reset
```

### Discovery

```bash
# Discover ecosystem services
toadstool discover

# Show primal capabilities
toadstool primals

# Test connectivity
toadstool ping <SERVICE>
```

## Configuration

Default config location: `~/.config/toadstool/config.toml`

```toml
[execution]
default_runtime = "native"
max_workers = 4
timeout_seconds = 300

[network]
port = 3000
host = "0.0.0.0"

[discovery]
method = "mdns"
enabled = true
```

## Environment Variables

Override configuration via environment:

```bash
export TOADSTOOL_NETWORK_PORT=8080
export TOADSTOOL_EXECUTION_MAX_WORKERS=8
export TOADSTOOL_DISCOVERY_METHOD=file
```

## Examples

### Execute Native Binary

```bash
toadstool exec --runtime native ./my_compiled_app
```

### GPU Compute with Monitoring

```bash
toadstool exec --runtime gpu \
    --monitor \
    --memory 4GB \
    shader.wgsl
```

### Secure Enclave (Zero-Knowledge Compute)

```bash
toadstool exec --runtime secure-enclave \
    --encrypted \
    --proof-of-isolation \
    sensitive_computation.wasm
```

## Features

- **Universal Execution**: Support for all ToadStool runtimes
- **Resource Management**: Built-in monitoring and limits
- **Ecosystem Integration**: Seamless primal discovery
- **Zero Hardcoding**: All configuration externalized
- **User-Friendly**: Intuitive commands with helpful output

## Architecture

The CLI demonstrates ToadStool's principles:

- ✅ **Zero `.unwrap()`**: Proper error handling with user-friendly messages
- ✅ **Modern Rust**: Idiomatic CLI patterns with `clap`
- ✅ **Rich Output**: Beautiful terminal UI with progress bars
- ✅ **Capability Discovery**: No hardcoded service URLs

## Development

```bash
# Run CLI locally
cargo run -- exec --runtime native ./app

# Run tests
cargo test

# Check code quality
cargo clippy -- -D warnings
```

## Documentation

For detailed API usage:

```bash
toadstool help
toadstool help exec
```

## License

AGPL-3.0-only

---

*Built with ❤️ for sovereignty and human dignity.*

