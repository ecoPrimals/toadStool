# Spring Absorption Tracker

**Session**: S129 Deep Debt Execution (March 7, 2026)
**ToadStool**: master, ~83% line coverage (170K lines). 19,109 tests, 0 failures. C deps evolved to pure Rust. Capability-based port resolution. 45+ god files refactored. Zero-copy hot paths. All quality gates passing.

## Spring Pin Status

| Spring | Version | Previous Pin | Current Pin | Tests | Delegations |
|--------|---------|--------------|-------------|-------|-------------|
| hotSpring | v0.6.17 | S95→S97 | S97→S128 | 669 | Lattice QCD, MD reservoir, NPU, gradient flow, Verlet |
| groundSpring | V85 | V80→S97 | V85→S128 | 812+390 | 87 (51 CPU + 36 GPU), metalForge 30 workloads, f64 shared-mem bug |
| neuralSpring | V86/S128 | V86/S128→S97 | V86/S128→S128 | 4,100+ | 42 WGSL, gpu_dispatch, coralForge |
| wetSpring | V97d | V97d→S97 | V97d→S128 | 1,047+200 | 150+ primitives, 0 local WGSL (fully lean) |
| airSpring | V071 | V071→S97 | V071→S128 | 827+1,498 | 25 Tier A GPU + 6 universal, 3 local WGSL ops |

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

### P3 — Shader Evolution (tracked)

| Item | Source | Status |
|------|--------|--------|
| Flash attention WGSL | neuralSpring | Tracked |
| Fused LayerNorm+GELU WGSL | neuralSpring | Tracked |
| Fused LSTM cell WGSL (streaming) | neuralSpring V86 | Tracked (NEW) |
| Deformed HFB full wiring (5 WGSL) | hotSpring | Tracked |
| Abelian Higgs U(1)+Higgs (3 WGSL) | hotSpring | Tracked |
| Richards PDE GPU full wiring | airSpring, groundSpring | Tracked |
| Multi-GPU interconnect | neuralSpring | Tracked |
| Autocorrelation GPU op | neuralSpring V86 | Tracked (NEW) |
| R² score GPU op | neuralSpring V86 | Tracked (NEW) |
| SCS-CN, Stewart, Blaney-Criddle (3 ops) | airSpring local WGSL | Tracked (NEW) |

### P4 — Future (lower priority)

| Item | Source | Status |
|------|--------|--------|
| NPU substrate kind in metalForge | neuralSpring | Open |
| Streaming FASTQ/mzML/MS2 (bio I/O) | wateringHole | Open |
| Pseudofermion HMC (477 lines) | wateringHole | Open |
| Omelyan integrator | wateringHole | Open |
| Richards PDE (12 USDA textures) | wateringHole | Open |
| Provenance tags | hotSpring, groundSpring, neuralSpring | Open |

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
| `SOVEREIGN_TITAN_V_PIPELINE_GAPS_MAR06` | hotSpring/coralReef | is_sovereign_capable, NVK allocation guard |
| `SOVEREIGN_PIPELINE_CROSS_PRIMAL_MAR05` | wateringHole | coralDriver routing, vendor-agnostic IR |
| `HOTSPRING_VERLET_EVOLUTION_MAR05` | hotSpring | 6 Verlet WGSL shaders (→barraCuda) |
| `HOTSPRING_V0617_ASYMMETRIC_LATTICE_MAR05` | hotSpring | HardwareFingerprint, TFLOPS estimation |
| `GROUNDSPRING_V80_FUSED_OPS_MAR05` | groundSpring | Welford GPU, metalForge 30 workloads |
| `NEURALSPRING_V86_S128_MODERN_REWIRE_MAR05` | neuralSpring | Fused LSTM, autocorrelation, R² GPU |
| `WETSPRING_V97C_FUSED_OPS_CHAIN_MAR05` | wetSpring | DF64 dispatch routing, fused ops |
| `AIRSPRING_V071_BARRACUDA_HEAD_SYNC_MAR05` | airSpring | wgpu 28, subgroup, 3 local WGSL ops |

### Outgoing (from ToadStool)

| Handoff | To | Session |
|---------|----|---------|
| `TOADSTOOL_S95_SPRING_SYNC` | Ecosystem | S95 |

## metalForge Clarification

**metalForge = silicon characterization**, not Apple Metal API. All GPU work uses **WGSL via wgpu** (Vulkan/Metal/DX12 backends). Apple Metal is transparently handled by the wgpu abstraction layer. metalForge probes the actual hardware substrate: GPU (wgpu), CPU (/proc), NPU (/dev).
