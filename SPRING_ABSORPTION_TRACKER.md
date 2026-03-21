# Spring Absorption Tracker

**Session**: S162 Coverage Expansion + Code Quality (March 21, 2026)
**ToadStool**: master, ~82.81% line coverage (llvm-cov re-baselined). **21,600+ tests**, 0 failures. Clippy pedantic clean (0 warnings across all 58 crates). AGPL-3.0-only. Zero C FFI deps (ecoBin v3.0). **43/43 crates** with `unsafe_code` lint policy (23 forbid + 20 deny). Zero production unwraps. All quality gates passing. S162.
**S156**: Full codebase audit: runtime-specialty resurrected (167 compile errors → 0), hardcoding → named constants, unsafe → safe error handling, doc warnings fixed, 17.4 GB build garbage cleaned. +313 net new tests.
**S153**: IPC-first reconciliation (barraCuda v0.35 zero compile-time deps), `compute.hardware.vfio_devices`, `ecoprimals-mode` CLI, hotSpring VFIO validation absorption (6/7 tests on Titan V pass, dispatch blocked on coralReef USERD_TARGET fix). Spring pins: hotSpring v0.6.31, coralReef Iter 43, barraCuda v0.35.
**S149**: Deep debt execution — clippy 0 warnings, resolve_credential() fully wired (file-based + security provider RPC), handler refactored (shader extracted), orphaned deps removed, nouveau_drm evolved to rustix, hw-learn stubs → real implementations, interned constants everywhere. Spring pins: hotSpring v0.6.30, neuralSpring V98/S145, coralReef Iter 35.
**S146**: Deep evolution execution — nvvm_transcendental_risk exposed in gpu.info JSON-RPC response (hotSpring v0.6.26 request). PrecisionBrain wired into compile_wgsl_multi flow (per-device precision advice prevents NVVM poisoning). PcieTopologyGraph marked stable for spring consumption (#[non_exhaustive] + empty() constructor). SpringDomain expanded (+10 new domains: Pharmacokinetics, Biosignal, Microbiome, Agriculture, Environmental, Phylogenetics, MassSpectrometry, UncertaintyQuantification, EvolutionaryComputation, Optimization) with SCREAMING_SNAKE_CASE as_str() per wetSpring V109 convention. HealthSpring added to Spring enum. gpu_memory_estimate_bytes on WorkloadPattern + route_with_vram for VRAM-aware scheduling (healthSpring V19). Cross-spring provenance expanded (healthSpring PK/PD + DiversityIndex flows). Spring pins updated.
**S145**: Spring absorption execution — PrecisionBrain absorbed from hotSpring v0.6.25 (domain-aware routing brain, F64 throttle detection, PrecisionHint enum). NvkZeroGuard absorbed from airSpring v0.7.5 (zero-output detection, NaN contamination). 8 new WorkloadPatterns from neuralSpring S140 + healthSpring V14.1 (Pairwise, BatchFitness, HmmBatch, SpatialPayoff, Stochastic, PopulationPk, DoseResponse, DiversityIndex). 5 new capability domains (biology, health, measurement, optimization, visualization). capability.call format B (qualified_method). Spring-as-Provider ProviderRegistry (ISSUE-007). ServerConfig port hardcoding evolved.
**S140**: Deep debt evolution — hardcoding elimination (beardog_impl, unibin/format, sandbox, primal_capabilities, display/ipc, cli/main, zero_config/discovery all evolved to interned_strings). StreamingDispatchContext enriched with healthSpring V13 execute_streaming() callback (StageProgress, ProgressCallback). barraCuda Sprint 2 API awareness (science.activations.list, science.rng.capabilities, science.special.functions JSON-RPC methods). Smart refactor: science.rs 1139→828 LOC via science_domains.rs extraction. New tests: 4 streaming dispatch + 3 barraCuda API handlers.
**S139**: Spring absorption — surveyed all 6 springs. Dual-write discovery (coralReef compat). gpu.dispatch + science.gpu.dispatch capabilities. GpuDevice enriched with render_node/driver/arch. StreamingDispatch absorbed from hotSpring v0.6.24. PipelineGraph DAG absorbed from neuralSpring S134. Compute triangle unblocked on toadStool side.
**S138**: Deep debt execution — +126 tests (sysmon 53, science handler 38, primal discovery 14, bear_dog 10, mdns 4, integrator 5, unibin 2). License alignment AGPL-3.0-only. 62 allow entries removed. 19 clone reductions. Hardcoding evolution to interned_strings. 7 stale TODO(D-PEDANTIC) removed. SPRING_ABSORPTION.md archived to fossil (superseded by this tracker).
**S135**: groundSpring V100 absorption (SubstrateCapabilityKind::SovereignCompile). GPU f64 reduction smoke test. All hardcoded primal names evolved to interned_strings constants. precision_defaults module. Pre-existing debt fixes.
**S134**: BearDog crypto delegation enforced (Node Atomic). `dev-crypto` feature gate. secure_enclave aes-gcm/getrandom removed. lifecycle_ops + management refactored.
**S133**: Cross-spring absorption from all 5 springs (Mar 8 handoffs). Ada Lovelace `F64NativeNoSharedMem` reclassification (P0). `f64_zeros_risk` + `fused_ops_healthy()` canary on `GpuAdapterInfo`. 14 airSpring ecology JSON-RPC methods. NUCLEUS adaptive discovery pattern (groundSpring V99). Deploy graph routing (wetSpring V99). +20 semantic methods (71→91). biomeOS socket scan on startup.

## Spring Pin Status

| Spring | Version | Previous Pin | Current Pin | Tests | Delegations |
|--------|---------|--------------|-------------|-------|-------------|
| hotSpring | v0.6.31 | S145→S146 | S146→S147 | 769 lib + 101 binaries + 84 WGSL | Chuna 44/44, Fp64Strategy::Sovereign, PrecisionRoutingAdvice, PrecisionBrain, HardwareCalibration, streaming_dispatch, spirv_codegen_safety (renamed from nvvm), FirmwareInventory, sovereign DRM breakthrough |
| groundSpring | V100 | V99→S140 | V100→S146 | 936 + 382 Python | 102 (61 CPU + 41 GPU), live NUCLEUS (4 primals), adaptive health, direct primal sockets, SubstrateCapabilityKind::SovereignCompile |
| neuralSpring | V96/S143 | V91/S140→S145 | V96/S143→S146 | 902 lib + 43 forge + 240 bins | petalTongue visualization, 8 new WorkloadPatterns, DataChannel, ParallelEighDispatch noted (barraCuda-level) |
| wetSpring | V109 | V99+→S145 | V109→S146 | 1,047 + 200 forge | petalTongue V2, DiversityIndex pattern, DataChannel::Spectrum, SpringDomain SCREAMING_SNAKE_CASE convention adopted |
| airSpring | v0.7.5 | v0.7.5→S140 | v0.7.5→S146 | 865 lib + 186 forge | 14 JSON-RPC science methods, NvkZeroGuard absorbed, 35 capabilities |
| healthSpring | V19 | V14.1→S145 | V19→S146 | 317+ tests | PopulationPk + DoseResponse + DiversityIndex thresholds, NLME dispatch, gpu_memory_estimate_bytes adopted, MichaelisMentenBatch/ScfaBatch/BeatClassifyBatch noted |

## Absorption Status

### P0 — Correctness (COMPLETE)

| Item | Source | Status |
|------|--------|--------|
| `anderson_4d` + `wegner_block_4d` re-export | groundSpring V68 | **DONE** |
| `SeasonalGpuParams::new()` constructor | groundSpring V68 | **DONE** |
| `BREAKING_CHANGES.md` | groundSpring, wetSpring | **DONE** |
| Feature-gate CI (`cargo check` without features) | wetSpring, groundSpring | **DONE** |

### P1 — API Gaps (COMPLETE)

| Item | Source | Status |
|------|--------|--------|
| `MultiHeadEsn::from_exported_weights()` | hotSpring | **DONE** |
| Cross-spring named tolerances expansion | airSpring, wetSpring | **DONE** |
| `NeighborMode` 4D index convention docs | hotSpring | **DONE** |
| SU(3) DF64 shader sync verification | hotSpring | **DONE** |

### P2 — Shader Evolution (COMPLETE)

| Item | Source | Status |
|------|--------|--------|
| L-BFGS GPU (batched numerical gradient) | groundSpring | **DONE** |
| Tridiag QL eigenvector solver | groundSpring | **DONE** |

### P3 — Sovereign Pipeline (NEW — from Mar 5-6 handoffs)

| Item | Source | Status |
|------|--------|--------|
| `is_sovereign_capable` API on GPU adapter | coralReef/hotSpring | **DONE** S96 |
| `HardwareFingerprint` with estimated TFLOPS | hotSpring V0617 | **DONE** S96 |
| NVK ~1.2 GB allocation guard in `GpuAdapterInfo` | hotSpring/Titan V gaps | **DONE** S96 |
| NVK Volta f64-returns-zeros detection | airSpring/hotSpring | **DONE** S97 |
| Subgroup size detection in `GpuAdapterInfo` | airSpring V071 | **DONE** S97 |
| 2D dispatch threshold helper | hotSpring | **DONE** S97 |
| `AdaptiveSimulationController` trait | hotSpring NPU worker | **DONE** S97 |
| `ProxyFeature` / `NpuInferenceRequest` types | hotSpring NPU worker | **DONE** S97 |
| `science.*` JSON-RPC namespace (10 methods) | wetSpring IPC | **DONE** S97 |
| coralDriver routing for sovereign dispatch | coralReef Phase 5 | Tracked (blocked on coralDriver) |
| Substrate capability expansion (4→12 variants) | metalForge (all springs) | **DONE** S96 |
| `f64_shared_memory_reliable` on `GpuAdapterInfo` | groundSpring V84-V85 | **DONE** S128 |
| `sovereign_binary_capable` on `HardwareFingerprint` | groundSpring V85 | **DONE** S128 |
| `PrecisionRoutingAdvice` enum + `precision_routing()` | groundSpring V84-V85 | **DONE** S128 |
| `shader.compile.*` JSON-RPC namespace (4 methods) | coralReef handoff | **DONE** S128 |
| `discover_capabilities` dynamically from registry | deep debt | **DONE** S128 |
| `query_available_backends()` runtime probing | deep debt | **DONE** S128 |
| Architecture stubs → typed implementations | deep debt | **DONE** S128 |
| coralReef shader proxy (`shader.compile.*` → real proxy) | coralReef Phase 10, S130 | **DONE** S130 |
| Cross-spring provenance tracking (17+ flows) | all springs | **DONE** S130 |
| `toadstool.provenance` JSON-RPC method | cross-spring | **DONE** S130 |
| Clippy pedantic zero (workspace-wide) | deep debt | **DONE** S130+ |

### P3 — New Items (S140 — Mar 9-10 Spring Sync)

| Item | Source | Status |
|------|--------|--------|
| Hardcoding evolution: beardog_impl, unibin/format, sandbox, primal_capabilities, display/ipc, cli/main, zero_config/discovery → interned_strings | deep debt | **DONE** S140 |
| StreamingDispatchContext enriched with StageProgress + ProgressCallback | healthSpring V13 | **DONE** S140 |
| barraCuda Sprint 2 API awareness (activations, rng, special) wired as JSON-RPC methods | barraCuda v0.3.3 Sprint 2 | **DONE** S140 |
| science.rs smart refactor (1139→828 LOC) via science_domains.rs extraction | deep debt | **DONE** S140 |
| healthSpring V13 pinned in tracker | healthSpring V13 | **DONE** S140 |
| Fp64Strategy::Sovereign awareness | hotSpring v0.6.24 | Tracked (shader.compile.* proxy already handles; needs coralReef NVIDIA E2E) |
| barraCuda `rng::lcg_step` consumption | barraCuda Sprint 2 | Noted — toadStool has no local LCG; awareness exposed via JSON-RPC |
| DataChannel types for compute-to-viz routing | wetSpring/neuralSpring | Tracked (petalTongue integration) |
| BatchReconcileGpu (DTL reconciliation) | wetSpring P2 | Tracked (blocked on barraCuda implementation) |

### P3 — Items (from Mar 7 S131+ handoffs)

| Item | Source | Status |
|------|--------|--------|
| `SubstrateCapabilityKind::Fft` already exists | groundSpring V96 request | **DONE** (already present since S96) |
| coralReef E2E AMD dispatch (WGSL→RDNA2→PM4→GPU→readback, pure Rust) | coralReef Phase 10/Iter 10 | **Milestone noted** — sovereign pipeline proven |
| `From<BarracudaError>` for typed error propagation | barraCuda v0.3.3 | Noted — available for cleaner error chains |
| `#[allow]` → `#[expect]` in production code | neuralSpring pattern | **DONE** S131+ |
| wetSpring `science.*` namespace: toadStool proxy confirmed canonical | wetSpring V97e | **DONE** — springs may call barraCuda directly or via toadStool proxy |
| Fp64Strategy fused GPU regression (VarianceF64/CorrelationF64 return 0.0) | all springs | Tracked (barraCuda P0 blocker) |
| `compile_wgsl_direct()` for sovereign metalForge compilation | neuralSpring V89 | Tracked (needs coralReef NVIDIA E2E) |
| `Tensor::mean()` entry point | neuralSpring V89 | Tracked |

### P3 — Shader Evolution (tracked)

| Item | Source | Status |
|------|--------|--------|
| Flash attention WGSL | neuralSpring | Tracked |
| Fused LayerNorm+GELU WGSL | neuralSpring | Tracked |
| Fused LSTM cell WGSL (streaming) | neuralSpring V86 | Tracked |
| Deformed HFB full wiring (5 WGSL) | hotSpring | Tracked |
| Abelian Higgs U(1)+Higgs (3 WGSL) | hotSpring | Tracked |
| Richards PDE GPU full wiring | airSpring, groundSpring | Tracked |
| Multi-GPU interconnect | neuralSpring | Tracked |
| Autocorrelation GPU op | neuralSpring V86 | Tracked |
| R² score GPU op | neuralSpring V86 | Tracked |
| SCS-CN, Stewart, Blaney-Criddle (3 ops) | airSpring V0.7.3 | **DONE** (absorbed upstream as ops 14-19) |
| Fused GPU seasonal pipeline (chain ops 0→7→1, no CPU round-trips) | airSpring V071 | Tracked |
| `UnidirectionalPipeline` for atlas_stream multi-year streaming | airSpring V071 | Tracked |

### P4 — Future (lower priority)

| Item | Source | Status |
|------|--------|--------|
| NPU substrate kind in metalForge | neuralSpring | Open |
| Streaming FASTQ/mzML/MS2 (bio I/O) | wateringHole | Open |
| Pseudofermion HMC (477 lines) | wateringHole | Open |
| Omelyan integrator | wateringHole | Open |
| Richards PDE (12 USDA textures) | wateringHole | Open |
| Provenance tags | hotSpring, groundSpring, neuralSpring | Open |
| Generic `NvkZeroGuard` wrapper concept | airSpring v0.7.5 NVK pattern | **DONE** S145 (nvk_zero_guard_check + ZeroGuardVerdict in spirv_codegen_safety.rs; renamed from nvvm_safety S147) |
| llvmpipe fused shader dispatch investigation (12 tests return 0.0) | neuralSpring V87 | Open (barraCuda-owned) |
| `CoralReefDevice` backend in barraCuda (sovereign SASS dispatch) | groundSpring V95 | Open (barraCuda-owned) |
| QMD constant buffer binding (coralReef P0 blocker) | groundSpring V95 | Open (coralReef-owned) |
| `SumReduceF64` / `VarianceReduceF64` Fp64Strategy branching fix | groundSpring V95 | Open (barraCuda-owned) |
| wetSpring `special::{erf, ln_gamma, dot, l2_norm}` absorption | wetSpring V97d | Open (barraCuda-owned) |

## New Handoff Items (Mar 7, 2026 — S131+ Spring Sync)

### From hotSpring v0.6.19 (unchanged from previous sync)

| Item | Impact |
|------|--------|
| DF64 compilation fully delegated to barraCuda | No toadStool action — hotSpring uses `compile_shader_universal(Precision::Df64)` |
| 3 Chuna papers (43-45) CPU-complete | GPU promotion is barraCuda-owned |
| Cross-spring GPU benchmarks validated | Autocorrelation, Mean+Variance, Correlation, Chi-squared via barraCuda |
| `GpuView<T>` adoption target | barraCuda-owned; eliminate per-call buffer upload/download |

### From groundSpring V96 (NEW)

| Item | Impact |
|------|--------|
| PrecisionRoutingAdvice wired into 11 GPU dispatch paths | All f64 workgroup reductions check `get_device_f64_safe()` before dispatch |
| 925 tests (was 907), 102 delegations (61 CPU + 41 GPU) | Ecosystem health confirmed |
| Stale clippy fix + collapsible_if | Internal quality |
| Requests: FFT in `SubstrateCapabilityKind` | **Already present** (`Fft` variant since S96) |
| Requests: wire `shader.compile.*` to live coralReef | **Already complete** — `CoralReefClient` proxies to coralReef when available |

### From neuralSpring V89/S131 (NEW)

| Item | Impact |
|------|--------|
| Full green: 901 lib + 43 forge tests, 89.1% coverage | Zero debt, zero TODO/FIXME/MOCK in src/ |
| Isomorphic catalog fix: 25/25 (100%) primitive coverage | WGSL discovery expanded to barraCuda + metalForge + Tensor ops |
| P0: Fp64Strategy fused GPU regression still present | 11 tests gated via canary; barraCuda P0 |
| Requests: `compile_wgsl_direct()` for sovereign compilation | Needs coralReef NVIDIA E2E first |
| Requests: `Tensor::mean()` entry point | Tracked |
| Pattern: `#[expect(lint, reason)]` instead of bare `#[allow]` | **Adopted in toadStool S131+** |

### From wetSpring V97e (NEW)

| Item | Impact |
|------|--------|
| 1,346 tests, builder pattern migration complete | All barraCuda breaking API changes absorbed |
| PrecisionRoutingAdvice wired in `GpuF64::optimal_precision()` | 4-tier routing (F64Native/NoSharedMem/Df64Only/F32Only) |
| Provenance API: `shaders_authored()`, `shaders_consumed()`, `wetspring_provenance_summary()` | Cross-spring tracking |
| IPC question: `science.*` overlap with toadStool S130 | **Resolved**: toadStool is canonical proxy; springs may also call barraCuda directly |
| Re-export suggestions for barraCuda (HmmForwardArgs, Dada2DispatchArgs, etc.) | barraCuda-owned |

### From airSpring V0.7.3 (NEW)

| Item | Impact |
|------|--------|
| Write→Absorb→Lean COMPLETE: all 6 local ops absorbed upstream | Zero local WGSL, fully lean on barraCuda |
| PrecisionRoutingAdvice integrated in `DevicePrecisionReport` | NVK shared-mem routing captured |
| Upstream provenance registry integrated | `upstream_airspring_provenance()`, `upstream_cross_spring_matrix()` |
| Requests: `shader.compile.wgsl` via toadStool JSON-RPC | **Already available** — `CoralReefClient` proxies when coralReef running |

### From barraCuda v0.3.3 (NEW)

| Item | Impact |
|------|--------|
| Typed error handling: `BarracudaCoreError` with `From<BarracudaError>` | Available for cleaner error chains at toadStool boundary |
| Hardcoding → named constants (`LOCALHOST`, `JSONRPC_VERSION`, memory fallbacks) | Pattern aligned with toadStool constants |
| 38 bare `#[allow(dead_code)]` → `#[allow(dead_code, reason = "...")]` | Matched by toadStool S131+ `#[expect]` evolution |
| 3,100+ tests, zero unsafe, zero clippy warnings | Quality gates aligned |

### From coralReef Phase 10/Iteration 10 (NEW — MILESTONE)

| Item | Impact |
|------|--------|
| **First E2E sovereign GPU dispatch on AMD RX 6950 XT** | WGSL → coral-reef compiler → PM4 dispatch → GPU execution → host readback, all pure Rust |
| 3 critical bugs fixed: wave size, literal constant emission, 64-bit address | Compiler and driver hardened |
| 990 tests (953 passing, 37 ignored hw tests) | Quality gates green |
| `ComputeDispatch::CoralReef` routes via JSON-RPC IPC (barraCuda v0.35 IPC-first) | toadStool's shader proxy can now produce real native binaries on AMD |
| NVIDIA E2E still pending | Next coralReef milestone |

## Historical Handoff Items (Mar 5-6, 2026)

Items from earlier sync sessions. Retained for cross-reference.

### From hotSpring v0.6.17-v0.6.19

| Item | Status |
|------|--------|
| DF64 delegation, cross-spring GPU benchmarks, Chuna papers | Absorbed S128-S130 |
| `gradient_flow.rs`, `brain.rs`, 6 Verlet WGSL shaders | barraCuda-owned |
| Kokkos parity: dispatch overhead dominates | barraCuda-owned (persistent buffers) |

### From groundSpring V80-V95

| Item | Status |
|------|--------|
| Fused `correlation_full` GPU, Welford CPU stats | barraCuda stats absorbed |
| metalForge: 30 workloads, 187 checks | Hardware validation corpus (active) |
| coralReef Phase 11 push buffer fix | coralReef-owned (fixed) |

### From neuralSpring V86-V87/S128-S129

| Item | Status |
|------|--------|
| 46 upstream rewires, struct-based API migration | Complete |
| coralForge structure prediction, baseCamp biophysical AI | Independent catalogs |
| 12 GPU test failures (fused shaders return 0.0) | barraCuda P0 (Fp64Strategy regression) |

### From wetSpring V97d

| Item | Status |
|------|--------|
| 0 local WGSL (fully lean), fused ops chain | Reference pattern |
| Bio Brain, IPC science primal | Active |

### From airSpring V071

| Item | Status |
|------|--------|
| 6 local ops absorption | **DONE** (all absorbed upstream as ops 14-19 in V0.7.2-V0.7.3) |
| Nautilus/AirSpringBrain | Active |

### From Sovereign Pipeline (coralReef Phases 1-10)

| Item | Status |
|------|--------|
| coralReef Phases 1-5 (NVIDIA WGSL→SASS) | Complete |
| coralReef Phase 10 (AMD WGSL→RDNA2) | **E2E PROVEN** on RX 6950 XT |
| coralDriver (Level 4 sovereign compute) | Blocked on QMD CBUF binding |

## Cross-Spring Patterns

| Pattern | Springs | Resolution |
|---------|---------|------------|
| GPU-resident state (no readback) | airSpring, groundSpring, hotSpring | `BatchedStatefulF64` exists; needs docs |
| Breaking changes tracking | groundSpring, wetSpring | `BREAKING_CHANGES.md` created |
| Feature-gate discipline | wetSpring, groundSpring | CI check added |
| Fused pipeline chains | airSpring, wetSpring | `UnidirectionalPipeline` exists |
| Shared named tolerances | airSpring, wetSpring | Expanded S88 |
| Write→Absorb→Lean lifecycle | all springs | wetSpring fully lean; pattern validated |
| metalForge hardware validation | all springs | Each spring has forge crate; toadStool bridges |
| Sovereign compute pipeline | hotSpring, coralReef | WGSL→SPIR-V→coralReef; NVK bypass active |

## Handoff Cross-Reference

### Incoming (to ToadStool) — Mar 7, 2026

| Handoff | From | Key Items |
|---------|------|-----------|
| `GROUNDSPRING_V96_UPSTREAM_REWIRE` | groundSpring V96 | PrecisionRoutingAdvice wired, 925 tests, FFT capability request |
| `NEURALSPRING_TOADSTOOL_V89_S131_FULL_GREEN` | neuralSpring V89 | 89.1% coverage, isomorphic 25/25, Fp64Strategy P0 blocker |
| `NEURALSPRING_TOADSTOOL_V88_S130_UPSTREAM_REWIRE` | neuralSpring V88 | Upstream rewire, `#[expect]` pattern |
| `WETSPRING_V97E_BARRACUDA_TOADSTOOL_EVOLUTION` | wetSpring V97e | Builder patterns, science.* IPC overlap question, re-export suggestions |
| `WETSPRING_V97E_PROVENANCE_REWIRE` | wetSpring V97e | Provenance API, precision routing, 1,346 tests |
| `AIRSPRING_V073_MODERN_INTEGRATION` | airSpring V0.7.3 | Write→Absorb→Lean complete, provenance, PrecisionRouting |
| `AIRSPRING_V073_BARRACUDA_TOADSTOOL_EVOLUTION` | airSpring V0.7.3 | Cross-spring shader flows, NVK shared-mem findings |
| `BARRACUDA_V033_COMPREHENSIVE_AUDIT_EVOLUTION` | barraCuda v0.3.3 | Typed errors, hardcoding→constants, 3,100+ tests |
| `CORALREEF_PHASE10_ITERATION10_E2E_GPU_DISPATCH` | coralReef Phase 10 | **First E2E AMD dispatch**, 3 critical bugs fixed, pure Rust pipeline |
| `HOTSPRING_V0619_BARRACUDA_REWIRE_MAR06` | hotSpring v0.6.19 | DF64 delegation complete, `GpuView<T>` target |
| `HOTSPRING_V0619_CROSS_SPRING_EVOLUTION_MAR06` | hotSpring v0.6.19 | 3 Chuna papers, cross-spring GPU benchmarks |

### Outgoing (from ToadStool)

| Handoff | To | Session |
|---------|----|---------|
| `TOADSTOOL_S131_SPRING_SYNC_DEEP_DEBT_MAR07` | Ecosystem | S131+ |
| `TOADSTOOL_S130_DEEP_DEBT_EXECUTION_MAR07` | Ecosystem | S130+ |
| `TOADSTOOL_S128_DEEP_DEBT_EVOLUTION_MAR06` | Ecosystem | S128 |

## metalForge Clarification

**metalForge = silicon characterization**, not Apple Metal API. All GPU work uses **WGSL via wgpu** (Vulkan/Metal/DX12 backends). Apple Metal is transparently handled by the wgpu abstraction layer. metalForge probes the actual hardware substrate: GPU (wgpu), CPU (/proc), NPU (/dev).
