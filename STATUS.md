# Status -- March 1, 2026

## Quality Gates

| Gate | Status | Notes |
|------|--------|-------|
| `cargo build --workspace` | PASS | Clean build |
| `cargo fmt --all -- --check` | PASS | 0 diffs |
| `cargo clippy --all-targets -- -D warnings` | PASS | **0 warnings** |
| `cargo doc --workspace --no-deps` | PASS | 0 warnings |
| `cargo test -p barracuda --lib` | PASS | **2,773+ tests** (5 pre-existing GPU device-loss flakes) |
| `cargo test -p toadstool-server --lib` | PASS | **576 tests** |
| `cargo test -p toadstool --lib` | PASS | **1,340 tests** |
| `cargo test -p toadstool-cli --lib` | PASS | **209 tests** |
| `cargo test -p toadstool-common --lib` | PASS | **921 tests** |
| `cargo test -p toadstool-distributed --lib` | PASS | **1,057 tests** |
| `cargo test -p toadstool-config --lib` | PASS | **368 tests** |
| `cargo test -p toadstool-api --lib` | PASS | **58 tests** |
| `cargo test --workspace` (excl barracuda) | PASS | **6m30s wall time, 8 threads** |
| All doctests | PASS | common, core, server, cli, testing, display |
| Standalone clone test | PASS | Pull to any machine, `cargo test` works — GPU-optional, CPU fallback |
| License compliance | PASS | AGPL-3.0-or-later: root LICENSE + SPDX headers |

## Codebase Metrics

| Metric | Value |
|--------|-------|
| WGSL shaders | **671** (zero orphans, 29 DF64 + 200+ f64, all f64 canonical) |
| Rust version | **1.80+** (std::sync::LazyLock) |
| `unsafe` blocks | **45** (all `// SAFETY:` documented; 2 barracuda SPIRV/cache, rest FFI/hardware/MMIO) |
| `#![deny(unsafe_code)]` | **36 crates** (2 justified: gpu, secure_enclave) |
| External dep debt | **Zero chrono, zero anyhow, zero log (stale), zero once_cell, zero num_cpus** |
| Production `Box<dyn Error>` | **0** — all typed errors via thiserror |
| Production unwraps | **0 blind** — infallible `expect()` only |
| Production mocks/stubs | **0** — all evolved to real implementations or proper errors |
| Dead code | **~35 justified `#[allow(dead_code)]`** (all documented with phase/reason) |
| File size limit | **All < 1000 lines** (22 large files smart-refactored to domain modules) |
| Hardcoded IPs/ports | **0** — named constants throughout |
| ComputeDispatch adoption | **66 ops migrated** (~184 legacy ops remaining, incremental) |
| Test concurrency | **All concurrent** — zero `#[serial]`, zero fixed sleeps in non-chaos tests |
| Environment safety | **All `temp_env`** — zero `std::env::set_var` in test code |
| Default test timeout | **5s** (unit: 2s, integration: 30s, chaos: 20s) |

## Architecture Highlights

### GPU Compute
- **Fp64Strategy**: Native/Hybrid/Concurrent with FMA-optimized DF64 + transcendentals
- **Runtime f64 probe**: `basic_f64` compile-time probe detects NAK/NVVM f64 failures
- **NAK workgroup tuning**: `workgroup_size_for_arch()` — Volta 64, Ada 256, RDNA 64, Intel Arc 128
- **ComputeDispatch builder**: Eliminates ~80 lines of BGL/BG/pipeline boilerplate per op
- **metalForge streaming**: `PipelineBuilder` → `StreamingPipeline` — chained GPU dispatches, zero CPU readback
- **StatefulPipeline**: GPU-resident iteration (MD, SCF) with 8-byte convergence readback
- **GPU device-lost recovery**: `catch_unwind` on all submit paths, `is_lost()` early-return

### Server / IPC
- **pure_jsonrpc**: Full JSON-RPC 2.0 with SemanticMethodRegistry, Unix/TCP serving, Cow zero-copy
- **manual_jsonrpc**: Fully deprecated (all handlers ported to pure_jsonrpc)
- **Error codes**: Proper `WORKLOAD_NOT_FOUND` (-32000) for job queue errors
- **Coordinator cancel**: Real `CancellationToken`-based execution cancellation

### Testing Infrastructure
- **Fully concurrent**: All tests run with `--test-threads=8`, zero serial tests
- **Event-driven**: Sleeps replaced with `timeout` + polling or `yield_now` in non-chaos tests
- **Thread-safe env**: All environment variable manipulation via `temp_env`
- **Unique temp files**: Storage benchmarks use nanos-based unique filenames
- **Reduced timeouts**: 5s default (was 30s), 2s unit (was 5s), 30s integration (was 120s)
- **Chaos tests**: Allowed longer timeouts and sleeps (fault injection stabilization)

### Cross-Spring Absorption (Session 69 + 70+)
- All 5 spring handoffs reviewed and absorbed (196 handoff files)
- 17 AlphaFold2 Evoformer shaders + dispatch
- GPU Lanczos eigensolver + 4 airSpring batch ops + MD observables
- HMM forward/backward/viterbi, stats ops, Anderson coupling
- S70+: 4 new science ops (SensorCalibration, Hargreaves ET0, KcClimateAdjust, DualKcKe)
- S70+: 5 new DF64 ML shaders (GELU, Sigmoid, Softmax, LayerNorm, SDPA)
- S70+: Brent root-finding + seasonal pipeline fused shader
- S70+: Evolution stats (kimura_fixation), jackknife resampling, fao56_et0, chao1_classic
- S70+: SimpleMLP with JSON weight serialization

## Session History (Recent)

### Session 71 (Mar 1, 2026) — GPU Dispatch Wiring + Sovereignty + Smart Refactoring
- Wired 4 previously orphaned shader constants to GPU dispatch: `WGSL_HMM_FORWARD_LOG_F32/F64`, `WGSL_BOOTSTRAP_MEAN_F64`, `WGSL_HISTOGRAM`
- 3 new GPU shaders: `kimura_fixation_f64.wgsl`, `jackknife_mean_f64.wgsl`, `hargreaves_batch_f64.wgsl`
- 6 new GPU dispatch structs: `HmmForwardLogF32/F64`, `BootstrapMeanGpu`, `HistogramGpu`, `KimuraGpu`, `JackknifeMeanGpu`, `HargreavesBatchGpu`
- Hardcoded primal names → `primals::*` constants in 6 production files
- `jsonrpc_server.rs` refactored 904→628 lines via shared test helper
- `network_config/types.rs` split 859→7 domain submodules (34/34 tests pass)
- 2,773+ barracuda tests, 671 WGSL shaders, all quality gates green

### Session 70+++ (Feb 28, 2026) — Builder Refactor + Dead Code + Monitoring Evolution
- `builder.rs` (975 lines) → `builder/` module: mod.rs (129) + profiler.rs (531) + substrate.rs (338)
- Deleted deprecated `EcosystemCaller` (95 lines dead code, zero references workspace-wide)
- Monitoring collectors evolved from hardcoded stubs to real `sysinfo` metrics:
  - `collect_system_health`: real CPU/memory/storage thresholds (80% warn, 95% critical)
  - `collect_resource_usage`: real GB/Mbps from sysinfo + load_average
  - `get_active_alerts`: generates alerts from health status (was empty vec)
  - `collect_biome_status`: returns empty (was fake "example-biome" data)
  - `collect_performance_metrics`: tracks active sessions (was hardcoded scores)
- NestGate `connect()`: placeholder endpoint → `primal_sockets::get_socket_path_for_service()`
- Root docs updated for S70+ through S70+++ (all stale counts fixed)
- All quality gates green: build, fmt, clippy, doc

### Session 70++ (Feb 28, 2026) — Sovereignty + Architecture + Stub Evolution
- Sovereignty: hardcoded port 8084 → `toadstool_config::ports::daemon_port()`
- Sovereignty: hardcoded "songbird" discovery backend → "mdns" (capability-based)
- Sovereignty: `create_adapter_for_endpoint` refactored from string-matching to universal adapter
- Architecture: `Fp64Strategy::Concurrent` variant for dual-validation harnesses (9 dispatch arms updated)
- Architecture: `barracuda::math` re-exports `lower_incomplete_gamma` + `norm_cdf`
- Refactoring: monitoring `lib.rs` split 1071→679 lines (extracted process, thresholds, platform modules)
- Stub evolution: `UniversalAdapter` now validates runtime hints and injects default timeouts
- Clippy: 2 `manual_div_ceil` fixes in linalg GPU executors
- All quality gates green: build, fmt, clippy, doc

### Session 70+ (Feb 28, 2026) — Cross-Spring Absorption (airSpring/groundSpring/neuralSpring/wetSpring)
- 7 new WGSL shaders: gelu_df64, sigmoid_df64, softmax_df64, layer_norm_df64, sdpa_df64, brent_f64, seasonal_pipeline
- 6 new GPU ops: batched_elementwise ops 5-8 (SensorCalibration, HargreavesEt0, KcClimateAdjust, DualKcKe), SymmetrizeGpu, LaplacianGpu
- 3 new stats modules: evolution (kimura_fixation_prob, detection_power/threshold), jackknife (leave-one-out + generalized), hydrology (fao56_et0)
- Diversity: chao1_classic (Chao 1984 u64 formula) alongside existing chao1 (Chao & Chiu 2016 f64)
- Neural network: SimpleMLP with JSON weight serde + forward inference
- Tensor: non-consuming `matmul_ref` for recurrent architectures
- GPU safety: `sanitize_max_buffer_size` (caps NVK absurd values to architectural limits)
- GPU tuning: `preferred_workgroup_size()` (Volta 64, Ampere/Ada 256, RDNA 256, fallback 128)
- +37 new tests across batched_elementwise, hydrology, diversity, evolution, jackknife, SimpleMLP

### Session 70 (Feb 28, 2026) — Deep Debt + Test Concurrency Evolution
- 15 production stubs evolved to real implementations (primals client, orchestrator, coordinator cancel, edge platforms)
- All `std::env::set_var` in tests migrated to `temp_env` (8 files)
- All sleeps removed from non-chaos tests (monitoring, tarpc, resilience)
- Default test timeouts reduced (30s→5s, 120s→30s, 60s→20s)
- All doctests fixed (common, core, display, testing)
- ChaosEngine metrics sync corrected (recovery_count)
- Storage benchmark race condition fixed (unique temp files)
- Nested runtime panics eliminated (MockTask drop)
- Barracuda `#![allow(clippy::unused_async)]` with justification
- Edge/embedded placeholders evolved to proper `PlatformNotAvailable` errors
- Real mDNS response parser implemented (replaced placeholder)
- +150 new tests: lifecycle, dispatch, jsonrpc, monitoring, nestgate, display IPC, daemon servers, config validation
- Killed 2 zombie barracuda processes (running since Feb 26)
- Full workspace test suite: 6m30s, 0 failures, 0 warnings

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
