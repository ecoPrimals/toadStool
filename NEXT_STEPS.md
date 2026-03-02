# ToadStool/BarraCuda -- Next Steps

**Updated**: March 2, 2026 -- Session 86
**Status**: Production-grade | AGPL-3 compliant | 0 clippy warnings | Standalone-resilient | Zero chrono | Zero anyhow | Zero pollster | Zero serde_yaml | Zero libc (akida-driver) | Zero production stubs | 45 justified unsafe | 844 WGSL shaders (37 DF64, 15 folding) | 2,866 barracuda tests | 5,500+ workspace lib tests (8,300+ total) | Rust 1.80+ | 35+ god files refactored | Capability-based discovery | NVK GPU resilience | barracuda::nautilus (22 tests) | 44 JSON-RPC methods | BatchedEncoder + fused_mlp | Batch Nelder-Mead GPU
**Latest**: S84–S86 — 144 ComputeDispatch ops (+33 across 3 sessions). Hydrology god-file refactored. Production stubs evolved (experimental.rs → real probes, wgpu_backend.rs → device limits). Full ops audit: ~139 legacy ops remaining.

---

## Active Work

### P0: ComputeDispatch Migration (Incremental)

144 of ~280+ ops migrated to the fluent `ComputeDispatch` builder. Each migration replaces
~80 lines of manual BGL/BG/pipeline boilerplate with ~5 lines. ~139 ops remaining (full
audit S86 revealed ops in subdirectories: bio/, md/, lattice/, complex/, linalg/).

Recent migrations:
- S84 (+9): matmul_tiled, gemm_f64, giou_loss, focal_loss, tversky_loss, huber_loss, hinge_loss, contrastive_loss, chamfer_distance
- S85 (+12): cosine_similarity, covariance, cross_product, psnr, ssim, diag, global_avgpool, box_iou, focal_loss_alpha, rotary_embedding, alibi, flatten
- S86 (+12): determinant, mse_loss, dice, quantize, dequantize, bce_loss, permute, movedim, logsumexp, index_add, tensor_split, concat
- See EVOLUTION_TRACKER.md for full migration history

### P1: DF64 Default Path (Architecture)

Make `df64_rewrite` the default precision strategy, not a fallback. Currently DF64 activates
only when `Fp64Strategy::Hybrid` is selected. For consumer GPUs (1:64 FP64:FP32),
DF64 should be the primary path with native f64 reserved for reductions/convergence.

### P1: DF64 Transcendental Coverage

Extend `df64_transcendentals.wgsl`:
- [x] `asin_df64`, `acos_df64`, `atan_df64`, `atan2_df64` (S71)
- [x] `sinh_df64`, `cosh_df64` (S71)
- [x] `gamma_df64` (Lanczos g=7, reflection formula), `erf_df64` (A&S 7.1.26) (S71++)

### P2: Architecture-Specific Polynomial Selection (Q3 2026)

Different evaluation strategies per silicon family:
- SM70 (Volta): 8-cycle ILP fill — longer Horner chains
- SM80+ (Ampere/Ada): 4-cycle ILP — Estrin evaluation may beat Horner
- RDNA2/3 (AMD): VALU utilization patterns differ from NVIDIA

### P2: NpuDispatch Trait

Generic NPU interface — airSpring/wetSpring/groundSpring converge on a single
dispatch trait for neuromorphic hardware (Akida, Coral, future NPUs).

### Sovereign Phase 4+ — naga-IR Optimizer Evolution

Phase 4 core is DONE (FMA fusion, DCE, SPIR-V passthrough). Remaining iterations:
- [ ] Register pressure estimation (live-range counting on naga expression arena)
- [ ] Loop software pipelining at naga IR level
- [ ] Architecture-specific peephole optimization per `GpuArch`
- [ ] naga → NAK IR direct bridge (research)

---

## Infrastructure Checklist

- [x] **Rust dispatch wiring** -- 13 S69 shaders + AlphaFold2 + Lanczos + airSpring + MD observables
- [x] **metalForge streaming** -- Stage/Pipeline/Topology builder (staging/pipeline.rs)
- [x] **NAK workgroup tuning** -- `workgroup_size_for_arch()` with 6 tests
- [x] **`anyhow` → `thiserror`** -- fully eliminated from all ~30 workspace crates
- [x] **`manual_jsonrpc` → `pure_jsonrpc`** -- full migration, unibin uses pure_jsonrpc
- [x] **GPU Lanczos kernel** -- `lanczos_iteration_f64.wgsl` + `lanczos_eigensolver()` dispatch
- [x] **rust-version** -- bumped 1.75 → 1.80 (LazyLock stable)
- [x] **Production stubs** -- 15+ stubs evolved to real implementations or proper errors
- [x] **Dead code documented** -- all `#[allow(dead_code)]` annotated with justification
- [x] **Unidirectional streaming** -- ring_buffer + unidirectional + stateful + pipeline
- [x] **MD observables** -- stress_virial_f64, vacf_batch_f64 created + dispatch wired
- [x] **AlphaFold2 advanced (17)** -- all created + dispatch wired
- [x] **airSpring batch ops** -- hargreaves_et0, dual_kc, van_genuchten, batched_crop_pipeline
- [x] **Test concurrency** -- all tests concurrent, zero serial, zero fixed sleeps in non-chaos
- [x] **Environment safety** -- all `std::env::set_var` migrated to `temp_env`
- [x] **All doctests passing** -- common, core, display, testing
- [x] **Error code correctness** -- `WORKLOAD_NOT_FOUND` for job queue, `EXECUTION_NOT_FOUND` for API
- [x] **Chaos metrics sync** -- ChaosEngine recovery_count propagated to both SystemState and ChaosMetrics
- [x] **Edge platform evolution** -- ESP32, Raspberry Pi, industrial, microcontroller return proper errors
- [x] **Real mDNS parser** -- replaces placeholder `Ok(None)` in zero_config service discovery
- [x] **pollster eliminated** -- removed from barracuda, toadstool, universal (→ tokio_block_on)
- [x] **serde_yaml → serde_yaml_ng** -- across workspace
- [x] **async-trait → AFIT** -- 5 crates migrated (performance, analytics, wasm, gpu, security/sandbox)
- [x] **Capability-based naming** -- CLI/JSON-RPC/error messages use capability language, type aliases added
- [x] **GPU test resilience** -- NVK catch_unwind wrappers on 11+29+homomorphic test files
- [x] **Wildcard re-exports narrowed** -- 13 crates (toadstool, distributed, server, gpu, universal, orchestration, sandbox, wasm, edge discovery/toolchain/comms/deployment)
- [x] **9 god files refactored (S74+S75)** -- primal_integration, capability_provider, primals/lib, opencl_impl, env_overrides, os_layer/compat, workload, unified, precision/mod
- [ ] **ComputeDispatch migration** -- 144/280+ ops migrated; ~139 remaining (incremental)
- [ ] **DF64 default path** -- df64_rewrite as default, not fallback (groundSpring V35)
- [ ] **NpuDispatch trait** -- generic NPU interface
- [ ] **Test coverage target 90%** -- significant gains across CLI, server, API, monitoring, distributed

### Cross-Repo Debt

- [ ] **D-S20-003**: neuralSpring `evolved/` migration (~2075 lines) — awaiting neuralSpring team
- [ ] **D-S18-002**: cubecl transitive `dirs-sys` — needs upstream PR

---

## Completed This Session (S84–S86)

### Session 86: ComputeDispatch Batch 7 + Production Stub Evolution
- 12 ops → ComputeDispatch (determinant, mse_loss, dice, quantize, dequantize, bce_loss, permute, movedim, logsumexp, index_add, tensor_split, concat)
- wgpu_backend.rs: magic numbers → real `device.limits()` queries
- deployment.rs: 10 placeholder stubs → capability-discovery documentation
- Full ops audit: corrected remaining count from ~57 to ~139

### Sessions 84–85: ComputeDispatch Batches 5–6 + Hydrology + Probes
- 21 ops → ComputeDispatch across two sessions
- hydrology.rs god file → hydrology/ directory module
- experimental.rs stub → real FPGA/neuromorphic/quantum probes
- mDNS constants extracted; frameworks.rs echo → proper error

### Session 80: Nautilus Absorption + BatchedEncoder + Nelder-Mead GPU
- `barracuda::nautilus` module (7 files, 22 tests) — standalone bingoCube evolutionary reservoir computing
- `ai.nautilus.*` 8 JSON-RPC methods wired into daemon (feature-gated `nautilus`)
- `BatchedEncoder` — single `CommandEncoder` for multi-op GPU pipelines (2 tests)
- `fused_mlp` — MLP forward pass via BatchedEncoder (single submit across layers)
- Batch Nelder-Mead GPU — N parallel optimizations, batched simplex shader ops
- `StatefulPipeline<S>` + `WaterBalanceState` — day-over-day state tracking
- `GpuDriverProfile` sin/cos F64 workarounds (Taylor preamble for NVK, 4 tests)
- `NeighborMode::PrecomputedBuffer` — 2D/3D/4D periodic lattice precomputation (6 tests)
- `BatchedMultinomialGpu` alignment — `cumulative_probs` + `seed` (groundSpring V37)
- ComputeDispatch: 76→95 ops (4 migration batches, 19 ops)
- Socket resolution consolidated: 4 call sites → `toadstool_common::primal_sockets` API
- Confirmed existing: `SparseGemmF64`, IPC multi-transport

### Session 79: ESN MultiHeadEsn + ExportedWeights + SpectralAnalysis
- 36-head `MultiHeadEsn` with 6 `HeadGroup` variants, `head_disagreement()` uncertainty
- `ExportedWeights` aligned with hotSpring: input_size, reservoir_size, output_size, leak_rate, head_labels
- `SpectralAnalysis` extensions: spectral_bandwidth, spectral_condition_number, classify_spectral_phase
- ComputeDispatch: 5 more ops → 76 total
- bitcast<f64> fixes in 2 WGSL shaders → storage buffer approach

## Completed (S78)

### Session 78: Deep Debt + Dependency Evolution
- Wildcard re-exports narrowed in 7 more crates (sandbox, wasm, edge discovery/toolchain/comms/deployment). Total: 13.
- `legacy_primal_to_capabilities` / `legacy_primal_primary_capability` removed from primal_capabilities.rs (no callers).
- `libc` fully removed from akida-driver — rustix for VFIO ioctls. Custom VfioIoctlReturn/VfioIoctlPtr wrappers.
- async-trait → native AFIT in security/sandbox (SandboxManager). Total: 5 crates.
- ComputeDispatch: 5 more ops (eq, map, dotproduct, dropout, split). Total: 71.
- ~40 new tests (api ~20, auto-config ~9, server ~11).
- 5 ToadStoolError doc links fixed.
- Compile bottleneck analysis done.

## Completed (S74 through S75)

### Session 75: Module Architecture + Build Streamlining
- 6 god files smart-refactored: primal_integration.rs (1,163L→5 modules), capability_provider.rs (746L→5 modules), primals/lib.rs (580L→7 modules), opencl_impl.rs (831L→6 modules), env_overrides.rs (726L→9 modules), os_layer/compat.rs (766L→7 modules)
- Wildcard `pub use *` narrowed in 6 crates: toadstool, distributed, server, gpu, universal, orchestration
- pollster removed from toadstool + universal
- 3 evolved backends gated behind `#[cfg(test)]`
- TYPES_REFERENCE.md updated with Module Structure Reference

### Session 74: Deep Debt Evolution — Dependencies + Capabilities + Resilience
- serde_yaml → serde_yaml_ng across workspace
- async-trait → native AFIT in 4 crates
- pollster → tokio_block_on in barracuda (dependency removed)
- Hardcoded primal names → capability-based language + type aliases
- Edge platform stubs → genuine hardware probing
- Discovery stubs → real mDNS/k8s/docker/registry probing
- 3 god files refactored: workload.rs, unified.rs, precision/mod.rs
- GPU test resilience: catch_unwind wrappers for NVK driver panics
- WgpuDevice::poll_safe() for device-lost recovery
- Net -3,828 lines across 182 files

### Previously Completed (S68–S71)
- **S71**: 6 GPU dispatch structs, DF64 transcendental suite (15 functions), 32 ComputeDispatch migrations, 6 god files refactored, net -9,192 lines
- **S70+++**: builder.rs refactored, EcosystemCaller deleted, monitoring evolved to real sysinfo
- **S70+/++**: 7 WGSL shaders, sovereignty evolution, Fp64Strategy::Concurrent, +37 tests
- **S70**: 15 stubs → real implementations, all env tests → temp_env, +150 tests
- **S69++**: metalForge streaming, manual_jsonrpc → pure_jsonrpc, 34 ComputeDispatch ops
- **S69/69+**: 5 spring handoffs absorbed, 30+ WGSL shaders, anyhow eliminated
- **S68+++**: chrono eliminated (28 crates), unsafe 47→45, ~400 lines dead code

---

See [CHANGELOG.md](CHANGELOG.md) for full completed session history.
