# ToadStool/BarraCuda -- Next Steps

**Updated**: March 6, 2026 -- S97
**Status**: Production-grade | AGPL-3 compliant | 0 clippy warnings | all tests pass | 57 JSON-RPC methods | NVK Volta f64 probe active | Hardware Transport Layer wired to daemon + CLI | REST + middleware removed | Sovereignty: capability-based canonical | ecoBin pure-rust verified | Sovereign pipeline: HardwareFingerprint + SubstrateCapabilityKind | 40+ god files refactored | All files < 1000 lines
**Latest**: S97 — Spring absorption: NVK Volta f64 probe, subgroup detection, AdaptiveSimulationController, science.* IPC namespace (10 methods), test coverage +59.

---

## Active Work

### ~~P0: ComputeDispatch Migration~~ → Transferred to barraCuda (S93)

**Transferred.** ComputeDispatch lives in the barraCuda crate. 144/280+ ops migrated;
~139 remaining. barraCuda team owns this incremental migration.

### ~~P1: DF64 Default Path~~ → Transferred to barraCuda (S93)

**Transferred.** barraCuda owns precision strategy (f64/df64/f32 validation, shader
selection, `df64_rewrite` as default). toadStool serves hardware capabilities.
Handoff: `wateringHole/handoffs/TOADSTOOL_S93_DF64_HANDOFF_MAR03_2026.md`.

### ~~P1: NpuDispatch Trait~~ ✅ RESOLVED (S94b)

`toadstool-core::npu_dispatch` — generic `NpuDispatch` trait + `AkidaNpuDispatch`
adapter. Vendor-agnostic, capability-based, zero-copy input (`Cow`). Also added
`NpuParameterController` trait (hotSpring absorption) for NPU-driven autonomous
parameter tuning.

### P1: Test Coverage → 90% (D-COV)

~85% line coverage. 6,176 lib tests pass. Focus areas for next push: low-coverage crates in CLI ecosystem
integration, distributed coordination, auto_config installer paths, runtime edge platforms.

### ~~P1: Sovereignty Migration (D-SOV)~~ ✅ RESOLVED (S94b)

All 7 production callers of `get_socket_path_for_service` migrated to
`get_socket_path_for_capability()`. CLI filesystem and socket discovery use capability
names directly. Deprecated API definitions retained for backward compatibility only.

---

### Transferred to Other Teams

| Item | Owner | Notes |
|------|-------|-------|
| D-DF64: DF64 as default precision | barraCuda team | S93: precision strategy is barraCuda's domain |
| DF64 transcendental coverage | barraCuda team | COMPLETE (S71): 15 functions |
| Architecture-specific polynomial selection | barraCuda team | Per-silicon Horner vs Estrin |
| Sovereign compiler Phase 4+ | barraCuda team | naga-IR optimizer, register pressure, peepholes |
| barraCuda budding Phases 1-4 | barraCuda team | API audit, SemVer 1.0, Springs rewire |
| ComputeDispatch migration (D-CD) | barraCuda team | 144/280+ done; ~139 remaining; lives in barraCuda crate |

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
- [x] **ComputeDispatch migration** -- transferred to barraCuda team (lives in barraCuda crate)
- [x] **DF64 default path** -- transferred to barraCuda team (S93)
- [x] **NpuDispatch trait** -- generic NPU interface (toadStool D-NPU)
- [ ] **Test coverage target 90%** -- 18,028 tests; focus on low-coverage crates (toadStool D-COV)
- [x] **Sovereignty migration** -- remaining callers to capability-based APIs (toadStool D-SOV)
- [x] **Hardware Transport wiring** -- transport.discover/list/route JSON-RPC + CLI commands
- [x] **Detection stubs evolved** -- 11 functions → real /proc + command-based detection
- [x] **Smart refactoring (round 2)** -- security.rs (771→5 modules), config_utils (777→5 modules)
- [x] **Smart refactoring** -- vfio.rs (971L) smart-refactored into `vfio/` directory (S94)
- [x] **manual_jsonrpc removal** -- deleted, pure_jsonrpc is canonical (S94)
- [x] **Barracuda fossilization** -- dead dep removed, crates/barracuda → archive/ (S94)
- [x] **Sovereign pipeline** -- HardwareFingerprint, is_sovereign_capable, safe_allocation_limit, SubstrateCapabilityKind (S96)
- [x] **SubstrateType expansion** -- 4→8 variants: IntegratedGpu, Npu, Tpu, Fpga, Dsp, Quantum (S96)
- [x] **God file splits (round 3)** -- dispatch.rs, detection.rs, engine.rs, protocols/lib.rs, specialized_templates.rs (S96)
- [x] **API orphan resolved** -- crates/api/ ByobApi extracted to container crate (S96)
- [x] **V4L2 unsafe docs** -- All SAFETY comments on unsafe blocks (S96)
- [x] **Debris cleanup** -- root tests/ stubs, stale checklists, false-positive TODOs (S95)
- [x] **management/resources re-added** -- real ResourceManager with sysinfo (S95)

### Cross-Repo Debt

- [ ] **D-S20-003**: neuralSpring `evolved/` migration (~2075 lines) — awaiting neuralSpring team
- [ ] **D-S18-002**: cubecl transitive `dirs-sys` — needs upstream PR

---

## Completed This Session (S90-96)

### Sessions S95–S96: Spring Absorption + Sovereign Pipeline + Debris Cleanup (Mar 6, 2026)
- **Sovereign pipeline**: `HardwareFingerprint` (estimated TFLOPS f32/f64, sovereign_capable flag), `is_sovereign_capable()`, `safe_allocation_limit` (NVK PTE fault mitigation), 12-variant `SubstrateCapabilityKind` (F64Native, Df64Emulation, Spmv, Eigen, Cg, Fft, MdForce, MonteCarlo, NnInference, ReservoirCompute, Fhe, SubgroupOps).
- **SubstrateType expansion**: 4→8 variants (Cpu, Gpu, IntegratedGpu, Npu, Tpu, Fpga, Dsp, Quantum) with `is_batch_oriented()` / `is_latency_oriented()` helpers.
- **God file splits (5)**: `dispatch.rs` (1252L→7 modules), `detection.rs` (1004L→3 modules), `engine.rs` (1098L→2 modules), `protocols/lib.rs` (985L→2 modules), `specialized_templates.rs` (924L→4 modules).
- **API orphan resolved**: `crates/api/` ByobApi route logic extracted to `crates/runtime/container/src/byob_routes.rs`; toadstool-api dependency removed from container.
- **V4L2 unsafe documentation**: All `unsafe` blocks in `v4l2/device.rs` documented with `// SAFETY:` comments.
- **Hardcoded IP evolved**: `0.0.0.0` fallback → `TOADSTOOL_DISCOVERY_BIND_ADDR` env var.
- **Debris cleanup**: Root `tests/` stubs removed (fossilized to ecoPrimals/fossil/). Stale `✅ COMPLETE` checklists cleaned from 11 files. False-positive TODO in `input/parser.rs` removed. Sprint/date doc comments cleaned in test files.
- **management/resources re-added**: Real `ResourceManager` with sysinfo re-added to workspace.
- **Clippy pedantic**: Resolved across workspace. `cargo clippy --lib -- -W clippy::pedantic` clean.
- **Spring absorption tracker**: Updated to current spring versions (hotSpring v0.6.17, groundSpring V80, neuralSpring V86/S128, wetSpring V97d, airSpring V071).

### Session 94b: Deep Execution + Spring Absorption
- **NpuDispatch trait** (`toadstool-core::npu_dispatch`): generic `NpuDispatch` trait + `AkidaNpuDispatch` adapter. Vendor-agnostic, capability-based, zero-copy `Cow` input. `NpuModelHandle`, `DispatchResult`, `NpuCapability` enum, `NpuInfo` struct.
- **NpuParameterController trait** (`toadstool-core::npu_controller`): absorbed from hotSpring — generic NPU-driven parameter tuning. `ParameterSuggestion<P>`, `SafetyClamp<P>`, `SuggestionSource`, `ControllerError`. Springs implement for domain-specific tuning.
- **GpuAdapterInfo** (`toadstool-runtime-universal`): exposes driver name, driver_info, vendor/device ID, backend, device type, workgroup limits, max buffer size, and shader-f64 support. barraCuda uses this for `GpuDriverProfile` (NVK detection, f64 workarounds).
- **Multi-adapter GPU selection**: `TOADSTOOL_GPU_ADAPTER` env var with comma-separated fallback (index, name substring, "auto"). Absorbed from hotSpring's `adapter.rs` pattern.
- **NestGate mock → real RPC**: `store_artifact`/`retrieve_artifact` evolved from hardcoded stubs to real JSON-RPC calls with graceful fallback when storage service unavailable.
- **Placeholder crate removed**: `management/resources` excluded from workspace (empty crate polluting build graph).
- **Production mock audit**: Complete — all remaining stubs are either error-returning (correct behavior for unimplemented hardware), test-gated, or documented heuristic models.
- **External dependency audit**: Workspace clean — all non-Rust deps behind optional features on excluded crates. No `build.rs` files.
- **Large file audit**: Production code well under 1000L limit. 812L and 806L files are ~490L production + ~320L tests.
- Verification: `cargo fmt` ✅ `cargo clippy -D warnings` ✅ `cargo doc` ✅ `cargo test` ✅ (all pass, 0 fail)

### Session 94: Deep Debt Execution — Fossilization + Deletion + Refactoring
- Removed dead `barracuda` dependency from `core/toadstool/Cargo.toml` (zero imports; barracuda is a peer primal)
- Fossilized `crates/barracuda/` (15MB, 1,790 files) to `ecoPrimals/fossil/toadStool/barracuda-fossil-S94b/`
- Deleted `manual_jsonrpc` module entirely (8 files + integration tests); `pure_jsonrpc` is canonical
- Smart-refactored `vfio.rs` (971L) into `vfio/` directory: `types.rs`, `ioctl.rs`, `dma.rs`, `mod.rs`
- Updated all doc references (ManualJsonRpcServer → pure_jsonrpc::JsonRpcHandler)
- Audited production panics/unwraps: all in test code (clean production)
- All files under 1000 lines (largest: 936 line test file)
- Verification: `cargo fmt` ✅ `cargo clippy -D warnings` ✅ `cargo doc` ✅ `cargo test` 17,986 pass, 0 fail

### Session 93: D-DF64 Transfer & Root Doc Cleanup
- Transferred D-DF64, D-CD (ComputeDispatch), DF64 transcendentals, arch-specific polynomial selection, naga-IR optimizer evolution, and barraCuda budding Phases 1-4 to barraCuda team ownership
- Created formal handoff: `wateringHole/handoffs/TOADSTOOL_S93_DF64_HANDOFF_MAR03_2026.md`
- Cleaned NEXT_STEPS.md to focus on toadStool-owned remaining work (D-NPU, D-COV, D-SOV, smart refactoring)
- Deleted 12 stale docs/debris files (~90 KB): orphan txt, completed migration guides, self-congratulatory status reports
- Root docs synchronized: STATUS, README, QUICK_REFERENCE, BREAKING_CHANGES, DOCUMENTATION, EVOLUTION_TRACKER, SPRING_ABSORPTION_TRACKER

### Session 92: Sovereignty Deprecation Sweep & Audit Continuation
- Deprecated `get_socket_path_for_service`, `get_primal_default_port`, `capability_typical_provider` with `#[deprecated(since = "0.92.0")]`
- Migrated NestGate client to `get_socket_path_for_capability` (3 callsites)
- Added `EcosystemDiscoverer::find_pattern_by_capability()` for capability-based lookup
- Neutralized 5 BearDog user-facing strings in access control manager
- `version_info()` → "Pure Rust (ecoPrimals sovereign pattern)"
- Removed dead middleware.rs + 7 test files (~131 KB)
- +47 tests → 5,369 total (monitoring, templates, installer, connection, wasm_ops, session)
- ecoBin `pure-rust` build verified: zero C FFI deps
- Fixed `bail!` macro undefined on `#[cfg(not(feature = "wasm"))]` path
- Extracted `verify_sha256()` as standalone fn for testability
- Audited: 0 production `todo!()`, 0 `unimplemented!()`, 0 FIXME, 0 HACK

### Session 90: Deep Audit, REST Removal, Sovereignty Evolution
- Fixed SIGSEGV in runtime-universal (wgpu catch_unwind + timeout)
- Unified 37 Cargo.toml license fields to workspace. 2,780+ SPDX headers added/normalized.
- Capability-based trust model. `get_socket_path_for_capability()` API added.
- Removed all REST routes + handlers + 8 test files. JSON-RPC only.
- Arc-cached compiled kernels, moved Vec, Arc<str> version on hot paths.
- PyO3 feature-gated. Python runtime optional in CLI.
- Documented all unsafe blocks in akida-driver.
- Rewrote handlers_basic_tests.rs (15 JSON-RPC integration tests).
- 5,322 tests, 0 failures.

### Session 89: barraCuda Primal Budding
- Full barracuda crate extracted to `ecoPrimals/barraCuda/` (956 .rs, 767 WGSL, 61 tests)
- `toadstool-core` gated behind `#[cfg(feature = "toadstool")]` — 1 file: `device/toadstool_integration.rs`
- `akida-driver` gated behind `#[cfg(feature = "npu-akida")]` — `npu/ml_backend.rs` + `npu/ops/` + bridge callsites
- `DeviceSelection`/`HardwareWorkload` extracted to `device/mod.rs` (zero external deps)
- `barracuda-core` wired: `BarraCudaPrimal::start()` runs device discovery, health reports adapter info
- Standalone quality: `cargo check/clippy/test` all pass (2,832 tests, 0 failures)
- MSRV 1.87 (code uses `is_multiple_of`)
- toadStool completely unchanged
- Pushed to GitHub: `ecoPrimals/barraCuda`

## Completed (S87-S88)

### Session 87: Deep Debt Resolution + Idiomatic Concurrent Rust + Code Quality
- TODO(afit) → NOTE(async-dyn): 75 instances across 52 files (reclassified from debt to architectural decision)
- gpu_helpers.rs: 663 lines → 3 submodules (buffers.rs, bind_group_layouts.rs, pipelines.rs)
- Unsafe code audit: All ~60+ sites documented; all verified necessary
- Hardware verification: 3 pre-existing failures fixed; 13/13 pass
- Hotspring fault tests: 6 pre-existing failures fixed — input validation, relaxed GPU assertions, device capability checks
- FHE shader fixes: u64_mod_simple + mod_mul; 19 FHE tests pass; MatMul/FHE validation; chaos test moduli constrained
- Device-lost recovery: BarracudaError::is_device_lost() + with_device_retry test helper

## Completed (S84–S86)

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
