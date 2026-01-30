# akida-models

**Pure Rust parser for Akida neural network models**

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

## Overview

This crate provides parsing and loading capabilities for Akida `.fbz` model files, enabling pure Rust manipulation of neuromorphic neural networks.

## Features

- ✅ FlatBuffers binary parsing
- ✅ Model metadata extraction
- ✅ Layer information
- 🚧 Weight data extraction (in progress)
- 🚧 Model loading to device (in progress)

## Format

Akida models use FlatBuffers binary format:

```
┌─────────────────────────┐
│ FlatBuffers Header      │  Magic: \x80D\x04\x10
├─────────────────────────┤
│ Version String          │  "2.18.2"
├─────────────────────────┤
│ Model Metadata          │
├─────────────────────────┤
│ Layer Definitions       │
├─────────────────────────┤
│ Weight Data             │  Quantized parameters
└─────────────────────────┘
```

## Quick Start

```rust
use akida_models::prelude::*;

fn main() -> Result<()> {
    // Load model from file
    let model = Model::from_file("model.fbz")?;
    
    println!("Model version: {}", model.version());
    println!("Layers: {}", model.layer_count());
    
    // Inspect layers
    for layer in model.layers() {
        println!("{}: {}", layer.name, layer.layer_type);
    }
    
    Ok(())
}
```

## Examples

```bash
# Parse a model file
cargo run --example parse_fbz /path/to/model.fbz

# Or use default test model
cargo run --example parse_fbz
```

## Dependencies

- `nom` - Parser combinators for binary parsing
- `byteorder` - Endian-aware reading
- `thiserror` - Error handling
- `tracing` - Logging

## Status

**Phase 2**: Model format analysis in progress

- [x] FlatBuffers header parsing
- [x] Version extraction
- [x] Layer name extraction
- [ ] Complete schema recovery
- [ ] Weight parsing
- [ ] Device loading

See [PURE_RUST_AKIDA_MIGRATION_PLAN.md](../../showcase/neuromorphic/PURE_RUST_AKIDA_MIGRATION_PLAN.md) for roadmap.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../../LICENSE-APACHE))
- MIT license ([LICENSE-MIT](../../../LICENSE-MIT))

at your option.
