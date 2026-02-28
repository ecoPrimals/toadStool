# ToadStool/BarraCuda -- Next Steps

**Updated**: February 28, 2026 -- Session 69++
**Status**: Production-grade | AGPL-3 compliant | 1 clippy warning (deprecated grpc fallback) | Standalone-resilient | Zero chrono | Zero anyhow | 45 justified unsafe | 661 WGSL shaders | 2,726+ barracuda tests | Barracuda ~82% coverage | Rust 1.80+
**Latest**: 34 ops → ComputeDispatch (~3,739 lines removed). NAK workgroup tuning. metalForge streaming. manual_jsonrpc fully migrated. 16 large files refactored. +100 new tests. All spring handoffs absorbed.

---

## Active Work

### P0: ComputeDispatch Migration (Incremental)

34 of ~250 ops migrated to the fluent `ComputeDispatch` builder. Each migration replaces
~80 lines of manual BGL/BG/pipeline boilerplate with ~5 lines. ~216 ops remaining.

Migrated so far:
- 5 linalg (cholesky f32/f64, eigh, inverse_f64, linsolve f32/f64)
- 15 special functions (hermite, bessel, digamma, legendre, laguerre, spherical_harmonics, beta — all f64)
- 14 MD/bio/reduce (morse, born_mayer, lennard_jones, yukawa, velocity_verlet, kinetic_energy, rdf, pairwise_l2, hmm_forward, sum/norm/variance/prod_reduce, max_abs_diff — all f64)

### P1: DF64 Default Path (Architecture)

Make `df64_rewrite` the default precision strategy, not a fallback. Currently DF64 activates
only when `Fp64Strategy::Hybrid` is selected. For consumer GPUs (1:64 FP64:FP32),
DF64 should be the primary path with native f64 reserved for reductions/convergence.

### P1: DF64 Transcendental Coverage

Extend `df64_transcendentals.wgsl`:
- [ ] `asin_df64`, `acos_df64`, `atan_df64`, `atan2_df64`
- [ ] `sinh_df64`, `cosh_df64`
- [ ] `gamma_df64`, `erf_df64` (Lanczos/Abramowitz at DF64 precision)

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
- [x] **Production stubs** -- 4 stubs evolved to real implementations
- [x] **Dead code documented** -- all 18 unjustified `#[allow(dead_code)]` annotated
- [x] **Unidirectional streaming** -- ring_buffer + unidirectional + stateful + pipeline
- [x] **MD observables** -- stress_virial_f64, vacf_batch_f64 created + dispatch wired
- [x] **AlphaFold2 advanced (17)** -- all created + dispatch wired
- [x] **airSpring batch ops** -- hargreaves_et0, dual_kc, van_genuchten, batched_crop_pipeline
- [ ] **ComputeDispatch migration** -- 34/250 ops migrated; ~216 remaining (incremental)
- [ ] **DF64 default path** -- df64_rewrite as default, not fallback (groundSpring V35)
- [ ] **NpuDispatch trait** -- generic NPU interface
- [ ] **Test coverage target 90%** -- barracuda at ~82% (2,726 tests); +100 new tests added S69++

### Cross-Repo Debt

- [ ] **D-S20-003**: neuralSpring `evolved/` migration (~2075 lines) — awaiting neuralSpring team
- [ ] **D-S18-002**: cubecl transitive `dirs-sys` — needs upstream PR

---

## Completed This Session

### Session 69++: Architecture & Code Evolution

**ComputeDispatch migration (34 ops)**: 5 linalg + 15 special functions + 14 MD/bio/reduce.
~3,739 lines of manual BGL/BG/pipeline boilerplate replaced with fluent builder pattern.

**NAK workgroup tuning**: `workgroup_size_for_arch()` (Volta 64, Ada 256, RDNA 64),
`workgroup_size_2d_for_arch()`, `optimal_workgroup_size_arch()` — 6 tests.

**metalForge streaming pipeline**: `PipelineBuilder` → `StreamingPipeline` with chained
GPU dispatches, zero CPU readback. `execute()`, `execute_iterations(n)`, `execute_and_read<T>()`.

**manual_jsonrpc → pure_jsonrpc**: Full handler parity (resources, gpu, ollama, gate/cluster).
Unix/TCP connection layer. Unibin migrated. manual_jsonrpc deprecated.

**Production stubs → implementations**: biome.rs (real validation), container benchmarking
(Docker/Podman runtime detection), gRPC fallback (deprecated), OpenCL (capability-based).

**Smart refactoring (10 large files)**: alphafold2.rs (882→5 modules), workload.rs (821→6),
cli/main.rs (805→setup+dispatch), server/lib.rs (710→3), installer.rs (852→8),
mdns.rs (730→3), performance/lib.rs (703→4). All production files < 1000 lines.

**Hardcoded IPs → constants**: 6 production files. rust-version 1.75→1.80. Dead code documented.
Unsafe evolution (GPU memory bounds checks, SAFETY docs). +100 new tests across workspace.

### Session 69/69+: Cross-Spring Absorption + Deep Debt

All 5 spring handoffs absorbed (196 handoff files). 30+ new WGSL shaders created + dispatch wired.
13 S69 shaders → Rust dispatch. anyhow fully eliminated. 6 large files refactored.

### Session 68+++: Deep Debt Sweep

chrono eliminated from 28 crates. Unsafe 47→45. ~400 lines dead code removed.

---

See [CHANGELOG.md](CHANGELOG.md) for full completed session history.
