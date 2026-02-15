# Deep Debt Evolution - February 15, 2026

## Executive Summary

This document tracks the comprehensive deep debt evolution effort for the ToadStool codebase,
focusing on modern idiomatic Rust, capability-based discovery, and eliminating technical debt.

## Completed Work

### 1. Clippy Error Resolution (CRITICAL)

**Status**: 45 `await_holding_lock` errors FIXED

The tests in `crates/core/common/src/infant_discovery/detectors.rs` were holding `std::sync::Mutex`
guards across async `.await` points, which is an anti-pattern that can cause deadlocks.

**Fix Applied**: Restructured tests to use scoped blocks that drop the lock before async operations:

```rust
// Before (BAD - lock held across await)
let _lock = ENV_MUTEX.lock().unwrap();
std::env::set_var("AWS_REGION", "us-east-1");
let result = detector.detect().await.unwrap();  // Lock still held!
std::env::remove_var("AWS_REGION");

// After (GOOD - lock released before await)
let prev = {
    let _lock = ENV_MUTEX.lock().unwrap();
    let prev = std::env::var("AWS_REGION").ok();
    std::env::set_var("AWS_REGION", "us-east-1");
    prev
};  // Lock dropped here

let result = detector.detect().await.unwrap();  // No lock held

{
    let _lock = ENV_MUTEX.lock().unwrap();
    // ... restore prev ...
}
```

### 2. SAFETY Comments Added (CRITICAL)

**Status**: 13 unsafe blocks documented

Added `// SAFETY:` comments to all undocumented unsafe blocks:

- `crates/neuromorphic/akida-driver/src/backends/vfio.rs` - DMA buffer allocation/deallocation
- `showcase/cross-platform/src/cuda_vs_barracuda.rs` - CUDA kernel launches
- `showcase/cross-platform/src/full_stack_benchmark.rs` - CUDA kernel launches
- `showcase/cross-platform/src/parity_benchmark.rs` - CUDA kernel launches

### 3. Capability-Based Discovery (ALREADY IMPLEMENTED)

**Status**: COMPLETE - System already follows Deep Debt principles

The codebase already has comprehensive capability-based discovery:

- `discover_crypto_socket()` - Replaces hardcoded beardog paths
- `discover_storage_socket()` - Replaces hardcoded nestgate paths
- `discover_coordination_socket()` - Replaces hardcoded songbird paths
- `discover_socket_for_capability()` - Generic capability discovery

Old functions are deprecated with migration guides. The primal_sockets.rs module has
clear documentation explaining:
- Fallback constants (transition period only)
- Environment variable overrides
- Capability-based discovery as the preferred method

### 4. Display IPC Semantic Naming (HIGH)

**Status**: FIXED - Method names now follow wateringHole standard

Changed from camelCase to snake_case per semantic naming standard:

| Before | After |
|--------|-------|
| `display.createWindow` | `display.create_window` |
| `display.destroyWindow` | `display.destroy_window` |
| `display.getWindowInfo` | `display.get_window_info` |
| `display.getCapabilities` | `display.get_capabilities` |

Updated in:
- `crates/runtime/display/src/ipc/mod.rs`
- `crates/runtime/display/src/ipc/types.rs`
- `crates/runtime/display/src/ipc/server.rs`
- `crates/runtime/display/src/ipc/client.rs`

### 5. Builder #[must_use] Attributes (MEDIUM)

**Status**: FIXED - GraphNodeBuilder methods annotated

Added `#[must_use]` to all fluent builder methods in `crates/server/src/graph_node.rs`:
- `new()`
- `primal()`
- `cpu()`
- `memory_bytes()`
- `memory_gb()`
- `gpu_memory_bytes()`
- `gpu_memory_gb()`
- `storage_bytes()`
- `storage_gb()`
- `network_mbps()`
- `duration()`
- `duration_secs()`
- `metadata()`

### 6. CI Coverage (ALREADY CONFIGURED)

**Status**: COMPLETE - llvm-cov already in CI workflow

The `.github/workflows/ci.yml` already has:
- cargo-llvm-cov installation
- Workspace coverage run with test filtering
- 90% threshold (currently as warning, can be upgraded to error)
- JSON report generation

## Remaining Work

### High Priority

1. **Fix Remaining 87 Clippy Errors** (id: 11)
   - 13 items missing backticks in documentation
   - 7 manual `RangeInclusive::contains` implementations
   - 5 assertions always true
   - 5 manual `Range::contains` implementations
   - Various other style issues

2. **Evolve Production Stubs** (id: 4)
   - Identify placeholder implementations
   - Complete ML model loaders (BERT, Whisper, YOLO)
   - Implement missing NPU backends

3. **Refactor Large Files** (id: 5)
   - `cg_gpu.rs`: 2,556 lines - Extract solver variants
   - `sparsity.rs`: 1,236 lines - Split by algorithm
   - `env_config.rs`: 1,158 lines - Group by category
   - `pppm_gpu.rs`: 1,158 lines - Extract components
   - Smart refactoring, not just splitting

### Medium Priority

4. **Convert panic!() to Result** (id: 6)
   - 80+ occurrences in library code
   - Focus on user-facing APIs first
   - Keep panics only for invariant violations

5. **Evolve .unwrap()/.expect()** (id: 9)
   - Replace with proper error handling
   - Use `?` operator where possible
   - Add context with `context()` from anyhow

## Architecture Compliance

### UniBin Standard: COMPLIANT
- Single binary per primal
- Subcommand-based modes
- JSON-RPC 2.0 + optional tarpc

### EcoBin Standard: MOSTLY COMPLIANT
- Pure Rust: Yes (except cudarc for CUDA showcase)
- Lint-free: 87 warnings remaining
- Centralized config: Yes
- Zero-copy: 162 files use zero-copy patterns

### Deep Debt Principles: COMPLIANT
- Self-knowledge only: Yes
- Runtime discovery: Yes (via Songbird)
- Services over libraries: Yes
- No hardcoded primal names in production paths: Yes (fallbacks clearly documented)

## Test Coverage

- 3,800+ test attributes found
- llvm-cov configured in CI
- 90% target threshold

## Code Metrics

- 2,263 Rust files
- ~754,267 lines of code
- 555 WGSL shader files
- 9 files exceed 1000 lines (need refactoring)

## Next Steps

1. Fix remaining clippy errors (batch fix for doc backticks)
2. Identify and prioritize stub implementations
3. Create refactoring plan for large files
4. Set up coverage baseline measurement

---

*Generated: February 15, 2026*
*Author: AI Assistant*
