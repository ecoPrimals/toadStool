# Evolution Tracker

**Date**: March 29, 2026 — S166
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
| hotSpring | v0.6.24 | ✅ Core absorbed | NVK serialization, brain arch, 31 experiments, NPU controlled params, adaptive dynamical HMC |
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
|| Deep debt S140 | toadStool | ✅ S140 evolved | **Hardcoding elimination** (7 production files → `interned_strings::primals::*`); **StreamingDispatch progress callbacks** (`StageProgress`, `ProgressCallback` from healthSpring V13); **barraCuda Sprint 2 API awareness** (3 new JSON-RPC: `science.activations.list`, `science.rng.capabilities`, `science.special.functions`); **Smart refactor** (science.rs 1139→828 LOC, science_domains.rs extracted); **Spring pins** (healthSpring V13 new); **Docs cleanup** (6 stale files → fossil); 19,900+ tests, 0 failures |
|| Spring absorption S145 | toadStool | ✅ S145 evolved | **PrecisionBrain** absorbed (hotSpring v0.6.25); **NvkZeroGuard** absorbed (airSpring v0.7.5); 8 new WorkloadPatterns (neuralSpring S140 + healthSpring V14.1); 5 new capability domains; capability.call format standardization; Spring-as-Provider ProviderRegistry; ServerConfig port hardcoding evolved; 19,965 tests |
|| Sovereign compute S150 | toadStool | ✅ S150 evolved | **VFIO GPU backend** (VfioBar0Access: full lifecycle container→group→device→BAR0 mmap, RegisterAccess impl); **BAR0 permissions** (nvpmu::permissions udev installer + setup-gpu-sovereign.sh); **nvpmu dedup** (apply_recipe delegates to hw-learn RecipeApplicator); **Live BAR0 apply** (JSON-RPC compute.hardware.apply with live:true); **Gap 5 auto_init** (auto-detect GPU → best recipe → BAR0 apply → confidence update); **NvPmuError::Hardware** variant; 19,567 tests |
|| Deep debt S149 | toadStool | ✅ S149 evolved | **Clippy 0 code warnings** (redundant guards, map_or, Error::other, long literal); **resolve_credential() fully wired** (file-based keyring + security provider JSON-RPC); **Handler refactor** (shader domain extracted: mod.rs 849→615); **Orphaned deps removed** (caps, console, ipnet, futures-util); **nouveau_drm evolved** (raw FFI → rustix); **hw-learn stubs implemented** (RegisterAccess trait + verify_register_via_access); **Interned constants** (primal names → primals::*); **Script fixes** (toadstool-runtime-display → toadstool-display); 20,192 tests |
|| Spring absorption S146 | toadStool | ✅ S146 evolved | **nvvm_transcendental_risk** in gpu.info response (hotSpring v0.6.26 request); **PrecisionBrain wired into compile_wgsl_multi** (per-device precision advice); **PcieTopologyGraph** marked stable (#[non_exhaustive] + empty() constructor); **SpringDomain expanded** (+10 new domains, SCREAMING_SNAKE_CASE per wetSpring V109, HealthSpring added); **gpu_memory_estimate_bytes** on WorkloadPattern + route_with_vram (healthSpring V19); **Cross-spring provenance** expanded (healthSpring PK/PD + DiversityIndex flows); Spring pins updated to hotSpring v0.6.27, neuralSpring V96, wetSpring V109, healthSpring V19; 20,015 tests |

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
| D-COV | Test coverage → 90% | Medium | 20,262 tests (S152); ~86% line coverage (~150K production lines). Mock hardware layers for V4L2/VFIO (S152). Focus: hardware-dependent code |
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
| `async-trait` | Required for dyn dispatch | ~102 uses across 53 files (S156 audit); NOTE(async-dyn) — conscious architectural decision (Rust dyn dispatch requires async-trait) |
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
| Workspace tests | ✅ **21,156 passed** (S156) |
| `#[serial]` tests | ✅ 0 remaining |
| Production sleeps (non-chaos) | ✅ 0 (documented exceptions: hardware polling, retry backoff) |
| Production mocks/stubs | ✅ 0 — all evolved to real implementations or proper errors |
| God files refactored | 40+ (all production files < 1000 lines) |
| Test coverage (llvm-cov) | ~83% lines (~182K production lines). Target: 90% |
| `unsafe` blocks | ~70+ — all `// SAFETY:` documented. 22 crates forbid, ~10 deny |
| File size limit | All < 1000 lines |

---

### Session S156 (Mar 16, 2026)

| Category | Change |
|----------|--------|
| runtime-specialty resurrected | 167 compile errors → 0. Core type drift fixed across ExecutionResponse, ExecutionRequest, ExecutionStatus, WorkloadType, RuntimeCapabilities. All trait impls aligned. |
| Hardcoding → constants | Dispatch 5000ms magic → `DISPATCH_DEFAULT_TIMEOUT` named constant |
| Unsafe → safe | `unreachable!()` in nvpmu/dma.rs → `Err(NvPmuError::Hardware(...))` |
| Doc warnings | 5 nvpmu bracket escapes, 2 specialty HTML tag fixes, unused CudaStream import removed |
| Test evolution | +313 net new tests (21,156 total). Specialty integration tests fully rewritten |
| Cleanup | 17.4 GB build garbage removed (profraw + target/), 2 orphan CSV files deleted |
| `warn(missing_docs)` | 4 crates retain lint; 39 deferred for progressive rollout |

### Session S153 (Mar 13, 2026)

| Category | Change |
|----------|--------|
| IPC-first reconciliation | Absorbed barraCuda v0.35 IPC-first architecture (zero compile-time deps). Updated pipeline: barraCuda →[JSON-RPC] coralReef →[JSON-RPC] toadStool → GPU |
| `compute.hardware.vfio_devices` | New JSON-RPC: discovers VFIO-bound GPUs. Sole VFIO detection source for barraCuda IPC-first |
| `ecoprimals-mode` CLI | `toadstool mode science/gaming/status` — GPU mode switching (display ↔ vfio-pci) |
| hotSpring VFIO absorption | 6/7 coralReef VFIO tests pass on Titan V (GV100). Dispatch blocked on coralReef USERD_TARGET encoding fix |
| Spring pins | hotSpring v0.6.31, coralReef Iter 43, barraCuda v0.35 IPC-first |

### Session S152 (Mar 13, 2026)

| Category | Change |
|----------|--------|
| Sovereign infrastructure | compute.dispatch.*, SOVEREIGN_BINARY_PIPELINE, GpuGen, auto_init_all, huge page DMA, MSI-X/eventfd, GpuPowerController, extern "C" elimination |
| OS keyring | D-Bus SecretService + macOS Keychain. Step 2.5 in resolve_credential chain |
| Cross-gate pooling | RemoteDispatcher (Unix + TCP), GateGpuInfo.endpoint, auto-forward |
| Mock hardware | MockV4l2Device + MockVfioDevice for headless CI |
| Coverage | 20,262 tests, 0 failures. ~150K production lines |

### Session S151 (Mar 12, 2026)

| Category | Change |
|----------|--------|
| Error recovery | RegisterSnapshot + apply_with_recovery + PartialInit rollback |
| DMA buffers | DmaAllocator + DmaBuffer (page-aligned, mlock, IOMMU-mapped) |
| PCI discovery | Unified PciFilter + vendor constants. Thermal safety. VFIO bind/unbind automation |

### Session S145 (Mar 10, 2026)

| Category | Change |
|----------|--------|
| PrecisionBrain | Absorbed from hotSpring v0.6.25: domain-aware routing brain with cached route table, F64 throttle detection (>8× ratio), 4-level PrecisionHint enum (Critical/Moderate/ThroughputBound/LowPrecision) |
| NvkZeroGuard | Absorbed from airSpring v0.7.5: zero-output detection for NVK on Volta (f64 + f32 variants), NaN contamination check, ZeroGuardVerdict enum |
| TierCapability evolution | Added `dispatch_latency_ratio` field for PrecisionBrain F64 throttle heuristic |
| Workload patterns | 8 new patterns absorbed from neuralSpring S140 + healthSpring V14.1: Pairwise (500K), BatchFitness (50K), HmmBatch (5K), SpatialPayoff (4K), Stochastic (100K), PopulationPk (100), DoseResponse (1K), DiversityIndex (500) |
| Capability domains | 5 new domains: biology, health, measurement, optimization, visualization |
| capability.call evolution | ISSUE-003: `qualified_method` format B support (`biology.phylo.infer`) alongside flat format A |
| Spring-as-Provider | ISSUE-007: `ProviderRegistry` for explicit Spring registration with socket path, methods, version; `prune_stale()` for liveness; falls back to filesystem convention |
| Hardcoding evolution | `ServerConfig::default()` port evolved from bare `8080` to `toadstool_config::ports` centralized lookup |
| Spring versions | hotSpring v0.6.25, groundSpring V100, neuralSpring V91/S140, wetSpring V99+, airSpring v0.7.5, healthSpring V14.1 |
| Coverage | 19,965 tests (18 new), 0 failures, 101 skipped |

### Session S144 (Mar 9, 2026)

| Category | Change |
|----------|--------|
| Flaky test fix | `test_concurrent_resource_monitoring_events`: event-type filtering for ResourceUsageUpdate in async broadcast |
| Clippy pedantic | 16+ violations fixed in `pcie_topology.rs`: `let..else`, `map_or`, `#[expect]`, `const fn` evolution |
| Hot-path optimization | `policy.rs`: eliminated N-1 intermediate `Arc::clone` in substrate selection |
| Magic string | `ai_mcp_interface`: hardcoded "45 minutes" evolved to computed average from session timestamps |
| Clippy auto-fix | `workload_routing.rs`, `workload_health.rs`: `doc_markdown`, `map_unwrap_or` |

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
| Coverage | ~86% line (121K production lines), 19,840+ tests |

### Session S131+ (Mar 7, 2026)

| Category | Change |
|----------|--------|
| Spring sync | All 5 springs pinned: neuralSpring V89/S131, wetSpring V97e, airSpring V0.7.3, groundSpring V96, hotSpring v0.6.19 |
| Lint evolution | `#[allow]` → `#[expect(lint, reason)]` (20+ attributes); 3 stale suppressions discovered and removed |
| Deep debt scan | All files <1000L, unsafe=HW FFI only, no production hardcoding, mocks test-isolated |
| IPC namespace | `science.*` resolved: toadStool canonical proxy, springs may call barraCuda directly |
| coralReef milestone | First E2E sovereign GPU dispatch on AMD RX 6950 XT (pure Rust WGSL→PM4→readback) |
| Coverage | ~86% line coverage (121K production lines) |

### hotSpring Integration Sprint (Mar 26, 2026)

| Category | Change |
|----------|--------|
| **Socket discovery** | hotSpring now discovers `$XDG_RUNTIME_DIR/biomeos/toadstool.jsonrpc.sock` (toadStool's actual path). Previous code only checked flat layout. |
| **Wire format** | Fixed `silicon_unit` mismatch: hotSpring was sending `"shader"` but toadStool `SiliconUnit` deserializes as `"shader_core"` (snake_case enum). All report paths now use correct wire names. |
| **Precision matrix** | `validate_precision_matrix` now reports to `compute.performance_surface.report` with `math.*` operation IDs (`math.arith.mul.df64`, `math.reduce.sum.fp32`, etc.), proper `silicon_unit: "shader_core"`, and measured tolerance. |
| **SPIR-V bridge** | New `barracuda-spirv` crate in barraCuda workspace — isolates the single `unsafe` call to `create_shader_module_passthrough`. Enables Tier 1 sovereign SPIR-V (naga IR → SPIR-V → GPU, bypasses WGSL re-emission). Gated behind `spirv-passthrough` feature flag. |

**Evolution priorities for toadStool team:**

1. **Performance surface persistence** — in-memory `RwLock<Vec<>>` is lost on restart. Needs JSON-lines or SQLite backing so silicon characterization accumulates across sessions.
2. **Socket path standardization** — all primals should agree on `$XDG_RUNTIME_DIR/biomeos/` layout. Document in wateringHole.
3. **Precision routing bridge** — expose `gpu.precision_routing` method combining toadStool's hardware truth with barraCuda's `PrecisionRoutingAdvice` for one-call routing.
4. **hw-learn recipes** — observe/distill/apply pipeline needs real init traces from NVIDIA and AMD to populate the knowledge store.
5. **SPIR-V compile proxy** — when coralReef native compilation is ready (biomegate), `shader.compile.spirv` should route through coralReef for sovereign SPIR-V, eliminating naga dependency long-term.

### Sessions S95–S96 (Mar 6, 2026)

| Category | Change |
|----------|--------|
| Sovereign pipeline | `HardwareFingerprint`, `is_sovereign_capable()`, `safe_allocation_limit`, `SubstrateCapabilityKind` (12 variants) |
| Substrate expansion | `SubstrateType` 4→8 variants (IntegratedGpu, Npu, Tpu, Fpga, Dsp, Quantum) |
| God file splits | dispatch.rs, detection.rs, engine.rs, protocols/lib.rs, specialized_templates.rs |
| API orphan | crates/api/ ByobApi → container; dependency removed |
| Unsafe docs | V4L2 `// SAFETY:` on all blocks |
| Debris cleanup | Root tests/ stubs, stale checklists (11 files), false-positive TODOs |
| management/resources | Real ResourceManager — sysinfo→toadstool-sysmon (pure Rust) S137 |
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
