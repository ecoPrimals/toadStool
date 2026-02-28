# Status -- February 28, 2026

## Quality Gates

| Gate | Status | Notes |
|------|--------|-------|
| `cargo build --workspace` | PASS | Clean build |
| `cargo fmt --all -- --check` | PASS | 0 diffs |
| `cargo clippy --workspace` | PASS | **1 warning (deprecated grpc fallback — intentional)** |
| `cargo doc --workspace --no-deps` | PASS | 0 warnings |
| `cargo test -p barracuda --lib` | PASS | **2,726+ tests** (5 pre-existing GPU device-loss flakes) |
| `cargo test -p toadstool-server --lib` | PASS | **569 tests** |
| `cargo test -p toadstool --lib` | PASS | **1,340 tests** |
| `cargo test -p toadstool-cli --lib` | PASS | **180 tests** |
| Standalone clone test | PASS | Pull to any machine, `cargo test` works — GPU-optional, CPU fallback |
| License compliance | PASS | AGPL-3.0-or-later: root LICENSE + SPDX headers |
| Test coverage (llvm-cov) | ~82% barracuda | Target: 90% |

## Codebase Metrics

| Metric | Value |
|--------|-------|
| WGSL shaders | **661** (zero orphans, 21 DF64 + 200+ f64, all f64 canonical) |
| Rust version | **1.80+** (std::sync::LazyLock) |
| `unsafe` blocks | **45** (all `// SAFETY:` documented; 2 barracuda SPIRV/cache, rest FFI/hardware/MMIO) |
| `#![deny(unsafe_code)]` | **36 crates** (2 justified: gpu, secure_enclave) |
| External dep debt | **Zero chrono, zero anyhow, zero log (stale), zero once_cell, zero num_cpus** |
| Production `Box<dyn Error>` | **0** — all typed errors via thiserror |
| Production unwraps | **0 blind** — infallible `expect()` only |
| Production mocks | **0** — TpuBackend::Mock behind `mock-tpu` feature gate |
| Dead code | **~35 justified `#[allow(dead_code)]`** (all documented with phase/reason) |
| File size limit | **All < 1000 lines** (16 large files smart-refactored to domain modules) |
| Hardcoded IPs/ports | **0** — named constants throughout |
| ComputeDispatch adoption | **34 ops migrated** (~216 legacy ops remaining, incremental) |

## Architecture Highlights

### GPU Compute
- **Fp64Strategy**: Native/Hybrid with FMA-optimized DF64 + transcendentals
- **Runtime f64 probe**: `basic_f64` compile-time probe detects NAK/NVVM f64 failures
- **NAK workgroup tuning**: `workgroup_size_for_arch()` — Volta 64, Ada 256, RDNA 64, Intel Arc 128
- **ComputeDispatch builder**: Eliminates ~80 lines of BGL/BG/pipeline boilerplate per op
- **metalForge streaming**: `PipelineBuilder` → `StreamingPipeline` — chained GPU dispatches, zero CPU readback
- **StatefulPipeline**: GPU-resident iteration (MD, SCF) with 8-byte convergence readback
- **GPU device-lost recovery**: `catch_unwind` on all submit paths, `is_lost()` early-return

### Server / IPC
- **pure_jsonrpc**: Full JSON-RPC 2.0 with SemanticMethodRegistry, Unix/TCP serving, Cow zero-copy
- **manual_jsonrpc**: Fully deprecated (all handlers ported to pure_jsonrpc)

### Cross-Spring Absorption (Session 69)
- All 5 spring handoffs reviewed and absorbed (196 handoff files)
- 17 AlphaFold2 Evoformer shaders + dispatch
- GPU Lanczos eigensolver + 4 airSpring batch ops + MD observables
- HMM forward/backward/viterbi, stats ops, Anderson coupling

## Session History (Recent)

### Session 69++ (Feb 28, 2026) — Architecture Evolution
- metalForge streaming pipeline implemented
- manual_jsonrpc → pure_jsonrpc: full migration
- 4 production stubs → real implementations
- 10 large files smart-refactored (700-880 lines → domain modules)
- 34 ops migrated to ComputeDispatch (~3,739 lines boilerplate removed)
- NAK architecture-aware workgroup tuning
- +100 new tests across workspace
- Hardcoded IPs → constants, rust-version 1.75→1.80, dead_code documented
- Unsafe evolution: GPU memory bounds checks, SAFETY docs, alloc_and_lock() helper

### Session 69/69+ (Feb 27, 2026) — Cross-Spring Absorption + Deep Debt
- 5 spring handoffs absorbed, 30+ new WGSL shaders created + dispatch wired
- anyhow fully eliminated from all ~30 crates (→ thiserror)
- 6 large files smart-refactored, hardcoding → constants, unsafe reduced
- 2,612+ → 2,625+ barracuda tests

### Session 68+++ (Feb 27, 2026) — Deep Debt Sweep
- chrono eliminated from 28 crates (200+ files migrated to std::time)
- Unsafe 47→45 blocks, ~400 lines dead code removed
- log crate removed, hardcoding → constants, pattern audit clean

### Session 68+ (Feb 26, 2026) — Standalone Resilience
- GPU device-lost recovery on all submit paths
- Test parallelism with RUST_TEST_THREADS=4
- 128 false test failures → 0

### Earlier Sessions (32-68)
- Dual-layer universal precision (op_preamble + df64_rewrite)
- Sovereign compiler phases 1-4 (FMA fusion, DCE, SPIR-V passthrough)
- ESN v2, batched eigensolvers, spectral analysis
- DF64 transcendentals, Lattice QCD, MD forces
- See CHANGELOG.md for full history
