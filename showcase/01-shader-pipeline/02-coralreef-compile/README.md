# coralReef Shader Compilation

ToadStool showcase demo: Shader compilation via coralReef when available, with graceful fallback to naga.

## What It Demonstrates

- **Discovery**: Check for coralReef at $CORALREEF_URL, $XDG_RUNTIME_DIR/ecoPrimals/coralreef-core.json, $XDG_RUNTIME_DIR/biomeos/coralreef.sock
- **WGSL Source**: Same compute shader as naga-fallback demo
- **Compilation Request**: JSON-RPC `shader.compile.wgsl`
- **Compilation Response**: coralReef path vs naga fallback with pipeline explanation
- **SPIR-V Compilation**: `shader.compile.spirv` request format for Vulkan target

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

- Banner: "ToadStool Showcase: coralReef Shader Compilation"
- Discovery results (found/not found) for each location
- WGSL shader source
- JSON-RPC compilation request
- Response (coralReef forward or naga fallback with pipeline explanation)
- SPIR-V compile request format
- Summary: "Shader pipeline: WGSL -> [coralReef | naga] -> SPIR-V -> native"

## Prerequisites

- Rust 1.82+
- coralReef optional (demo shows fallback when absent)
