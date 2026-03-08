# Evolution Tracker

**Date**: March 8, 2026 — S133
**Philosophy**: Deep debt solutions pay off. Modern idiomatic Rust. Capability-based discovery. Self-knowledge only. Zero-cost abstractions.

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
9. **Concurrency-first** — no sleeps in non-chaos tests; test issues are production issues (S87: validated by hardware_verification + hotspring fault test fixes)
10. **Device resilience** — all GPU paths protected by catch_unwind; errors propagate as Result

---

## Spring Absorption — Completed

All P0 dispatch wiring complete. Core absorption from 5 springs validated:

| Spring | Version | Status | Key Deliverables |
|--------|---------|--------|-----------------|
| neuralSpring | V90/S132 | ✅ Core absorbed | 39/39 CPU↔GPU parity, AlphaFold2 17 shaders, HillGateGpu, SwarmNnGpu, DF64 ML primitives |
| wetSpring | V99 | ✅ Core absorbed | 144 primitives, ValidationHarness, 52 papers |
| airSpring | v0.7.5 | ✅ Core absorbed | Ops 0-8, seasonal pipeline, 72 experiments, 25 Tier A GPU |
| groundSpring | V99 | ✅ Core absorbed | 95/95 three-tier parity, wright_fisher, grid ops, tissue_anderson |
| hotSpring | v0.6.23 | ✅ Core absorbed | NVK serialization, brain arch, 31 experiments, NPU controlled params |
| wateringHole | V69 | ✅ Core absorbed | Chi-squared batch, MC ET0 propagate |
| groundSpring | V61 | ✅ S81 absorbed | InterconnectTopology, SubstratePipeline, BandwidthTier (PCIe P2P routing) |
| neuralSpring | V70 | ✅ S81 absorbed | IFFT/NTT/INTT buffer fixes, `enable f64;` stripping |
| Cross-spring S83 | All springs | ✅ S83 absorbed | BrentGpu, anderson_4d+Wegner, Omelyan, RichardsGpu, L-BFGS, BatchedStatefulF64, HeadKind generalization, SpectralBridge, ESN shape hardening |
| Deep debt S84 | toadStool | ✅ S84 evolved | 9 ops → ComputeDispatch, hydrology god-file refactored, experimental.rs stub → real probes, frameworks.rs echo → proper error, mDNS constants extracted |
| Deep debt S86 | toadStool | ✅ S86 evolved | 12 ops → ComputeDispatch (determinant, mse_loss, dice, quantize, dequantize, bce_loss, permute, movedim, logsumexp, index_add, tensor_split, concat); wgpu_backend.rs magic numbers → real device limits; deployment.rs stubs → capability-discovery docs |
| Deep debt S87 | toadStool | ✅ S87 evolved | TODO(afit)→NOTE(async-dyn) (75 instances, 52 files); gpu_helpers 663L→3 submodules; unsafe audit (~60+ sites documented); FHE shader fixes; hardware_verification 13/13 pass; hotspring fault tests fixed |
| Deep audit S90 | toadStool | ✅ S90 evolved | REST API + handlers removed; 2,780+ SPDX headers; license workspace unified; `get_socket_path_for_capability()` API; Arc-cached kernels; PyO3 feature-gated; capability-based trust; 15 JSON-RPC integration tests |
| Coverage + debt S92 | toadStool | ✅ S92 evolved | +47 tests → 5,369; dead middleware eliminated (~131 KB); sovereignty deprecations formalized; BearDog strings neutralized; ecoBin pure-rust verified; `find_pattern_by_capability()` API |
| Deep debt S94 | toadStool | ✅ S94 evolved | Dead barracuda dep removed; crates/barracuda fossilized (15MB→archive); manual_jsonrpc deleted (8 files); vfio.rs 971L→4-module directory; all files <1000L; 17,986 tests pass |
| Deep execution S94b | toadStool | ✅ S94b evolved | **NpuDispatch trait** (generic + AkidaNpuDispatch adapter); **NpuParameterController trait** (hotSpring absorption); **GpuAdapterInfo** (driver/f64/workgroup for barraCuda); Multi-adapter selection (`TOADSTOOL_GPU_ADAPTER`); NestGate mock→real RPC; placeholder crate removed; production mock audit complete; **D-SOV sovereignty migration** (7 callers → capability-based); hardcoded ports → config constants; integration-tests barracuda dep → optional |
| Debris + coverage S95 | toadStool | ✅ S95 evolved | Root `tests/` stubs removed; stale checklists cleaned (11 files); false-positive TODOs removed; sprint/date doc comments cleaned; management/resources re-added as real ResourceManager; clippy pedantic resolved |
| Sovereign pipeline S96 | toadStool | ✅ S96 evolved | **HardwareFingerprint** (TFLOPS, sovereign_capable); **SubstrateCapabilityKind** (12 variants); **SubstrateType** 4→8 variants; 5 god files split (dispatch, detection, engine, protocols, templates); crates/api/ orphan resolved; V4L2 SAFETY docs; hardcoded IP → env var |
| Spring absorption S97 | toadStool | ✅ S97 evolved | NVK Volta f64 probe (`f64_compute_unreliable`, `has_reliable_f64()`); subgroup size detection; `AdaptiveSimulationController` trait; `ProxyFeature` struct; `NpuInferenceRequest`; science.* IPC namespace (10 methods); ecoBin compliance (ring/zstd removed); +59 tests |
| Deep debt S128 | toadStool | ✅ S128 evolved | **f64_shared_memory_reliable** on GpuAdapterInfo (groundSpring V84-V85 bug); **sovereign_binary_capable** on HardwareFingerprint; **PrecisionRoutingAdvice** enum + `precision_routing()` method; shader.compile.* IPC (4 methods); `discover_capabilities` dynamically built from registry; `query_available_backends()` runtime probing; architecture stubs evolved (auth TrustLevel/CapabilityToken, scheduling Priority/PlacementConstraint/Decision); +25 tests |
| Deep debt S129 | toadStool | ✅ S129 evolved | **C dep elimination** (flate2→rust_backend, procfs default features disabled); **Capability-based ports** (`resolve_capability_or_legacy_port()`); 5 god files refactored (ipc/server 987→428, container/lib 981→582, ecosystem 963→556, handler/mod 832→610, nestgate/client 824→555); **Zero-copy hot paths** (Cow/Arc<str>); BYOB API state ownership split; 200+ coverage tests; long-running test debt (1,237x speedup); 19,109 tests, 0 failures |

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
| D-NPU | ~~NpuDispatch trait~~ | **RESOLVED S94** | `toadstool-core::npu_dispatch` — generic `NpuDispatch` trait + `AkidaNpuDispatch` adapter |
| D-COV | Test coverage → 90% | Medium | 19,820+ tests; ~86% line coverage (121K production lines). Focus: hardware-dependent code |
| D-SOV | ~~Sovereignty migration~~ | **RESOLVED S94b** | All 7 production callers migrated to `get_socket_path_for_capability()` |
| D-WC | Wildcard re-exports remaining | Low | 13 crates narrowed; remaining have 15+ items (justified) |
| — | ~~vfio.rs smart refactoring~~ | **RESOLVED S94** | 971L → `vfio/` directory (types.rs, ioctl.rs, dma.rs, mod.rs) |

**Transferred to barraCuda team (S93):** D-CD (ComputeDispatch, ~139 remaining), D-DF64 (precision strategy), naga-IR optimizer Phases 4+, barraCuda budding Phases 1-4.

### God Files — All Resolved

40+ god files smart-refactored across S69–S96. All production files under 1000 lines. Recent splits (S96):

| File | Original | Result | Session |
|------|----------|--------|---------|
| `cli/commands/dispatch.rs` | 1252L | 7 domain modules | S96 |
| `distributed/universal/detection.rs` | 1004L | 3 modules (helpers, gpu, mod) | S96 |
| `runtime/gpu/engine.rs` | 1098L | 2 modules (mod, tests) | S96 |
| `integration/protocols/lib.rs` | 985L | 2 modules (bear_dog extracted) | S96 |
| `cli/templates/specialized_templates.rs` | 924L | 4 modules (ml_science, infrastructure, custom, mod) | S96 |

Barracuda god files (wgpu_device, driver_profile, probe, capabilities, etc.) transferred to barraCuda (S93–S94).

### Dependency Evolution

| Dependency | Status | Path |
|------------|--------|------|
| `async-trait` | Required for dyn dispatch | ~75 uses; S87: TODO(afit)→NOTE(async-dyn) — reclassified as conscious architectural decision (Rust 1.92) |
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
| `get_socket_path_for_service` / `get_primal_default_port` / `capability_typical_provider` | ✅ S92: Deprecated with `#[deprecated(since = "0.92.0")]`. NestGate client migrated. Migration bridge documented in `integrator_impl.rs`. |
| BearDog user-facing strings | ✅ S92: Neutralized in access control manager (5 locations) + JSON-RPC version_info |
| CPU memory detection | ✅ S82: `estimate_system_memory()` reads `/proc/meminfo` (Linux) / `sysctl hw.memsize` (macOS) |
| AMQP port hardcoding | ✅ S82: Extracted `storage::AMQP_PORT` constant |

### Unsafe Code

| Status | Count | Notes |
|--------|-------|-------|
| Total `unsafe` blocks | ~70+ | All `// SAFETY:` documented (S77+S87+S131+: barracuda + runtime/gpu + V4L2 audit) |
| Reducible | 0 | S77: All verified necessary (64-byte aligned alloc, wgpu FFI, CUDA FFI) |
| `#![deny(unsafe_code)]` | 36 crates | 2 justified exceptions: gpu, secure_enclave |
| SAFETY comments | ✅ | S77: Invariants, violation effects, and justification documented |

---

## Quality Gates

| Gate | Status |
|------|--------|
| `cargo check --workspace` | ✅ 0 errors |
| `cargo clippy --workspace --all-targets -- -D warnings -W clippy::pedantic` | ✅ 0 warnings (S131+: `#[expect]` evolution) |
| `cargo fmt --all -- --check` | ✅ 0 diffs |
| `cargo doc --workspace --no-deps` | ✅ 0 warnings |
| Workspace tests | ✅ 19,820+ passed (S133) |
| `#[serial]` tests | ✅ 0 remaining |
| Production sleeps (non-chaos) | ✅ 0 (documented exceptions: hardware polling, retry backoff) |
| Production mocks/stubs | ✅ 0 — all evolved to real implementations or proper errors |
| God files refactored | 40+ (all production files < 1000 lines) |
| Test coverage (llvm-cov) | ~86% lines (121K production lines, excl GPU SIGSEGV). Target: 90% |
| `unsafe` blocks | ~70+ — all `// SAFETY:` documented (S131+: full audit) |
| File size limit | All < 1000 lines |

---

### Session S133 (Mar 8, 2026)

| Category | Change |
|----------|--------|
| Ada Lovelace reclassification | GPU adapter classification updated for Ada architecture |
| f64_zeros_risk | f64 shared-memory zeros risk tracking and mitigation |
| fused_ops_healthy() | Fused operations health check added |
| 14 ecology.* methods | New ecology domain JSON-RPC methods for ecosystem integration |
| NUCLEUS discovery | NUCLEUS capability discovery and routing |
| deploy graph routing | Deploy graph routing and workload placement |
| 20 semantic methods | Semantic method registry expanded 71→91 |
| Spring versions | hotSpring v0.6.23, groundSpring V99, neuralSpring V90/S132, wetSpring V99, airSpring v0.7.5 |
| Coverage | ~86% line (121K production lines), 19,820+ tests |

### Session S131+ (Mar 7, 2026)

| Category | Change |
|----------|--------|
| Spring sync | All 5 springs pinned: neuralSpring V89/S131, wetSpring V97e, airSpring V0.7.3, groundSpring V96, hotSpring v0.6.19 |
| Lint evolution | `#[allow]` → `#[expect(lint, reason)]` (20+ attributes); 3 stale suppressions discovered and removed |
| Deep debt scan | All files <1000L, unsafe=HW FFI only, no production hardcoding, mocks test-isolated |
| IPC namespace | `science.*` resolved: toadStool canonical proxy, springs may call barraCuda directly |
| coralReef milestone | First E2E sovereign GPU dispatch on AMD RX 6950 XT (pure Rust WGSL→PM4→readback) |
| Coverage | ~85% line coverage (121K production lines) |

### Sessions S95–S96 (Mar 6, 2026)

| Category | Change |
|----------|--------|
| Sovereign pipeline | `HardwareFingerprint`, `is_sovereign_capable()`, `safe_allocation_limit`, `SubstrateCapabilityKind` (12 variants) |
| Substrate expansion | `SubstrateType` 4→8 variants (IntegratedGpu, Npu, Tpu, Fpga, Dsp, Quantum) |
| God file splits | dispatch.rs, detection.rs, engine.rs, protocols/lib.rs, specialized_templates.rs |
| API orphan | crates/api/ ByobApi → container; dependency removed |
| Unsafe docs | V4L2 `// SAFETY:` on all blocks |
| Debris cleanup | Root tests/ stubs, stale checklists (11 files), false-positive TODOs |
| management/resources | Re-added as real ResourceManager with sysinfo |
| Root docs | All updated to S96 |
| Spring tracker | Updated to current versions (hotSpring v0.6.17, groundSpring V80, neuralSpring V86/S128, wetSpring V97d, airSpring V071) |

### Session 93 (Mar 3, 2026)

| Category | Change |
|----------|--------|
| Debt transfer | D-DF64, D-CD, barraCuda budding, naga-IR optimizer, DF64 transcendentals → barraCuda team |
| Debris cleanup | 12 stale docs deleted (~90 KB) |
| Root docs | All bumped to S93; NEXT_STEPS refocused on toadStool-only work |
| Handoff | `wateringHole/handoffs/TOADSTOOL_S93_DF64_HANDOFF_MAR03_2026.md` created |

*This tracker is the single source of truth for evolution status. Updated each session.*
