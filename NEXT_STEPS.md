# ToadStool/BarraCuda -- Next Steps

**Updated**: March 1, 2026 -- Session 71
**Status**: Production-grade | AGPL-3 compliant | 0 clippy warnings | Standalone-resilient | Zero chrono | Zero anyhow | Zero production stubs | 45 justified unsafe | 671 WGSL shaders (29 DF64) | 2,773+ barracuda tests | 4,700+ workspace lib tests | Rust 1.80+
**Latest**: 4 orphaned shader constants wired to GPU dispatch. 3 CPU-only primitives evolved to GPU (kimura, jackknife, hargreaves). Hardcoded primal names evolved to constants. jsonrpc_server + types.rs smart-refactored.

---

## Active Work

### P0: ComputeDispatch Migration (Incremental)

41 of ~250 ops migrated to the fluent `ComputeDispatch` builder. Each migration replaces
~80 lines of manual BGL/BG/pipeline boilerplate with ~5 lines. ~209 ops remaining.

Migrated so far:
- 5 linalg (cholesky f32/f64, eigh, inverse_f64, linsolve f32/f64)
- 15 special functions (hermite, bessel, digamma, legendre, laguerre, spherical_harmonics, beta — all f64)
- 14 MD/bio/reduce (morse, born_mayer, lennard_jones, yukawa, velocity_verlet, kinetic_energy, rdf, pairwise_l2, hmm_forward, sum/norm/variance/prod_reduce, max_abs_diff — all f64)
- 7 reduction ops (sum, prod, mean, norm, max, argmin, argmax — S71)

### P1: DF64 Default Path (Architecture)

Make `df64_rewrite` the default precision strategy, not a fallback. Currently DF64 activates
only when `Fp64Strategy::Hybrid` is selected. For consumer GPUs (1:64 FP64:FP32),
DF64 should be the primary path with native f64 reserved for reductions/convergence.

### P1: DF64 Transcendental Coverage

Extend `df64_transcendentals.wgsl`:
- [x] `asin_df64`, `acos_df64`, `atan_df64`, `atan2_df64` (S71)
- [x] `sinh_df64`, `cosh_df64` (S71)
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
- [ ] **ComputeDispatch migration** -- 34/250 ops migrated; ~216 remaining (incremental)
- [ ] **DF64 default path** -- df64_rewrite as default, not fallback (groundSpring V35)
- [ ] **NpuDispatch trait** -- generic NPU interface
- [ ] **Test coverage target 90%** -- significant gains across CLI, server, API, monitoring, distributed

### Cross-Repo Debt

- [ ] **D-S20-003**: neuralSpring `evolved/` migration (~2075 lines) — awaiting neuralSpring team
- [ ] **D-S18-002**: cubecl transitive `dirs-sys` — needs upstream PR

---

## Completed This Session (S70 through S70+++)

### Session 70+++: Builder Refactor + Dead Code + Monitoring Evolution
- `builder.rs` (975 lines) smart-refactored into `builder/` module (3 files, all <600 lines)
- Deleted deprecated `EcosystemCaller` (95 lines dead code, zero references)
- 5 monitoring collector stubs → real `sysinfo` implementations (health, resources, alerts, perf)
- NestGate `connect()` → real socket path resolution
- All root docs cleaned (7 files, all stale counts fixed)

### Session 70+/++: Cross-Spring Absorption + Sovereignty + Architecture
- 7 new WGSL shaders, 6 new GPU ops, 3 new stats modules, SimpleMLP
- Sovereignty: port 8084→dynamic, songbird→mdns, capability-based adapter
- `Fp64Strategy::Concurrent`, monitoring split (1071→679), `UniversalAdapter` evolved
- +37 new tests

### Session 70: Deep Debt + Test Concurrency Evolution
- 15+ production stubs → real implementations, +150 new tests
- All env tests → `temp_env`, all non-chaos sleeps removed, timeouts reduced
- ChaosEngine fix, error codes, doctests, real mDNS parser
- Full workspace: 6m30s, 0 failures, 0 warnings

### Session 69++: Architecture & Code Evolution

**ComputeDispatch migration (34 ops)**: 5 linalg + 15 special functions + 14 MD/bio/reduce.
~3,739 lines of manual BGL/BG/pipeline boilerplate replaced with fluent builder pattern.

**metalForge streaming pipeline**: `PipelineBuilder` → `StreamingPipeline` with chained
GPU dispatches, zero CPU readback.

**manual_jsonrpc → pure_jsonrpc**: Full handler parity. Unix/TCP connection layer. Unibin migrated.

### Session 69/69+: Cross-Spring Absorption + Deep Debt

All 5 spring handoffs absorbed (196 handoff files). 30+ new WGSL shaders created + dispatch wired.
anyhow fully eliminated. 6 large files refactored.

### Session 68+++: Deep Debt Sweep

chrono eliminated from 28 crates. Unsafe 47→45. ~400 lines dead code removed.

---

See [CHANGELOG.md](CHANGELOG.md) for full completed session history.
