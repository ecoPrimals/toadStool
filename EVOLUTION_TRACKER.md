# Evolution Tracker

**Date**: March 2, 2026 — Session 86
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
| groundSpring | V61 | ✅ S81 absorbed | InterconnectTopology, SubstratePipeline, BandwidthTier (PCIe P2P routing) |
| neuralSpring | V70 | ✅ S81 absorbed | IFFT/NTT/INTT buffer fixes, `enable f64;` stripping |
| Cross-spring S83 | All springs | ✅ S83 absorbed | BrentGpu, anderson_4d+Wegner, Omelyan, RichardsGpu, L-BFGS, BatchedStatefulF64, HeadKind generalization, SpectralBridge, ESN shape hardening |
| Deep debt S84 | toadStool | ✅ S84 evolved | 9 ops → ComputeDispatch, hydrology god-file refactored, experimental.rs stub → real probes, frameworks.rs echo → proper error, mDNS constants extracted |
| Deep debt S86 | toadStool | ✅ S86 evolved | 12 ops → ComputeDispatch (determinant, mse_loss, dice, quantize, dequantize, bce_loss, permute, movedim, logsumexp, index_add, tensor_split, concat); wgpu_backend.rs magic numbers → real device limits; deployment.rs stubs → capability-discovery docs |

---

## Spring Absorption — Pending

### P1: Partially Absorbed (signature/API gaps)

| Item | Source | What Exists | What Remains |
|------|--------|------------|-------------|
| `barracuda::nn` completeness | neuralSpring V24 | ✅ SimpleMLP + LstmReservoir + EsnClassifier (S76) | — |
| ESN full API | wetSpring V61 | ✅ EsnConfig/train/predict/reset/serde (S76) | — |
| `BatchedMultinomialGpu` alignment | groundSpring V37 | ✅ S80: `BatchedMultinomialConfig` + cumulative_probs + seed | — |
| `NeighborMode::PrecomputedBuffer` | hotSpring S68 | ✅ S80: 2D/3D/4D periodic lattice precompute (6 tests) | — |

### P2: New Shaders & Ops

| Item | Source | Priority | Status |
|------|--------|----------|--------|
| 15 sovereign folding DF64 shaders | neuralSpring V60 | HIGH | ✅ S76: All 15 + FoldingOp + compile_folding_shader() |
| `fused_chi_squared_f64` | neuralSpring V24 | MEDIUM | ✅ S76: FusedChiSquaredGpu + shader |
| `fused_kl_divergence_f64` | neuralSpring V24 | MEDIUM | ✅ S76: FusedKlDivergenceGpu + shader |
| `BatchReconcileGpu` | wetSpring V61 | MEDIUM | ☐ Deferred — full DTL reconciliation, no existing primitives |
| RAWR weighted resampling kernel | groundSpring V10/V54 | MEDIUM | ✅ S76: RawrWeightedMeanGpu + shader |
| Batch Nelder-Mead | airSpring V039 | MEDIUM | ✅ S80: `batched_nelder_mead_gpu` + batched simplex shaders |
| Pedotransfer polynomial | airSpring V039 | MEDIUM | ✅ S76: Op 13 in batched_elementwise_f64 |
| VG θ/K, Thornthwaite, GDD | airSpring V039 | MEDIUM | ✅ S76: Ops 9-12 in batched_elementwise_f64 |
| Boltzmann sampling dispatch | wateringHole V69 | MEDIUM | ✅ S76: BoltzmannSamplingGpu + shader |
| `GpuDriverProfile` sin/cos workarounds | hotSpring F64 | MEDIUM | ✅ S80: Taylor preamble + asin/acos protection (4 tests) |

### P3: Infrastructure & Architecture

| Item | Source | Priority | Status |
|------|--------|----------|--------|
| NautilusBrain API (`ai.nautilus.*`) | hotSpring V0615 | HIGH | ✅ S80: 8 JSON-RPC methods in daemon (nautilus feature) |
| bingoCube standalone absorption | hotSpring V0615 | HIGH | ✅ S80: barracuda::nautilus module (7 files, 22 tests) |
| IPC evolution (multi-transport) | wateringHole | MEDIUM | ✅ Already exists: Unix/Abstract/TCP in ipc/platform |
| Batched encoder (fused pipeline) | neuralSpring V64 | MEDIUM | ✅ S80: `BatchedEncoder` (194 lines, 2 tests) |
| NPU bandwidth model | neuralSpring V60 | LOW | ✅ S81: BandwidthTier::PcieLow for AKD1000 in InterconnectTopology |
| `PipelineBuilder` CPU-only mode | wetSpring V82 | LOW | ✅ S80: StatefulPipeline<S> |
| metalForge Stage/Pipeline topology | groundSpring V61 | LOW | ✅ S81: SubstratePipeline + InterconnectTopology (capability-based routing) |

### P4: Lower Priority (Carried)

| Item | Source | Status |
|------|--------|--------|
| SparseGemmF64 (CSR × dense for NMF) | wetSpring V82 | ✅ Already exists: sparse_gemm_f64.rs + spmm_f64.wgsl |
| ESN 36-head MultiHeadEsn + ExportedWeights alignment | hotSpring V0615 | ✅ S79 |
| StatefulPipeline (water balance state) | airSpring V039 | ✅ S80: StatefulPipeline<S> + WaterBalanceState |
| NPU substrate kind in metalForge | neuralSpring V60 | ✅ S81: `SubstrateType::Npu` in device/substrate.rs |
| Streaming FASTQ/mzML/MS2 (bio I/O) | wateringHole V69 | ☐ |
| Pseudofermion HMC (477 lines) | wateringHole V69 | ✅ Already exists (CPU + GPU + shaders) — tracker stale |
| Omelyan integrator | wateringHole V69 | ✅ S83: `OmelyanIntegrator` wraps `GpuHmcLeapfrog` (2MN, λ=0.1932) |
| Richards PDE GPU solver | wateringHole V69 | ✅ S83: `RichardsGpu` multi-dispatch Picard (3 kernels) |
| `TensorSession::fused_mlp` | wateringHole V69 | ✅ S80: fused_mlp via BatchedEncoder |

---

## Deep Debt — Active

### Architecture

| ID | Description | Priority | Status |
|----|-------------|----------|--------|
| D-CD | ComputeDispatch migration (~155+ legacy ops) | High | 144 done (+12 S86: determinant, mse_loss, dice, quantize, dequantize, bce_loss, permute, movedim, logsumexp, index_add, tensor_split, concat), ~139 remaining (full audit revealed more ops than originally tracked) |
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
| Production stubs/mocks | ✅ S77+S82: TCP provider, EMA prediction, detect_capabilities→OS probing, LocalhostDiscovery→env-based |
| `legacy_primal_to_capabilities` / `legacy_primal_primary_capability` | ✅ S78: Removed (no callers); primal_capabilities now clean capability-to-primal mapping |
| CPU memory detection | ✅ S82: `estimate_system_memory()` reads `/proc/meminfo` (Linux) / `sysctl hw.memsize` (macOS) |
| AMQP port hardcoding | ✅ S82: Extracted `storage::AMQP_PORT` constant |

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
| `cargo test -p barracuda --lib` | ✅ 2,866 passed, 13 ignored (S84: net -13 from refactoring; all passing) |
| Workspace lib tests | ✅ 5,500+ passed |
| `#[serial]` tests | ✅ 0 remaining |
| Production sleeps (non-chaos) | ✅ 0 (documented exceptions: hardware polling, retry backoff) |
| Production mocks/stubs | ✅ Evolved (S84: frameworks.rs echo→error, experimental.rs stub→real probes; S86: deployment.rs docs→capability-discovery, wgpu_backend.rs magic numbers→device-limits) |
| WGSL shaders | 844 (0 orphans, 0 f32-only, 37 DF64, 15 folding, 2 bitcast-fixed) |
| God files refactored | 37+ (S84: hydrology.rs 690→mod.rs ~310 + gpu.rs ~280) |
| `cargo doc` | ✅ S77/S78: private intra-doc links fixed (ToadStoolError in universal_adapter, discovery_integration) |
| e2e runtime nesting | ✅ S77: `run_gpu_resilient_async` evolved to dedicated runtime |
| Zero-copy anti-patterns | ✅ S77: All `cast_slice().to_vec()` verified necessary, documented |
| Test coverage (llvm-cov) | 41.86% lines | Target: 90% — major gap remaining |
| Compile bottleneck analysis | S78 | tfhe+tfhe-fft = 30.6% CPU (showcase); wgpu 22/23 duplication wastes ~90s |

---

*This tracker is the single source of truth for evolution status. Updated each session.*
