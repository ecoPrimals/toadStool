# Spring Absorption Tracker

**Session**: S130+ Clippy Pedantic Clean + Spring Sync (March 7, 2026)
**ToadStool**: master, ~83% line coverage (576K lines, 1,868 .rs files). 19,536 tests, 0 failures. Clippy pedantic clean. coralReef shader proxy with capability-based discovery. Cross-spring provenance tracking. 45+ god files refactored. All quality gates passing.

## Spring Pin Status

| Spring | Version | Previous Pin | Current Pin | Tests | Delegations |
|--------|---------|--------------|-------------|-------|-------------|
| hotSpring | v0.6.19 | S97→S128 | S128→S130+ | 724 lib + 19 integration | Lattice QCD, MD reservoir, NPU, gradient flow, Verlet, DF64 delegation complete, 3 Chuna papers CPU-validated |
| groundSpring | V95 | V85→S128 | V95→S130+ | 907 + 390 forge | 102 (61 CPU + 41 GPU), metalForge 30 workloads, coralReef Phase 11 push buffer fix |
| neuralSpring | V87/S129 | V86/S128→S128 | V87/S129→S130+ | 883 lib + 240 bins | 46 upstream rewires, struct-based API migration complete, `#![forbid(unsafe_code)]` |
| wetSpring | V97d+ | V97d→S128 | V97d+→S130+ | 1,047 + 200 forge | 150+ primitives, 0 local WGSL (fully lean), zero API breakage confirmed |
| airSpring | V071 | V071→S128 | V071→S130+ | 850 + 61 forge | 21 Tier A GPU + 6 universal, 3 local WGSL ops, NVK zero-output detection |

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
| SCS-CN, Stewart, Blaney-Criddle (3 ops) | airSpring local WGSL | Tracked |
| Fused GPU seasonal pipeline (chain ops 0→7→1, no CPU round-trips) | airSpring V071 | Tracked (NEW) |
| `UnidirectionalPipeline` for atlas_stream multi-year streaming | airSpring V071 | Tracked (NEW) |

### P4 — Future (lower priority)

| Item | Source | Status |
|------|--------|--------|
| NPU substrate kind in metalForge | neuralSpring | Open |
| Streaming FASTQ/mzML/MS2 (bio I/O) | wateringHole | Open |
| Pseudofermion HMC (477 lines) | wateringHole | Open |
| Omelyan integrator | wateringHole | Open |
| Richards PDE (12 USDA textures) | wateringHole | Open |
| Provenance tags | hotSpring, groundSpring, neuralSpring | Open |
| Generic `NvkZeroGuard` wrapper concept | airSpring V071 NVK pattern | Open (barraCuda-owned) |
| llvmpipe fused shader dispatch investigation (12 tests return 0.0) | neuralSpring V87 | Open (barraCuda-owned) |
| `CoralReefDevice` backend in barraCuda (sovereign SASS dispatch) | groundSpring V95 | Open (barraCuda-owned) |
| QMD constant buffer binding (coralReef P0 blocker) | groundSpring V95 | Open (coralReef-owned) |
| `SumReduceF64` / `VarianceReduceF64` Fp64Strategy branching fix | groundSpring V95 | Open (barraCuda-owned) |
| wetSpring `special::{erf, ln_gamma, dot, l2_norm}` absorption | wetSpring V97d | Open (barraCuda-owned) |

## New Handoff Items (Mar 7, 2026)

### From hotSpring v0.6.19

| Item | Impact |
|------|--------|
| DF64 compilation fully delegated to barraCuda | No toadStool action — hotSpring now calls `compile_shader_universal(Precision::Df64)` |
| 3 Chuna papers (43-45) CPU-complete | GPU promotion is barraCuda-owned |
| Cross-spring GPU benchmarks (Autocorrelation, Mean+Variance, Correlation, Chi-squared) | Benchmarked via barraCuda APIs; validates cross-spring evolution |
| `GpuView<T>` adoption target | Eliminate per-call buffer upload/download for MD; barraCuda-owned |
| Edition 2024 migration candidate | Align with barraCuda edition |

### From groundSpring V95

| Item | Impact |
|------|--------|
| coralReef Phase 11: push buffer encoding fixed | `[PBENTRY]` bug was count/method field swap in Kepler+ Type 1 headers. All 5 GPU method tests pass on Titan V. coralReef-owned. |
| NVIF constants aligned to Mesa `nvif/ioctl.h` | `ROUTE_NVIF=0x00`, `ROUTE_HIDDEN=0xFF`, `OWNER_NVIF=0x00`, `OWNER_ANY=0xFF` |
| P0 barraCuda fixes: `SumReduceF64`/`VarianceReduceF64` missing Fp64Strategy | Consumer GPUs produce wrong values; barraCuda-owned |
| `multinomial_sample_cpu` outside `cfg(gpu)` | CPU fallback gated behind GPU feature; barraCuda-owned |
| Sovereign pipeline E2E: QMD CBUF binding is next blocker | coralReef-owned |

### From neuralSpring V87/S129

| Item | Impact |
|------|--------|
| Struct-based API migration complete (HmmForwardArgs, GillespieModel, etc.) | neuralSpring consuming barraCuda APIs; no toadStool action |
| 12 GPU test failures on llvmpipe (fused shaders return 0.0) | wgpu 28 + llvmpipe interaction; barraCuda investigation needed |
| `#![forbid(unsafe_code)]` enforced | neuralSpring internal quality gate |
| 883 lib + 240 bin tests; 218/218 validate_all PASS | Confirms evolution chain health |

### From wetSpring V97d/V97d+

| Item | Impact |
|------|--------|
| Zero API breakage against toadStool S130 + barraCuda `2a6c072` | 1,347 tests pass; ecosystem sync confirmed |
| New barraCuda primitives available: `BatchedOdeRK45F64`, provenance module, `mean_variance_to_buffer()` | wetSpring adoption targets; no toadStool action |
| I/O API deprecation: `parse_fastq`/`parse_mzml`/`parse_ms2` → streaming iterators | wetSpring internal evolution |
| 104 bare `.unwrap()` → contextual `.expect()` across 17 validators | Crash diagnostics improvement |

### From airSpring V071

| Item | Impact |
|------|--------|
| NVK zero-output detection + CPU fallback in `gpu::bootstrap` | Recommends generic `NvkZeroGuard` in barraCuda; toadStool's `PrecisionRoutingAdvice` already routes around this |
| Kokkos validation gap (100x-2600x dispatch overhead for stats ops) | Phase 1: persistent buffers + fused reductions; barraCuda-owned |
| Provenance convention: benchmark JSON `_provenance.baseline_commit` as authoritative | Good practice note for all springs |
| `cargo-deny` license enforcement added | airSpring internal quality gate |

### From Sovereign Pipeline (coralReef Phase 10-11)

| Item | Impact |
|------|--------|
| `shader.compile.*` IPC contract aligned across 3 primals | toadStool, barraCuda, coralReef all on `shader.compile.*` namespace |
| coralReef Phase 11: push buffer + NVIF encoding fixed | 5/5 GPU method tests pass on Titan V; remaining blocker is QMD CBUF binding |
| coralReef Iteration 5-7: debt reduction, deep internalization, safety boundaries | 856 tests, zero warnings; AGPL-3.0 sovereign compiler |

## New Handoff Items (Mar 5-6, 2026)

### From hotSpring v0.6.17

| Item | Impact |
|------|--------|
| `gradient_flow.rs` — Wilson SU(3) gradient flow | CPU-only Tier 2 QCD; absorption candidate for barraCuda |
| `brain.rs` — MD Nautilus Brain (12-head reservoir) | Uses upstream `nautilus`; validates NautilusBrain API |
| 6 Verlet WGSL shaders | barraCuda absorption (build, check_displacement, copy_ref, force f64/df64) |
| Kokkos parity: 27x→3.7x gap | Dispatch overhead dominates; persistent kernels + fused VV needed |

### From groundSpring V80

| Item | Impact |
|------|--------|
| Fused `correlation_full` GPU (5-accumulator) | barraCuda stats absorption candidate |
| Welford single-pass CPU stats | `pearson_full_cpu` candidate for `barracuda::stats` |
| metalForge: 30 workloads, 187 checks | Hardware validation corpus |
| NUCLEUS atomics pattern | Reusable pass/fail harness |

### From neuralSpring V86/S128

| Item | Impact |
|------|--------|
| `VarianceReduceF64` → `VarianceF64` rewire | API name cleanup |
| 46 upstream rewires complete | All 17 shortcomings resolved |
| coralForge structure prediction | Independent shader catalog |
| baseCamp biophysical AI (6 modules) | weight_spectral, info_flow, loss_landscape, neural_pgm, agent_coordination, immunological_anderson |

### From wetSpring V97d

| Item | Impact |
|------|--------|
| Fused ops chain (Exp306–310) | DF64 dispatch routing |
| 0 local WGSL (fully lean) | Reference for Write→Absorb→Lean pattern |
| Bio Brain cross-spring integration | `BioNautilusBrain` adapter |
| IPC science primal (JSON-RPC 2.0) | `science.diversity`, `science.qs_model`, `science.anderson` |

### From airSpring V071

| Item | Impact |
|------|--------|
| wgpu 28 + subgroup detection | DevicePrecisionReport already reports subgroup sizes |
| 3 local ops ready for absorption | Makkink→Op14, Turc→Op15, Hamon→Op16 (DONE upstream) |
| 3 remaining local WGSL ops | SCS-CN, Stewart, Blaney-Criddle |
| Nautilus/AirSpringBrain | ET₀ forecasting + drift detection |

### From Sovereign Pipeline (coralReef/wateringHole)

| Item | Impact |
|------|--------|
| coralReef Phases 1–5 complete | Sovereign Rust NVIDIA shader compiler (WGSL→SASS) |
| coralDriver assigned to groundSpring | Level 4 sovereign compute |
| Titan V pipeline: 4 levels | WGSL polyfills (done) → fork NAK → coralNak → coralDriver |
| Ada Lovelace SM89 f64 crash | Proprietary driver cannot compile native f64 transcendentals |

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

### Incoming (to ToadStool)

| Handoff | From | Key Items |
|---------|------|-----------|
| `NEURALSPRING_TOADSTOOL_V87_S129_API_SYNC_MAR07` | neuralSpring V87 | Struct-based API sync, 12 llvmpipe failures, `#![forbid(unsafe_code)]` |
| `AIRSPRING_V071_DEEP_DEBT_NVK_TOADSTOOL_MAR07` | airSpring V071 | NVK zero-output detection, Kokkos gap, provenance convention |
| `GROUNDSPRING_V95_TOADSTOOL_BARRACUDA_CORALREEF_MAR07` | groundSpring V95 | coralReef Phase 11 push buffer fix, 102 delegations, Fp64Strategy gaps |
| `WETSPRING_V97D_DEEP_AUDIT_EVOLUTION_MAR07` | wetSpring V97d | Zero unsafe, streaming I/O deprecation, forge dispatch |
| `WETSPRING_V97D_ECOSYSTEM_SYNC_MAR07` | wetSpring V97d+ | Zero API breakage vs S130, new primitives available |
| `HOTSPRING_V0619_BARRACUDA_REWIRE_MAR06` | hotSpring v0.6.19 | DF64 delegation complete, `GpuView<T>` target |
| `HOTSPRING_V0619_CROSS_SPRING_EVOLUTION_MAR06` | hotSpring v0.6.19 | 3 Chuna papers, cross-spring GPU benchmarks |
| `CROSS_PRIMAL_IPC_REWIRE_SHADER_COMPILE_MAR07` | coralReef/ecosystem | `shader.compile.*` contract aligned across 3 primals |
| `SOVEREIGN_TITAN_V_PIPELINE_GAPS_MAR06` | hotSpring/coralReef | is_sovereign_capable, NVK allocation guard |
| `SOVEREIGN_PIPELINE_CROSS_PRIMAL_MAR05` | wateringHole | coralDriver routing, vendor-agnostic IR |

### Outgoing (from ToadStool)

| Handoff | To | Session |
|---------|----|---------|
| `TOADSTOOL_S130_DEEP_DEBT_CLIPPY_PEDANTIC_MAR07` | Ecosystem | S130+ |
| `TOADSTOOL_S128_DEEP_DEBT_EVOLUTION_MAR06` | Ecosystem | S128 |
| `TOADSTOOL_S97_SPRING_ABSORPTION_MAR06` | Ecosystem | S97 |

## metalForge Clarification

**metalForge = silicon characterization**, not Apple Metal API. All GPU work uses **WGSL via wgpu** (Vulkan/Metal/DX12 backends). Apple Metal is transparently handled by the wgpu abstraction layer. metalForge probes the actual hardware substrate: GPU (wgpu), CPU (/proc), NPU (/dev).
