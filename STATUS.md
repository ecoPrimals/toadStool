# Status -- March 2, 2026 (Session 82)

## Quality Gates

| Gate | Status | Notes |
|------|--------|-------|
| `cargo build --workspace` | PASS | Clean build |
| `cargo fmt --all -- --check` | PASS | 0 diffs |
| `cargo clippy --all-targets -- -D warnings` | PASS | **0 warnings** |
| `cargo doc --workspace --no-deps` | PASS | 0 warnings |
| `cargo test -p barracuda --lib` | PASS | **2,858+ tests** (GPU device-loss resilient via `catch_unwind`) |
| `cargo test -p toadstool-server --lib` | PASS | **576 tests** |
| `cargo test -p toadstool --lib` | PASS | **1,340 tests** |
| `cargo test -p toadstool-cli --lib` | PASS | **209 tests** |
| `cargo test -p toadstool-common --lib` | PASS | **923 tests** |
| `cargo test -p toadstool-distributed --lib` | PASS | **1,057 tests** |
| `cargo test -p toadstool-config --lib` | PASS | **368 tests** |
| `cargo test -p toadstool-api --lib` | PASS | **58 tests** |
| `cargo test --workspace` (excl barracuda) | PASS | **6m30s wall time, 8 threads, NVK GPU resilience wrappers** |
| All doctests | PASS | common, core, server, cli, testing, display |
| Standalone clone test | PASS | Pull to any machine, `cargo test` works — GPU-optional, CPU fallback |
| License compliance | PASS | AGPL-3.0-or-later: root LICENSE + SPDX headers |

## Codebase Metrics

| Metric | Value |
|--------|-------|
| WGSL shaders | **844** (zero orphans, 37 DF64 + 15 folding + 200+ f64, all f64 canonical) |
| Rust version | **1.80+** (std::sync::LazyLock) |
| `unsafe` blocks | **45** (all `// SAFETY:` documented; 2 barracuda SPIRV/cache, rest FFI/hardware/MMIO) |
| `#![deny(unsafe_code)]` | **36 crates** (2 justified: gpu, secure_enclave) |
| External dep debt | **Zero chrono, zero anyhow, zero log (stale), zero once_cell, zero num_cpus, zero pollster, zero serde_yaml** |
| Production `Box<dyn Error>` | **0** — all typed errors via thiserror |
| Production unwraps | **0 blind** — infallible `expect()` only |
| Production mocks/stubs | **0** — all evolved to real implementations or proper errors (S82 memory detection evolution) |
| Dead code | **~35 justified `#[allow(dead_code)]`** (all documented with phase/reason) |
| File size limit | **All < 1000 lines** (32+ large files smart-refactored to domain modules) |
| Wildcard re-exports narrowed | 13 crates (sandbox, wasm, edge discovery/toolchain/comms/deployment + 6 prior) |
| External deps removed (S74-S78) | pollster, serde_yaml, async-trait (5 crates), libc (akida-driver) |
| Hardcoded IPs/ports | **0** — named constants throughout |
| ComputeDispatch adoption | **111 ops migrated** (~139 legacy ops remaining, incremental) |
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

### Session 80 (Mar 2, 2026) — Nautilus Absorption + BatchedEncoder + Nelder-Mead GPU
- `barracuda::nautilus` module (7 files, 22 tests): standalone bingoCube evolutionary reservoir computing — boards, evolution, population, readout, shell, brain
- `ai.nautilus.*` JSON-RPC namespace: 8 methods wired into daemon (`status`, `observe`, `train`, `predict`, `screen`, `edges`, `shell.export`, `shell.import`). Feature-gated `nautilus` in CLI
- `BatchedEncoder`: single `CommandEncoder` for multi-op GPU pipelines (46-78× speedup potential). `BatchedPassBuilder` API.
- `fused_mlp`: MLP forward pass via BatchedEncoder (single submit across layers, ReLU activation)
- Batch Nelder-Mead GPU: N independent optimizations in parallel via batched simplex shader ops
- `StatefulPipeline<S>`: generic pipeline for day-over-day state tracking + `WaterBalanceState` example
- `GpuDriverProfile` sin/cos F64 workarounds: Taylor-series preamble for NVK, `asin`/`acos` protected
- `NeighborMode::PrecomputedBuffer`: 2D/3D/4D periodic lattice precomputation (6 tests)
- `BatchedMultinomialGpu` alignment: `cumulative_probs` + `seed` config (groundSpring V37)
- ComputeDispatch: 76→95 ops migrated (4 batches: elastic_transform, gillespie, tree_inference, mixup, random_affine, random_perspective, lennard_jones_f64, cumsum_f64, label_smoothing, slice_assign, random_crop, lp_pool2d, unfold, global_maxpool, adaptive_avgpool2d, adaptive_maxpool2d, reduce, scan, embedding_wgsl)
- Socket resolution consolidated: 4 call sites → `toadstool_common::primal_sockets` API
- Confirmed existing: `SparseGemmF64` (CSR×dense SpMM), IPC multi-transport (Unix/Abstract/TCP)
- All quality gates green: clippy 0, fmt 0, doc 0

### Session 79 (Mar 2, 2026) — ESN MultiHeadEsn + ExportedWeights + SpectralAnalysis
- 36-head `MultiHeadEsn` with 6 `HeadGroup` variants (Anderson, Qcd, Potts, Steering, Brain, Meta)
- `head_disagreement()` uncertainty metric, configurable per-head readout via `HeadConfig`
- `ExportedWeights` aligned with hotSpring: `input_size`, `reservoir_size`, `output_size`, `leak_rate`, `head_labels`
- `SpectralAnalysis` extensions: `spectral_bandwidth`, `spectral_condition_number`, `classify_spectral_phase` (Bulk/EdgeOfChaos/Chaotic)
- ComputeDispatch: 5 more ops (boltzmann_sampling, batched_multinomial, diversity_fusion, batched_elementwise_f64, earth_mover_distance) → 76 total
- bitcast<f64> fixes in 2 WGSL shaders (jackknife_mean_f64, boltzmann_sampling_f64) → storage buffer approach

### Session 78 (Mar 2, 2026) — Deep Debt + Dependency Evolution
- Wildcard re-exports narrowed in 7 more crates (sandbox, wasm, edge discovery/toolchain/comms/deployment). Total: 13 crates.
- `legacy_primal_to_capabilities()` and `legacy_primal_primary_capability()` removed from primal_capabilities.rs (no callers). Module now clean capability-to-primal mapping.
- `libc` fully removed from akida-driver — migrated to `rustix` for all VFIO ioctls (vfio.rs, mmio.rs). Custom `VfioIoctlReturn`/`VfioIoctlPtr` safe wrappers. 6 clippy `ref_as_ptr`/`borrow_as_ptr` fixes.
- `async-trait` migration: 1 more crate (security/sandbox — `SandboxManager` trait). Total: 5 crates migrated to native AFIT.
- ComputeDispatch: 5 more ops migrated (boltzmann_sampling, batched_multinomial, diversity_fusion, batched_elementwise_f64, earth_mover_distance). Total: 76 ops, ~174 remaining.
- ~40 new tests: toadstool-api (~20), toadstool-auto-config (~9), toadstool-server (~11).
- 5 broken `ToadStoolError` doc links fixed (universal_adapter/mod.rs, discovery_integration.rs).
- Compile bottleneck analysis: tfhe+tfhe-fft = 30.6% CPU (showcase); wgpu 22/23 duplication wastes ~90s.
- Quality gates: build, clippy (0 warnings), fmt (0 diffs), doc (toadstool-common) all PASS.

### Session 76 (Mar 1, 2026) — Spring Absorption Execution + Folding Shaders + New GPU Ops
- `EVOLUTION_TRACKER.md` created — root-level single source of truth for evolution status
- `barracuda::nn` complete: LstmReservoir + EsnClassifier (12 nn tests pass)
- 15 sovereign folding DF64 shaders: geometry (4), energy (4), refinement (4), prediction (3) + `FoldingOp` enum
- 4 new GPU ops: `FusedChiSquaredGpu`, `FusedKlDivergenceGpu`, `RawrWeightedMeanGpu`, `BoltzmannSamplingGpu`
- airSpring ops 9-13: VG θ(h), VG K(h), Thornthwaite ET₀, GDD, Pedotransfer polynomial
- 4 god files refactored: wgpu_device/mod.rs (→compilation.rs), driver_profile (→directory), probe (→directory), jsonrpc (→directory)
- Dependency analysis: async-trait 50+ uses all appropriate; libc FFI-only
- Hardcoding audit: 2 production fixes (industrial/raspberry_pi DEFAULT_HOSTNAME)
- Metrics: 844 shaders (+98), 37 DF64 (+12), 2,781 barracuda tests (+20), 32+ god files refactored (+4)

### Session 75 (Feb 28, 2026) — Module Architecture + Build Streamlining
- 6 god files smart-refactored: primal_integration.rs (1,163L→5 modules), capability_provider.rs (746L→5 modules), integration/primals/lib.rs (580L→7 modules), opencl_impl.rs (831L→6 modules), env_overrides.rs (726L→9 modules), os_layer/compat.rs (766L→7 modules)
- Wildcard `pub use *` narrowed to explicit re-exports in 6 crates: toadstool, distributed, server, gpu, universal, orchestration
- `pollster` removed from toadstool + universal Cargo.toml
- 3 evolved backends gated behind `#[cfg(test)]` (biomeos_integration)
- TYPES_REFERENCE.md updated with Section 7: Module Structure Reference
- All quality gates green: build, fmt, clippy (0 warnings), doc

### Session 74 (Feb 28, 2026) — Deep Debt Evolution: Dependencies + Capabilities + Resilience
- `serde_yaml` → `serde_yaml_ng` across workspace
- `async-trait` → native AFIT in 4 crates (performance, analytics, wasm, gpu)
- `pollster` → `tokio_block_on` in barracuda; dependency removed
- Hardcoded primal names → capability-based language in CLI templates, JSON-RPC, error messages
- `AuthResponse::standalone()` + `is_standalone()` formalized
- Type aliases: OrchestrationConfigurator, OrchestrationNetworkConfig, PkiSecurityConfig
- Edge platform stubs → genuine hardware probing (Raspberry Pi, industrial, microcontroller)
- Discovery stubs → real mDNS/k8s/docker/registry probing
- God files: workload.rs (829L→2 modules), unified.rs (613L→3 modules), precision/mod.rs (816L→3 modules)
- GPU test resilience: 11 barracuda + 29 ml-inference + homomorphic tests wrapped with catch_unwind
- WgpuDevice::poll_safe() for device-lost recovery
- Doctest fixes across barracuda and showcase crates
- Net -3,828 lines across 182 files

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
