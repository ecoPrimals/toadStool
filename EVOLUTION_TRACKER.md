# Evolution Tracker

**Date**: March 2, 2026 — Session 79
**Philosophy**: Deep debt solutions pay off. Modern idiomatic Rust. Capability-based discovery. Self-knowledge only.

---

## Principles

1. **Math is universal, precision is silicon** — all math originates as WGSL, barracuda owns all precisions
2. **Deep debt over shortcuts** — complete implementations, no mocks in production
3. **Modern idiomatic Rust** — evolve external deps to pure Rust, native AFIT over async-trait where possible
4. **Smart refactoring** — large files decomposed by domain, not just split
5. **Unsafe → fast AND safe** — narrow scope, safe wrappers, documented invariants
6. **Capability-based discovery** — primals discover each other at runtime by capability, not name
7. **Self-knowledge** — primal code only knows its own identity; everything else is runtime discovery
8. **Mocks isolated to testing** — `#[cfg(test)]` gated; production code has complete implementations
9. **Concurrency-first** — no sleeps in non-chaos tests; test issues are production issues
10. **Device resilience** — all GPU paths protected by catch_unwind; errors propagate as Result

---

## Spring Absorption — Completed

All P0 dispatch wiring complete. Core absorption from 5 springs validated:

| Spring | Version | Status | Key Deliverables |
|--------|---------|--------|-----------------|
| neuralSpring | V64 | ✅ Core absorbed | 39/39 CPU↔GPU parity, AlphaFold2 17 shaders, HillGateGpu, SwarmNnGpu, DF64 ML primitives |
| wetSpring | V82 | ✅ Core absorbed | 85 primitives, ValidationHarness, 42/42 Exp247 |
| airSpring | V039/V052 | ✅ Core absorbed | Ops 0-8, seasonal pipeline, 57 experiments, 26/26 GPU rewire |
| groundSpring | V54 | ✅ Core absorbed | 95/95 three-tier parity, wright_fisher, grid ops, DF64 default fallback |
| hotSpring | V0615 | ✅ Core absorbed | NVK serialization, brain arch reviewed, device-creation documented |
| wateringHole | V69 | ✅ Core absorbed | Chi-squared batch, MC ET0 propagate |

---

## Spring Absorption — Pending

### P1: Partially Absorbed (signature/API gaps)

| Item | Source | What Exists | What Remains |
|------|--------|------------|-------------|
| `barracuda::nn` completeness | neuralSpring V24 | ✅ SimpleMLP + LstmReservoir + EsnClassifier (S76) | — |
| ESN full API | wetSpring V61 | ✅ EsnConfig/train/predict/reset/serde (S76) | — |
| `BatchedMultinomialGpu` alignment | groundSpring V37 | Struct + shader | `cumulative_probs` + closure RNG signature |
| `NeighborMode::PrecomputedBuffer` | hotSpring S68 | — | Precomputed neighbor table for lattice ops |

### P2: New Shaders & Ops

| Item | Source | Priority | Status |
|------|--------|----------|--------|
| 15 sovereign folding DF64 shaders | neuralSpring V60 | HIGH | ✅ S76: All 15 + FoldingOp + compile_folding_shader() |
| `fused_chi_squared_f64` | neuralSpring V24 | MEDIUM | ✅ S76: FusedChiSquaredGpu + shader |
| `fused_kl_divergence_f64` | neuralSpring V24 | MEDIUM | ✅ S76: FusedKlDivergenceGpu + shader |
| `BatchReconcileGpu` | wetSpring V61 | MEDIUM | ☐ reconciliation_gpu passthrough |
| RAWR weighted resampling kernel | groundSpring V10/V54 | MEDIUM | ✅ S76: RawrWeightedMeanGpu + shader |
| Batch Nelder-Mead | airSpring V039 | MEDIUM | ☐ Multi-start parallel shader for isotherm fitting |
| Pedotransfer polynomial | airSpring V039 | MEDIUM | ✅ S76: Op 13 in batched_elementwise_f64 |
| VG θ/K, Thornthwaite, GDD | airSpring V039 | MEDIUM | ✅ S76: Ops 9-12 in batched_elementwise_f64 |
| Boltzmann sampling dispatch | wateringHole V69 | MEDIUM | ✅ S76: BoltzmannSamplingGpu + shader |
| `GpuDriverProfile` sin/cos workarounds | hotSpring F64 | MEDIUM | ☐ `needs_sin_f64_workaround()` / `needs_cos_f64_workaround()` |

### P3: Infrastructure & Architecture

| Item | Source | Priority | Status |
|------|--------|----------|--------|
| NautilusBrain API (`ai.nautilus.*`) | hotSpring V0615 | HIGH | ☐ JSON-RPC methods for brain architecture |
| bingoCube-nautilus workspace dep | hotSpring V0615 | HIGH | ☐ Workspace integration |
| IPC evolution (multi-transport) | wateringHole | MEDIUM | ☐ Abstract sockets + TCP fallback |
| Batched encoder (fused pipeline) | neuralSpring V64 | MEDIUM | ☐ Per-op submit → batched encoder (46-78× speedup) |
| NPU bandwidth model | neuralSpring V60 | LOW | ☐ Transfer cost tiers for metalForge |
| `PipelineBuilder` CPU-only mode | wetSpring V82 | LOW | ☐ Topology analysis without GPU context |
| metalForge Stage/Pipeline topology | hotSpring/wateringHole | LOW | ☐ Stage<In,Out>, Chain/FanIn/FanOut/Graph |

### P4: Lower Priority (Carried)

| Item | Source | Status |
|------|--------|--------|
| SparseGemmF64 (CSR × dense for NMF) | wetSpring V82 | ☐ |
| ESN 36-head MultiHeadEsn + ExportedWeights alignment | hotSpring V0615 | ✅ S79 |
| StatefulPipeline (water balance state) | airSpring V039 | ☐ |
| NPU substrate kind in metalForge | neuralSpring V60 | ☐ |
| Streaming FASTQ/mzML/MS2 (bio I/O) | wateringHole V69 | ☐ |
| Pseudofermion HMC (477 lines) | wateringHole V69 | ☐ |
| Omelyan integrator | wateringHole V69 | ☐ |
| Richards PDE (12 USDA textures) | wateringHole V69 | ☐ |
| `TensorSession::fused_mlp` | wateringHole V69 | ☐ |

---

## Deep Debt — Active

### Architecture

| ID | Description | Priority | Status |
|----|-------------|----------|--------|
| D-CD | ComputeDispatch migration (~174 legacy ops) | High | 76 done, incremental |
| D-DF64 | DF64 as default precision path | Medium | Architectural decision pending |
| D-NPU | NpuDispatch trait (generic NPU interface) | Medium | Design phase |
| D-COV | Test coverage → 90% | Medium | Major gains; gap in GPU ops, neuromorphic |
| D-WC | Wildcard re-exports remaining | Low | 13 crates narrowed; remaining have 15+ items (justified) |

### God Files Remaining (>600 lines)

| File | Lines | Domain | Priority | Status |
|------|-------|--------|----------|--------|
| `barracuda/src/device/wgpu_device/mod.rs` | ~520 | GPU device (post-compilation.rs extract) | ✅ S76 | Refactored |
| `barracuda/src/device/driver_profile/` | ~370 | Driver detection + workarounds + arches | ✅ S76 | → directory |
| `barracuda/src/device/probe/` | ~120 | GPU probing → 5 modules | ✅ S76 | → directory |
| `api/src/jsonrpc/` | ~230 | JSON-RPC → types + handlers + dispatch | ✅ S76 | → directory |
| `barracuda/src/ops/batched_elementwise_f64/` | ~967→4 files | Batched GPU ops (13 ops) | ✅ S77 | → directory (op, cpu_ref, executor, mod) |
| `barracuda/src/device/capabilities/` | ~912→3 files | Device caps + info | ✅ S77 | → directory (wgpu, device_info, mod) |
| `barracuda/tests/fhe_shader_unit_tests.rs` | 1028→8 files | FHE shader tests | ✅ S77 | → tests/fhe/ (ntt, intt, pointwise, etc.) |
| `core/toadstool/src/workload/mod.rs` | 699 | Workload spec | Low | Recently refactored |
| `management/monitoring/src/lib.rs` | 679 | Monitoring | Low | Recently refactored |

### Dependency Evolution

| Dependency | Status | Path |
|------------|--------|------|
| `async-trait` | 5 crates migrated to AFIT | ~71 uses remain (all need dyn Trait; TODO(afit): trait_variant) |
| `pollster` | ✅ Eliminated workspace-wide | — |
| `serde_yaml` | ✅ Migrated to serde_yaml_ng | — |
| `chrono` | ✅ Eliminated (std::time) | — |
| `anyhow` | ✅ Eliminated (thiserror) | — |
| `log` | ✅ Eliminated (tracing) | — |
| `libc` in akida-driver | ✅ Eliminated (rustix) | — |

### Hardcoding → Capability-Based

| Area | Status |
|------|--------|
| CLI templates/error messages | ✅ Capability-based language |
| JSON-RPC health/metrics | ✅ `ecosystem_connected` |
| Type aliases for new code | ✅ OrchestrationConfigurator, PkiSecurityConfig, etc. |
| DNS discovery | ✅ Documented as compatibility defaults |
| `well_known` module | ✅ Deprecated with `#[allow(deprecated)]` on IPC callers |
| Edge platform stubs | ✅ Genuine hardware probing |
| Discovery functions | ✅ Real mDNS/k8s/docker/registry probing |
| Deprecated name-based discovery | ✅ S77: `discover_beardog_at`/`discover_nestgate_at` removed |
| K8s/Docker port hardcoding | ✅ S77: Configurable via `TOADSTOOL_DISCOVERY_HTTP_PORT` |
| Production stubs/mocks | ✅ S77: TCP provider, EMA prediction, proper error returns |
| `legacy_primal_to_capabilities` / `legacy_primal_primary_capability` | ✅ S78: Removed (no callers); primal_capabilities now clean capability-to-primal mapping |

### Unsafe Code

| Status | Count | Notes |
|--------|-------|-------|
| Total `unsafe` blocks | 45 | All `// SAFETY:` documented (S77: comprehensive audit) |
| Reducible | 0 | S77: All verified necessary (64-byte aligned alloc, wgpu FFI, CUDA FFI) |
| `#![deny(unsafe_code)]` | 36 crates | 2 justified exceptions: gpu, secure_enclave |
| SAFETY comments | ✅ | S77: Invariants, violation effects, and justification documented |

---

## Quality Gates

| Gate | Status |
|------|--------|
| `cargo check --workspace` | ✅ 0 errors |
| `cargo clippy --workspace -- -D warnings` | ✅ S77: deprecated discovery calls removed |
| `cargo fmt --all -- --check` | ✅ S77: 340 diffs fixed |
| `cargo test -p barracuda --lib` | ✅ 2,781 passed, 13 ignored |
| Workspace lib tests | ✅ 5,500+ passed |
| `#[serial]` tests | ✅ 0 remaining |
| Production sleeps (non-chaos) | ✅ 0 (documented exceptions: hardware polling, retry backoff) |
| Production mocks/stubs | ✅ 0 |
| WGSL shaders | 844 (0 orphans, 0 f32-only, 37 DF64, 15 folding, 2 bitcast-fixed) |
| God files refactored | 35+ (all <1000 lines, S77: +3) |
| `cargo doc` | ✅ S77/S78: private intra-doc links fixed (ToadStoolError in universal_adapter, discovery_integration) |
| e2e runtime nesting | ✅ S77: `run_gpu_resilient_async` evolved to dedicated runtime |
| Zero-copy anti-patterns | ✅ S77: All `cast_slice().to_vec()` verified necessary, documented |
| Test coverage (llvm-cov) | 41.86% lines | Target: 90% — major gap remaining |
| Compile bottleneck analysis | S78 | tfhe+tfhe-fft = 30.6% CPU (showcase); wgpu 22/23 duplication wastes ~90s |

---

*This tracker is the single source of truth for evolution status. Updated each session.*
