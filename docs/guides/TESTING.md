# Testing Guide

**Last Updated**: Jun 2026 — S310

## Quick Status

| Metric | Status | Details |
|--------|--------|---------|
| **Lib-Only Tests** | **9,145+ (S326)** | Unlimited parallelism; 0 failures |
| **Workspace Tests** | **23,000+ (default features)** | +1,289 behind `legacy-coordination` feature |
| **Ignored Tests** | **~222 `#[ignore]`** | GPU/hardware tests requiring real devices |
| **Line Coverage** | **~85%+ (lib-only)** | ~185K production lines. Gap: hardware-dependent code (V4L2/VFIO/neuromorphic) |
| **Clippy Pedantic** | **0 warnings** | Full workspace (`-D warnings`), in CI |
| **BarraCuda Tests** | Separate primal | Budded to `ecoPrimals/barraCuda/` (S93) |

## Running Tests

```bash
# Full workspace
cargo test --workspace

# Specific crate
cargo test -p toadstool-server

# With coverage
cargo llvm-cov --workspace --ignore-filename-regex "tests/" -- --skip performance_bench --skip slow

# Coverage report
cargo llvm-cov report
```

## Test Architecture

Tests live under `crates/*/tests/` as integration tests, and inline `#[cfg(test)]` modules for unit tests.

### Test Conventions

- `#![allow(clippy::pedantic)]` at the top of dedicated test files
- `temp_env` for all environment variable manipulation (no `std::env::set_var`)
- All tests run concurrently — zero `#[serial]`, zero fixed sleeps in non-chaos tests
- Default timeouts: 5s (unit: 2s, integration: 30s, chaos: 20s)
- `#[tokio::test]` for async tests

### Coverage Breakdown

| Domain | Approximate Coverage |
|--------|---------------------|
| IPC / JSON-RPC | ~95% |
| Core logic / config | ~90% |
| Runtime engines | ~85% |
| Integration / distributed | ~80% |
| Hardware drivers (V4L2, VFIO) | ~25% (requires real hardware) |
| Neuromorphic (Akida) | ~20% (requires AKD1000 NPU) |

### Coverage Gap Analysis

The remaining gap to 90% (~7,400 lines) is concentrated in:
- **V4L2/display** (~3,800 lines): Kernel FFI, mmap, ioctl — requires video capture hardware
- **Neuromorphic/VFIO** (~2,000 lines): PCIe BAR mapping, DMA — requires Akida NPU
- **Test infrastructure** (~1,000 lines): Performance harness code not meant to be self-tested

Software-only modules are at ~89% coverage.

## Contributing Tests

When adding tests:

1. Place integration tests in `crates/<crate>/tests/`
2. Use real implementations, not mocks (mocks only where hardware is unavailable)
3. Test error paths explicitly
4. Use `expect("reason")` instead of `unwrap()` in tests for clear failure messages

```rust
#[tokio::test]
async fn test_feature_workflow() {
    let system = setup().await.expect("setup");
    let result = system.perform_action().await;
    assert!(result.is_ok(), "action should succeed: {result:?}");
}
```

## See Also

- **[NEXT_STEPS.md](../../NEXT_STEPS.md)** — Overall project status and next steps
- **[DOCUMENTATION.md](../../DOCUMENTATION.md)** — Navigation hub and reference index
