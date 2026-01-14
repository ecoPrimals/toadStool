# ToadStool Universal Compute Runtime

Universal compute runtime providing unified abstraction over CPU, GPU, and future compute paradigms.

## Features

- **CPU**: Native CPU compute with automatic parallelization
- **OpenCL**: Cross-platform GPU compute
- **WebGPU**: Modern, safe GPU compute abstraction
- **Capability Discovery**: Runtime detection of available compute resources

## Usage

```rust
use toadstool_runtime_universal::{UniversalRuntime, ComputeCapability};

// Discover available compute capabilities at runtime
let runtime = UniversalRuntime::discover()?;
let capabilities = runtime.available_capabilities();

// Execute workload on best available backend
runtime.execute(workload, capabilities).await?;
```

## Deep Debt Compliance

This runtime follows ToadStool's Deep Debt principles:
- **No Hardcoding**: All capabilities discovered at runtime
- **Self-Knowledge**: Runtime only knows its own capabilities
- **Zero Vendor Lock-in**: Backend-agnostic compute abstraction
