# Active Technical Debt Register

**Date**: March 6, 2026
**Philosophy**: Math is universal, precision is silicon. Workarounds are
short-term solutions that increase debt. We aim to solve deep debt over
iterations, evolving toward vendor-agnostic, capability-based solutions.

---

## Active Workarounds

### W-001: f64 Transcendental Polyfills — Transferred to barraCuda (S93)

**Status**: TRANSFERRED — barraCuda team owns precision strategy and polyfill infrastructure
**Impact**: Enables f64 transcendentals on ALL GPUs regardless of vendor math library support

**Root Cause**: SPIR-V has no mechanism to link vendor math libraries (NVIDIA libdevice, AMD ocml).
Every f64 transcendental fails through SPIR-V on NVK/NAK, NVIDIA proprietary (Ada), and RADV.

**Solution**: `math_f64.wgsl` — 28 pure-WGSL polyfill functions (Cody-Waite range reduction,
Lanczos gamma, Horner polynomials). Auto-injected by `compile_shader_f64()`. No vendor
dependencies, works on every GPU, ships with the crate, testable in CI without hardware.

**Files** (in `ecoPrimals/barraCuda/`):
- `crates/barracuda/src/shaders/math/math_f64.wgsl` — 28 polyfill functions
- `crates/barracuda/src/shaders/precision/mod.rs` — `inject_missing_math_f64()`, `patch_transcendentals_in_code()`
- `crates/barracuda/src/device/wgpu_device/capabilities.rs` — `needs_f64_exp_log_workaround()`
- `crates/barracuda/src/device/probe.rs` — runtime capability probing, global cache

**F64 Built-in Capability Matrix** (probed Feb 18, 2026):

| Function     | RTX 3090 (Ampere) | RX 6950 XT (RDNA2) | Titan V (NVK/NAK) |
|-------------|-------------------|---------------------|-------------------|
| exp, log    | NATIVE            | fallback            | fallback          |
| sin, cos    | NATIVE†           | fallback            | TBD               |
| sqrt, fma   | NATIVE            | **NATIVE**          | TBD               |
| abs/min/max | NATIVE            | **NATIVE**          | TBD               |

†NVIDIA PTXAS sin/cos on f64 uses MUFU — likely f32 precision in f64 register.

**Evolution Path**:
1. DONE: Capability probing (`probe_f64_builtins()`) + fossil substitution
2. DONE: Fossil f64 functions (abs, sqrt, min, max, etc.) marked and auto-substituted
3. Upstream ACO fix: Contribute `fexp2(f64)` to Mesa ACO for RDNA2/3
4. Upstream NAK fix: Contribute `exp(f64)` lowering to Mesa NAK

---

### W-003: NAK Compiler 149x Performance Gap — Transferred to barraCuda (S93)

**Status**: TRANSFERRED — barraCuda team owns compiler optimization and hw validation
**Impact**: NVK/NAK Jacobi eigensolve ~9x slower than NVIDIA proprietary after warp-packing

**Phases**:

| # | Phase | Status |
|---|-------|--------|
| 1 | SM70 instruction latency tables | **DONE** — `sm70_instr_latencies.rs`, DFMA=8cy |
| 2 | f64 FMA selection (mul+add → DFMA) | Pending |
| 3 | Loop unrolling for bounded nested loops | Pending |
| 4 | Sovereign naga-IR FMA fusion + DCE | **DONE** — Phase 4 compiler |

**First solution absorbed**: Warp-packed eigensolve (`@workgroup_size(32,1,1)`) — 2.2x NVK speedup.
`GpuDriverProfile::optimal_eigensolve_strategy()` — data-driven strategy selection.

**Tracking**: https://gitlab.freedesktop.org/mesa/mesa/-/tree/main/src/nouveau/compiler

---

## Remaining Debt

### Architecture

| ID | Description | Priority | Notes |
|----|-------------|----------|-------|
| D-NPU | ~~NpuDispatch trait~~ | **RESOLVED S94** | `toadstool-core::npu_dispatch` — generic `NpuDispatch` trait + `AkidaNpuDispatch` adapter |
| D-RING | ~~ring C FFI in dev-deps~~ | **RESOLVED S97** | `reqwest` removed from integration-tests; `zstd` → `ruzstd` (pure Rust) |
| D-COV | Test coverage → 90% | Medium | ~85% line coverage. 6,176 lib tests (18,028+ total workspace). Focus: low-coverage crates (CLI, distributed, auto_config, edge). |
| D-SOV | ~~Sovereignty: primal-name → capability~~ | **RESOLVED S94b** | All production callers migrated to `get_socket_path_for_capability()`. Deprecated definitions retained for fallback only. |
| D-WC | Wildcard re-exports remaining | Low | 13 crates narrowed; remaining have 15+ items each (justified) |

### Transferred to barraCuda Team (S93)

| ID | Description | Notes |
|----|-------------|-------|
| D-CD | ComputeDispatch migration (~139 remaining) | Lives in barraCuda crate |
| D-DF64 | DF64 as default precision path | barraCuda owns precision strategy. Handoff: `wateringHole/handoffs/TOADSTOOL_S93_DF64_HANDOFF_MAR03_2026.md` |
| W-001 | f64 transcendental polyfills (28 functions) | Architecturally solved; sovereign solution |
| W-003 | NAK compiler 149x performance gap | Phases 1+4 done; Titan V hw validation pending |
| — | DF64 transcendental coverage (COMPLETE S71) | 15 functions in `df64_transcendentals.wgsl` |
| — | Sovereign compiler Phase 4+ | FMA fusion, DCE done; register pressure, peepholes, naga→NAK remaining |

### Cross-Repo Debt

| ID | Description | Status |
|----|-------------|--------|
| D-S20-003 | neuralSpring `evolved/` migration (~2075 lines) | Awaiting neuralSpring team |
| D-S18-002 | cubecl transitive `dirs-sys` | Needs upstream PR |

### Lower Priority (Carried)

| ID | Description | Status |
|----|-------------|--------|
| D-S46-001 | Conv2D/Pool WGSL shader evolution (stride/padding/channels/batch) | GPU shaders exist, lack full parameter support |
| D-S18-003 | e2e, fhe, comprehensive pending integration tests | Require future APIs |

---

## Recently Resolved (S95–S96 — Mar 6, 2026)

| Item | Resolution |
|------|-----------|
| Sovereign pipeline infrastructure | `HardwareFingerprint`, `is_sovereign_capable()`, `safe_allocation_limit` (NVK PTE guard), 12-variant `SubstrateCapabilityKind` |
| `SubstrateType` expansion | 4→8 variants: IntegratedGpu, Npu, Tpu, Fpga, Dsp, Quantum (metalForge alignment) |
| God file splits (5) | `dispatch.rs` (1252→7 modules), `detection.rs` (1004→3), `engine.rs` (1098→2), `protocols/lib.rs` (985→2), `specialized_templates.rs` (924→4) |
| `crates/api/` orphan | ByobApi extracted to `container/src/byob_routes.rs`; api crate no longer a dependency |
| V4L2 `// SAFETY:` documentation | All `unsafe` blocks in `v4l2/device.rs` documented with invariants |
| Hardcoded discovery IP | `0.0.0.0` → `TOADSTOOL_DISCOVERY_BIND_ADDR` env var |
| Root `tests/` debris | Stale test stubs removed; spec docs fossilized to `ecoPrimals/fossil/` |
| Stale completion checklists | Removed trailing `✅ COMPLETE` blocks from 11 files |
| `management/resources` re-added | Real `ResourceManager` with sysinfo (was placeholder removed in S94b) |
| Clippy pedantic | Resolved across workspace; `cargo clippy --lib -- -W clippy::pedantic` clean |

## Recently Resolved (Deep Debt Execution — Mar 5, 2026)

| Item | Resolution |
|------|-----------|
| Hardware Transport wiring | JSON-RPC `transport.discover/list/route` + CLI `toadstool transport discover/list/status` |
| Pixel format mismatch | CaptureTransport `AB24` → `AR24` to match DisplayTransport's `Argb8888` |
| Double-buffer alternation | `DisplayTransport` now alternates buffers via `write_idx ^= 1` after each flip |
| Detection stubs (11) | CPU, memory, distro, GPU, OpenCL, ROCm, neuromorphic, edge/IoT → real /proc + command parsing |
| `security.rs` god file (771L) | Smart-refactored → `security/` with types.rs, policy.rs, context.rs, provider.rs |
| `config_utils/mod.rs` god file (777L) | Smart-refactored → paths.rs, network.rs, environment.rs, defaults.rs |
| `FrameworkHandle::Placeholder` | → `FrameworkHandle::Unavailable { name, reason }` with explicit context |
| Hardcoded primal names (35+) | Evolved to `well_known::*` constants across primal_sockets, adapters, templates |
| Production `unwrap()` (frame protocol) | Replaced with direct array indexing in `decode_frame` |
| `management/resources` placeholder | Evolved to real `ResourceManager` with sysinfo (CPU, memory, disk tracking) |
| `collect_biome_status` stub | Real runtime directory scanning for socket/PID files |
| `#![allow(clippy::unused_async)]` | Removed crate-level suppression from distributed (zero warnings without it) |
| Dead code (15 fields) | Prefixed with `_`; 3 functions gated to `#[cfg(test)]` |
| Idiomatic Rust patterns | `div_ceil`, `is_some_and`, `is_ok_and`; rust-version 1.80→1.82 |

## Recently Resolved (S94 — Deep Debt Execution + Spring Absorption)

| Item | Resolution |
|------|-----------|
| Dead barracuda dependency | Removed from `core/toadstool/Cargo.toml` — zero imports found; barracuda is a peer primal, discovered at runtime via capability-based IPC |
| Embedded `crates/barracuda/` (15MB) | Moved to `ecoPrimals/fossil/toadStool/barracuda-fossil-S94b/` (S94b) |
| `manual_jsonrpc` module | Deleted entirely (8 files + integration tests). All capabilities ported to `pure_jsonrpc`. Doc references updated. |
| `vfio.rs` god file (971L) | Smart-refactored into `vfio/` directory: `types.rs` (kernel ABI), `ioctl.rs` (safe wrappers), `dma.rs` (DmaBuffer), `mod.rs` (backend integration) |
| Production panics/unwraps | Audited — all panics and unwraps are in `#[cfg(test)]` code; production code is clean |
| Sovereignty audit | `get_socket_path_for_capability()` is canonical; deprecated name-based APIs preserved for fallback only |
| All files < 1000 lines | Largest: 936 (test file). All production code well under limit. |
| **D-NPU: NpuDispatch trait** | `toadstool-core::npu_dispatch` — generic `NpuDispatch` trait + `AkidaNpuDispatch` adapter + `NpuModelHandle`. Vendor-agnostic, capability-based, zero-copy input (`Cow`). |
| **NpuParameterController trait** | `toadstool-core::npu_controller` — generic NPU-driven parameter tuning abstraction (absorbed from hotSpring). `ParameterSuggestion<P>`, `SafetyClamp<P>`, `SuggestionSource`, `ControllerError`. |
| **GpuAdapterInfo** | `toadstool-runtime-universal::GpuAdapterInfo` — exposes driver name, vendor/device ID, f64 support, workgroup limits for barraCuda's `GpuDriverProfile`. |
| **Multi-adapter GPU selection** | `TOADSTOOL_GPU_ADAPTER` env var: comma-separated fallback (index, name substring, "auto"). Absorbed from hotSpring's `adapter.rs`. |
| **NestGate production mock → real RPC** | `store_artifact`/`retrieve_artifact` evolved from hardcoded stubs to real JSON-RPC calls (`storage.artifact.store`/`storage.artifact.retrieve`) with graceful fallback. |
| **Placeholder crate removed** | `management/resources` excluded from workspace — no implementation, was polluting build graph. |
| **D-SOV: Sovereignty migration** | All 7 production callers of `get_socket_path_for_service` migrated to `get_socket_path_for_capability()`. CLI filesystem/socket discovery uses capability names directly. Deprecated APIs retained for backward compatibility. |
| **Hardcoded ports → config constants** | CLI `8080` → `ConfigUtils::get_toadstool_port()`, `9090` → `ports::toadstool::METRICS`. Network policy port reads from config. |
| **integration-tests barracuda dep** | Made optional (zero imports found in crate). Workspace builds without barraCuda present. |
| Build verification | `cargo fmt` ✅ `cargo clippy -D warnings` ✅ `cargo doc` ✅ `cargo test` ✅ (all pass, 0 fail) |

## Recently Resolved (S87)

| Item | Resolution |
|------|-----------|
| TODO(afit) migration | 75 instances across 52 files → NOTE(async-dyn); reclassified from debt to conscious architectural decision (async-trait required for dyn traits in Rust 1.92) |
| gpu_helpers.rs | 663 lines → 3 cohesive submodules (buffers.rs, bind_group_layouts.rs, pipelines.rs) |
| Unsafe code audit | All ~60+ unsafe sites across barracuda + runtime/gpu documented with SAFETY comments; all verified necessary |
| Hardware verification tests | 3 pre-existing failures fixed (kernel router threshold, cross-vendor adapter feature detection); 13/13 pass |
| Hotspring fault tests | 6 pre-existing failures fixed — input validation (LinearMixer, Gradient1D), relaxed GPU NaN/Infinity assertions, device capability checks |
| FHE shader arithmetic | u64_mod_simple rewritten in fhe_ntt.wgsl + fhe_intt.wgsl; mod_mul fixed in fhe_pointwise_mul.wgsl; 19 FHE tests pass |
| MatMul/FHE validation | Inner-dimension validation in MatMul::execute(); minimum degree ≥ 2 in FheNtt::new() |
| FHE chaos test | Random moduli constrained to NTT-friendly primes (12289, 65537) |
| Device-lost recovery | BarracudaError::is_device_lost() + with_device_retry test helper |

## Recently Resolved (S84–S86)

| Item | Resolution |
|------|-----------|
| ComputeDispatch 111→144 | 33 ops migrated across 3 sessions: 9 (S84: losses + matmul + gemm) + 12 (S85: metrics + ML core) + 12 (S86: math + tensor ops + losses) |
| hydrology.rs god file | Smart-refactored 690L → hydrology/ directory (mod.rs ~310 + gpu.rs ~280) |
| experimental.rs stub | Evolved to real FPGA/neuromorphic/quantum probes with env/device-path detection |
| frameworks.rs echo | Placeholder "echo input" → proper error with migration guidance |
| wgpu_backend.rs magic numbers | `num_units: 1000`, `memory_bandwidth: 500GB/s`, `optimal_batch_size: 10000` → real `device.limits()` queries |
| deployment.rs stubs | 10 placeholder methods → capability-discovery documentation |
| mDNS constants | Inline `"224.0.0.251"` + `5353` → named `MDNS_MULTICAST_ADDR` + `MDNS_PORT` |

## Recently Resolved (S79–S80)

| Item | Resolution |
|------|-----------|
| bingoCube Nautilus standalone absorption | `barracuda::nautilus` module — 7 files, 22 tests. Board, Evolution, Population, Readout, Shell, Brain. |
| `ai.nautilus.*` JSON-RPC (8 methods) | status, observe, train, predict, screen, edges, shell.export, shell.import — feature-gated `nautilus` in CLI |
| `BatchedEncoder` (fused pipeline) | Single `CommandEncoder` for multi-op GPU dispatches. `BatchedPassBuilder` API. 194 lines, 2 tests. |
| `fused_mlp` | MLP forward pass via BatchedEncoder — single submit across layers |
| Batch Nelder-Mead GPU | N independent optimizations in parallel, batched simplex shader ops |
| `StatefulPipeline<S>` | Generic pipeline for day-over-day state tracking + `WaterBalanceState` |
| `GpuDriverProfile` sin/cos F64 | Taylor-series preamble for NVK; `asin`/`acos` protected. 4 tests. |
| `NeighborMode::PrecomputedBuffer` | 2D/3D/4D periodic lattice precomputation. 6 tests. |
| `BatchedMultinomialGpu` alignment | `cumulative_probs` + `seed` config (groundSpring V37) |
| ComputeDispatch 76→95 | 19 ops migrated in 4 batches |
| Socket resolution consolidation | 4 scattered call sites → `toadstool_common::primal_sockets` API |
| ESN MultiHeadEsn + ExportedWeights | 36-head, 6 HeadGroup variants, head_disagreement(), spectral extensions |
| `SparseGemmF64` confirmation | Already exists: CSR×dense SpMM + spmm_f64.wgsl |
| IPC multi-transport confirmation | Already exists: Unix/Abstract/TCP in ipc/platform |

## Recently Resolved (S78)

| Item | Resolution |
|------|-----------|
| `libc` in akida-driver | Fully removed — migrated to `rustix` for all VFIO ioctls (vfio.rs, mmio.rs). Custom `VfioIoctlReturn`/`VfioIoctlPtr` safe wrappers. |
| `legacy_primal_to_capabilities` / `legacy_primal_primary_capability` | Removed from primal_capabilities.rs (no callers). Module evolved to clean capability-to-primal reference mapping. |
| 5 broken `ToadStoolError` doc links | Fixed in universal_adapter/mod.rs, discovery_integration.rs |
| Wildcard re-exports | 7 more crates narrowed (sandbox, wasm, edge discovery/toolchain/comms/deployment). Total: 13. |

## Recently Resolved (S77)

| Item | Resolution |
|------|-----------|
| `cargo fmt` 340 diffs | Formatted entire workspace |
| `cargo clippy` deprecated discovery | `discover_beardog_at`/`discover_nestgate_at` removed; tests evolved to `discover_service_by_capability` |
| `cargo doc` private link | Fixed `select_with_preference` doc link in `unified.rs` |
| e2e runtime nesting | `run_gpu_resilient_async` evolved to spawn dedicated tokio runtime (no more nested `block_on`) |
| `batched_elementwise_f64.rs` (967L) | Smart-refactored into 4-module directory: op, cpu_ref, executor, mod |
| `capabilities.rs` (912L) | Smart-refactored into 3-module directory: wgpu, device_info, mod |
| `fhe_shader_unit_tests.rs` (1028L) | Smart-refactored into 8-file `tests/fhe/` directory: ntt, intt, pointwise, fast_poly_mul, error_handling, performance, helpers |
| TCP security provider stub | Implemented `TcpSecurityProvider` with JSON-RPC 2.0 over TCP |
| Performance prediction placeholder | Implemented EMA-based `PredictionModel` with confidence scoring |
| Embedded programmer/emulator stubs | Evolved to proper `Err(not_supported(...))` returns |
| CPU resource placeholder | Implemented real byte-mixing compute operation |
| Hardcoded K8s/Docker ports | Configurable via `TOADSTOOL_DISCOVERY_HTTP_PORT` |
| Unsafe code SAFETY docs | All 45 unsafe blocks documented with invariants and violation effects |
| Zero-copy anti-patterns | All `cast_slice().to_vec()` verified necessary, documented with rationale |

## Recently Resolved (S74–S75)

| Item | Resolution |
|------|-----------|
| 6 god files >700 lines | primal_integration.rs (1,163L→5 modules), capability_provider.rs (746L→5 modules), primals/lib.rs (580L→7 modules), opencl_impl.rs (831L→6 modules), env_overrides.rs (726L→9 modules), os_layer/compat.rs (766L→7 modules) |
| 3 god files from S74 | workload.rs (829L→2 modules), unified.rs (613L→3 modules), precision/mod.rs (816L→3 modules) |
| Wildcard re-exports | `pub use *` narrowed in 6 high-traffic crates (toadstool, distributed, server, gpu, universal, orchestration) |
| `pollster` dependency | Removed from barracuda, toadstool, universal (→ tokio_block_on) |
| `serde_yaml` dependency | Migrated to `serde_yaml_ng` across workspace |
| `async-trait` dependency | Migrated to native AFIT in 4 crates (performance, analytics, wasm, gpu) |
| Dead evolved backends | 3 modules gated behind `#[cfg(test)]` in biomeos_integration |
| Hardcoded primal names | Evolved to capability-based language in CLI/JSON-RPC/errors + type aliases |
| Edge platform stubs | Raspberry Pi, industrial, microcontroller → genuine hardware probing |
| Discovery stubs | mDNS, Kubernetes, Docker Compose, Registry → real capability-probing |
| GPU test resilience (NVK) | 11 barracuda + 29 ml-inference + homomorphic tests wrapped with catch_unwind |
| WgpuDevice::poll_safe() | Device-lost recovery via catch_unwind on poll paths |
| TYPES_REFERENCE.md | Updated with Module Structure Reference (Section 7) |

## Recently Resolved (S70–S75)

| Item | Resolution |
|------|-----------|
| `primal_integration.rs` god file (1,163L) | Smart-refactored into 5 domain modules (capabilities, socket, discovery, tests) |
| `capability_provider.rs` god file (746L) | Smart-refactored into 5 domain modules (error, serialize, discovery, provider) |
| `primals/lib.rs` god file (580L) | Smart-refactored into 7 domain modules (types, service, health, messaging, manifest, manager) |
| `opencl_impl.rs` god file (831L) | Smart-refactored into 6 domain modules (backend, resource, context, kernels, tests) |
| `env_overrides.rs` god file (726L) | Smart-refactored into 9 domain modules (parse, app, network, resources, features, runtime, security, logging, tests) |
| `os_layer/compat.rs` god file (766L) | Smart-refactored into 7 domain modules (trait_def, linux, windows, macos, legacy, tests) |
| Wildcard `pub use *` re-exports | Narrowed to explicit re-exports in 6 high-traffic crates |
| `pollster` dependency | Removed from barracuda, toadstool, universal — replaced with tokio-native |
| `serde_yaml` dependency | Replaced with maintained `serde_yaml_ng` across workspace |
| `async-trait` in 4 crates | Migrated to native AFIT (performance, analytics, wasm, gpu) |
| Evolved backends dead code | Gated behind `#[cfg(test)]` (agent, auth, storage backends) |
| Hardcoded primal names in CLI/UI | Capability-based language: "PKI security service", "Orchestration service", "Storage capability" |
| `AuthResponse` stub | Formalized `AuthResponse::standalone()` with `is_standalone()` |
| Edge platform stubs | Genuine hardware probing (Raspberry Pi, industrial, microcontroller) |
| Discovery stubs | Real mDNS/k8s/docker/registry capability probing |
| GPU test resilience | 40+ test files wrapped with `catch_unwind` for NVK driver panics |
| `WgpuDevice::poll()` panics | `poll_safe()` catches panics, sets device lost, returns `Err` |
| Doctest compilation failures | Fixed across barracuda ops and ml-inference showcase |
| `workload.rs` god file (829L) | Smart-refactored into 2 domain modules (types extracted) |
| `unified.rs` god file (613L) | Smart-refactored into 3 domain modules (device_types, routing, capabilities extended) |
| `precision/mod.rs` god file (816L) | Smart-refactored into 3 domain modules (compiler, polyfill) |

### Resolved (S70–S71)

| Item | Resolution |
|------|-----------|
| 4 orphaned shader constants | HMM_FORWARD_LOG_F32/F64, BOOTSTRAP_MEAN_F64, HISTOGRAM — all wired to GPU dispatch |
| 3 CPU-only primitives → GPU | kimura_fixation, jackknife_mean, hargreaves_batch — GPU shaders + Rust dispatch |
| Hardcoded primal strings | 6 production files evolved to `primals::*` constants |
| jsonrpc_server.rs >900 lines | Refactored 904→628 via shared test helper |
| network_config/types.rs >800 lines | Split 859→7 domain submodules |
| builder.rs >1000 risk | Smart-refactored 975→mod.rs (129) + profiler.rs (531) + substrate.rs (338) |
| EcosystemCaller dead code | Deleted entirely (deprecated since 2.0.0, zero references) |
| Monitoring stub collectors | Evolved to real `sysinfo` (health thresholds, real metrics, session tracking) |
| NestGate connect placeholder | Evolved to `primal_sockets::get_socket_path_for_service()` |
| Sovereignty: port 8084 | `toadstool_config::ports::daemon_port()` — configurable, zero hardcoded |
| Sovereignty: songbird discovery | `"mdns"` capability-based default (was hardcoded `"songbird"`) |
| Sovereignty: adapter string-matching | Universal `SongbirdAdapter` for all JSON-RPC endpoints (capability-based) |
| Monitoring >1000 lines | Split `lib.rs` 1071→679 lines + process.rs + thresholds.rs + platform.rs |
| UniversalAdapter stub | Evolved to validate runtime hints, check adapter state, inject default timeout |
| 7 new WGSL shaders | gelu_df64, sigmoid_df64, softmax_df64, layer_norm_df64, sdpa_df64, brent_f64, seasonal_pipeline |
| 4 batched_elementwise ops | SensorCalibration, HargreavesEt0, KcClimateAdjust, DualKcKe (from airSpring) |
| SymmetrizeGpu/LaplacianGpu | Previously unwired shaders → proper GPU pipeline executors |
| 3 stats modules | evolution (kimura_fixation), jackknife (leave-one-out), fao56_et0, chao1_classic |
| SimpleMLP | CPU MLP with JSON weight serde + forward inference |
| Fp64Strategy::Concurrent | Dual-validation variant for running DF64 + native f64 side-by-side |
| NVK max_buffer_size | `sanitize_max_buffer_size` caps absurd values to architectural limits |
| preferred_workgroup_size | Architecture-aware 1D sizes (Volta 64, Ampere 256, RDNA 256) |
| matmul_ref | Non-consuming matmul for recurrent architectures |
| 15 production stubs | Primals client (real JSON-RPC), orchestrator deploy, coordinator cancel (CancellationToken), deprecated HTTP caller (returns error), edge platforms (proper errors) |
| Test concurrency | All tests concurrent, zero `#[serial]`, zero fixed sleeps in non-chaos tests |
| Environment safety | All `std::env::set_var` in tests → `temp_env` (8 files migrated) |
| Test timeouts | Reduced defaults: 30s→5s, 120s→30s, 60s→20s, unit 5s→2s |
| All doctests | Fixed across common, core, display, testing crates |
| ChaosEngine metrics | `recovery_count` synced between SystemState and ChaosMetrics |
| Error codes | `WORKLOAD_NOT_FOUND` for job queue (was METHOD_NOT_FOUND) |
| Storage benchmark | Race condition fixed (unique nanos-based temp files) |
| Nested runtime | MockTask drop panic eliminated (AtomicUsize replaces RwLock) |
| +187 new tests | lifecycle, dispatch, jsonrpc, monitoring, nestgate, display IPC, daemon, config, barracuda stats/ops |
| Real mDNS parser | Replaced placeholder `Ok(None)` in zero_config service discovery |
| Barracuda unused_async | Crate-level `#![allow(clippy::unused_async)]` with documented justification |

## Previously Resolved (S69++)

| Item | Resolution |
|------|-----------|
| metalForge streaming pipeline | `PipelineBuilder` → `StreamingPipeline` (staging/pipeline.rs) |
| manual_jsonrpc → pure_jsonrpc | Full migration — all handlers, Unix/TCP, unibin migrated |
| 4 production stubs | biome.rs (real validation), container benchmark (runtime detection), gRPC (deprecated), OpenCL (capability-based) |
| 16 large files | Smart-refactored to domain modules (all < 1000 lines) |
| 66 ComputeDispatch ops | 5 linalg + 15 special + 14 MD/bio + 7 reduce + 6 attention + 5 tensor + 3 index + 4 FFT + 7 misc (~9,000+ lines removed) |
| NAK workgroup tuning | `workgroup_size_for_arch()` — Volta 64, Ada 256, RDNA 64, Intel Arc 128 |
| Hardcoded IPs | 6 production files → named constants |
| anyhow elimination | Fully eliminated from all ~30 workspace crates |
| rust-version 1.75→1.80 | `std::sync::LazyLock` stable |
| Dead code documented | All 18 unjustified `#[allow(dead_code)]` instances annotated |
| +100 new tests | naga validation, untested modules, staging, pure_jsonrpc, distributed, monitoring |
| Unsafe evolution | GPU memory bounds checks, SAFETY docs, `alloc_and_lock()` helper |
| chrono elimination | 28 crates, 200+ files → `std::time` |
| Unsafe 47→45 | `BorrowedFd` → safe `AsFd` in akida-driver |

## Previously Resolved

Full session-by-session resolution history is in [CHANGELOG.md](CHANGELOG.md).

Key milestones:
- **S68**: Dual-layer universal precision (`op_preamble` + `df64_rewrite`), 122 shader tests
- **S66**: Cross-spring absorption wave (airSpring V009 + groundSpring V7), 707 shaders classified
- **S61**: Sovereign Compiler Phase 4 (naga-IR FMA fusion, DCE, SPIR-V passthrough)
- **S60**: DF64 FMA optimization (`two_prod` Dekker→`fma`), DF64 transcendentals, 4 force shaders all-DF64
- **S50**: Coverage push 73%→84%, hardcoded ports/URLs eliminated, mock evolution, cargo-deny
- **S25**: GPU FFT f64 validation, error system deep debt
- **S21**: wetSpring bio GPU primitives (Smith-Waterman, Gillespie SSA, decision tree, Felsenstein)
- **S14-20**: neuralSpring 11-shortcoming absorption, TensorSession ML ops, chrono/futures/dashmap eliminated
- **S5-13**: Coverage sprints, sleep elimination, sovereign compiler phases 1-3

---

*Debt is tracked, not ignored. Each workaround has an evolution path.*
*The goal is zero workarounds — vendor-agnostic, capability-based code.*
