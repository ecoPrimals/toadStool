# ToadStool -- Next Steps

**Updated**: April 2026 — S213 (Deep Debt — Lint Reason Sweep + Capability Names + Orchestrator Resilience)
**Status**: Production-grade | Rust edition **2024** (MSRV 1.85) | **AGPL-3.0-or-later** | **All quality gates green** | tests verified (21,600+ workspace, 0 failures) | **~65 JSON-RPC methods** | Wire Standard L3 (partial) | Zero C FFI deps (ecoBin v3.0) | **Zero production panics/expects** | IPC-first | workspace `unsafe_code = "deny"`, **41 crates `forbid`** | **49 unsafe blocks** (all in hw containment, all SAFETY-documented) | **0 production TODOs** | **rustix 1.x workspace-wide** | **capability-based primal references (no hardcoded names)** | **`async-trait` DEPRECATED** (banned in `deny.toml`) | **`deny.toml` ring + async-trait + zstd-sys bans active** | **BTSP handshake bounded** (5s default, PG-46, S210) | **All lint attrs with reason (S211+S213)** | **Auth issuer capability-based (S209)** | **Self-registration with Songbird (S207)** | **Encrypted compute dispatch (Phase 55)** | **Display Phase 2 (petalTongue IPC)** | **BTSP JSON-line relay (Phase 45c)** | **Orchestrator lock-panic-free (S213)**
**Latest**: S213 — Deep debt: all bare #[allow]/#[expect] now have `reason=`; GPU stubs evolved from hardcoded primal names to `gpu.dispatch.cuda` capability URIs; WorkloadOrchestrator evolved from expect("lock poisoned") panics to Result-based error returns. 0 failures, clippy clean, fmt clean.

---

## Active Work

### ~~P0: ComputeDispatch Migration~~ → Transferred to barraCuda (S93)

**Transferred.** ComputeDispatch lives in the barraCuda crate. 144/280+ ops migrated;
~139 remaining. barraCuda team owns this incremental migration.

### ~~P1: DF64 Default Path~~ → Transferred to barraCuda (S93)

**Transferred.** barraCuda owns precision strategy (f64/df64/f32 validation, shader
selection, `df64_rewrite` as default). toadStool serves hardware capabilities.
Handoff: `wateringHole/handoffs/TOADSTOOL_S93_DF64_HANDOFF_MAR03_2026.md`.

### ~~P1: NpuDispatch Trait~~ ✅ RESOLVED (S94b)

`toadstool-core::npu_dispatch` — generic `NpuDispatch` trait + `AkidaNpuDispatch`
adapter. Vendor-agnostic, capability-based, zero-copy input (`Cow`). Also added
`NpuParameterController` trait (hotSpring absorption) for NPU-driven autonomous
parameter tuning.

### ~~P1: Fix `set_var`/`remove_var` unsafe blocks~~ ✅ RESOLVED (S157b)

All `set_var`/`remove_var` calls wrapped in `unsafe {}` across 14 files. Mangled
syntax fixed in 3 server files. Test suite fully unblocked.

### P1: Test Coverage → 90% (D-COV) — Ongoing (S164)

**~83.6% line coverage** (lib-only, 185K lines instrumented). **22,000+ tests** (0 failures). Target 90%.

**S164** expanded coverage with **+94 new tests** across 7 low-coverage files:
- `resource_validator.rs` 20% → ~75% (+19 tests)
- `primal_integration/discovery.rs` 57% → 88% (+21 tests)
- `universal/scheduler/execution.rs` 45% → 99% (+25 tests)
- `cloud/orchestrator/mod.rs` 43% → 100% (+6 tests)
- `auto_config/ecosystem.rs` 68% → ~85% (+17 tests)
- `client/core.rs` 54% → ~85% (+18 tests)
- `pure_jsonrpc/handler/dispatch.rs` 40% → ~70% (+13 tests)

**S168** expanded coverage with 11 more 0% files → covered (see DEBT.md D-COV-*-S168).

**Remaining gap**: Largest uncovered areas are hardware-dependent paths (VFIO, DRM, V4L2, akida userspace), neuromorphic drivers, GPU engine/execution paths, CLI discovery modules, and specialty engine.rs (4% coverage). These require integration-level testing with hardware or mock hardware infrastructure.

### ~~P1: Sovereignty Migration (D-SOV)~~ ✅ RESOLVED (S94b)

All 7 production callers of `get_socket_path_for_service` migrated to
`get_socket_path_for_capability()`. CLI filesystem and socket discovery use capability
names directly. Deprecated API definitions retained for backward compatibility only.

---

### All-Silicon Pipeline (S159b-d) ✅ Phase B+C LANDED

| Phase | Status |
|-------|--------|
| **Phase B: Silicon discovery + performance surface** | ✅ COMPLETE — `SiliconUnit` model (9 units), wgpu adapter probe, sysfs PCI device ID tables, `compute.performance_surface.{report,query,list}` JSON-RPC handlers |
| **Phase C: Multi-unit routing engine** | ✅ LANDED — `compute.route.multi_unit` handler, tolerance-based routing, heuristic fallback, shader-core fallback on every decision |
| **Phase D: Mixed command streams** | Planned — blocked on coralReef FECS firmware loading; extends PBDMA with draw/RT/texture/tensor/framebuffer commands |

### Key Remaining Items (S159)

| Item | Status |
|------|--------|
| Coverage push 83%→90% | Ongoing — hardware mocks needed for remaining gaps |
| Phase D mixed command streams | Planned — requires Phase A (VFIO) + coralReef |
| VFIO PBDMA dispatch | Blocked on coralReef (USERD_TARGET encoding fix) |
| E2E sovereign pipeline test | Blocked on VFIO dispatch |
| Phase 2 dep migration: procfs → toadstool-sysmon | **RESOLVED** — `procfs` default features disabled (S129); dead `procfs` dep removed where unused (S160); runtime discovery uses `toadstool-sysmon` where applicable |
| Phase 3: tarpc binary transport | **RESOLVED** S203t — MessagePack binary framing for Rust-to-Rust peers |
| Property-based testing for computation modules | Pending |
| Multi-primal integration test infrastructure | Pending |
| Pipeline dispatch for ordered multi-stage (neuralSpring PG-05) | **RESOLVED** S199 — `compute.dispatch.pipeline.submit` + `.status` |
| Stable compute.dispatch.submit IPC for springs (PG-05) | **RESOLVED** S199 — methods stable, pipeline scheduling added |
| Deep debt audit + service_discovery refactor | **RESOLVED** S200 — 0 production unwraps/mocks/hardcoded names; fallback.rs extraction |
| rustix version unification | **RESOLVED** S200/S203 — rustix 1.x workspace-wide (display migrated S203) |

### Transferred to Other Teams

| Item | Owner | Notes |
|------|-------|-------|
| D-DF64: DF64 as default precision | barraCuda team | S93: precision strategy is barraCuda's domain |
| DF64 transcendental coverage | barraCuda team | COMPLETE (S71): 15 functions |
| Architecture-specific polynomial selection | barraCuda team | Per-silicon Horner vs Estrin |
| Sovereign compiler Phase 4+ | barraCuda team | naga-IR optimizer, register pressure, peepholes |
| barraCuda budding Phases 1-4 | barraCuda team | API audit, SemVer 1.0, Springs rewire |
| ComputeDispatch migration (D-CD) | barraCuda team | 144/280+ done; ~139 remaining; lives in barraCuda crate |

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
- [x] **pollster eliminated** -- removed from barracuda, toadstool, universal (→ tokio_block_on)
- [x] **serde_yaml → serde_yaml_ng** -- across workspace
- [x] **async-trait → DEPRECATED** -- fully removed and banned in `deny.toml` (S203t); all annotations evolved to manual `Pin<Box<dyn Future>>` or native AFIT, and subsequently to enum dispatch + RPITIT (S203s)
- [x] **Capability-based naming** -- CLI/JSON-RPC/error messages use capability language, type aliases added
- [x] **GPU test resilience** -- NVK catch_unwind wrappers on 11+29+homomorphic test files
- [x] **Wildcard re-exports narrowed** -- 13 crates (toadstool, distributed, server, gpu, universal, orchestration, sandbox, wasm, edge discovery/toolchain/comms/deployment)
- [x] **9 god files refactored (S74+S75)** -- primal_integration, capability_provider, primals/lib, opencl_impl, env_overrides, os_layer/compat, workload, unified, precision/mod
- [x] **ComputeDispatch migration** -- transferred to barraCuda team (lives in barraCuda crate)
- [x] **DF64 default path** -- transferred to barraCuda team (S93)
- [x] **NpuDispatch trait** -- generic NPU interface (toadStool D-NPU)
- [x] **Clippy pedantic clean** -- `cargo clippy --workspace --all-targets -- -D warnings -W clippy::pedantic` zero warnings (S130+)
- [x] **`#[expect]` evolution** -- production `#[allow]` evolved to `#[expect(lint, reason)]` where the lint fires; ~80 justified `#[allow]` remain (S198); S131+ removed stale suppressions
- [x] **Spring sync S131+** -- all 5 springs pinned to latest, SPRING_ABSORPTION_TRACKER updated (S131+)
- [ ] **Test coverage target 90%** -- 22,000+ tests; ~83.6% line; mock hardware layers for V4L2/VFIO (MockV4l2Device, MockVfioDevice); push to 90% ongoing
- [x] **C dep elimination** -- flate2 → rust_backend, procfs default features disabled (S129)
- [x] **Capability-based ports** -- `resolve_capability_or_legacy_port()` with graceful legacy fallback (S129)
- [x] **God file splits (round 4)** -- ipc/server.rs, container/lib.rs, ecosystem.rs, handler/mod.rs, nestgate/client.rs (S129)
- [x] **Zero-copy hot paths** -- `Cow<'static, str>`, `Arc<str>` in execution types (S129)
- [x] **Generated artifacts cleaned** -- removed tracked JSON files from git (S129)
- [x] **Primal overstep cleanup (S169)** -- Ollama, HTTP server stack (server+cli → Songbird), shader **compile** proxy (→ coralReef), science/ecology/deploy relay (→ biomeOS); pyo3/gbm/linfa/hmac/indicatif removed; **`shader.dispatch`** retained
- [x] **coralReef / shader compiler discovery (TS-01, S198)** -- `visualization_client.rs` uses `capability.discover` (no `CORALREEF_*` env, no coralreef-core.json, no coralreef dir scan). S169 removed compile from toadStool (coralReef-only); dispatch E2E via **`shader.dispatch`**
- [x] **Cross-spring provenance** -- `cross_spring_provenance.rs`, `toadstool.provenance` JSON-RPC method (S130)
- [x] **Sovereignty migration** -- remaining callers to capability-based APIs (toadStool D-SOV)
- [x] **Hardware Transport wiring** -- transport.discover/list/route JSON-RPC + CLI commands
- [x] **Detection stubs evolved** -- 11 functions → real /proc + command-based detection
- [x] **Smart refactoring (round 2)** -- security.rs (771→5 modules), config_utils (777→5 modules)
- [x] **Smart refactoring** -- vfio.rs (971L) smart-refactored into `vfio/` directory (S94)
- [x] **manual_jsonrpc removal** -- deleted, pure_jsonrpc is canonical (S94)
- [x] **Barracuda fossilization** -- dead dep removed, crates/barracuda → archive/ (S94)
- [x] **Sovereign pipeline** -- HardwareFingerprint, is_sovereign_capable, safe_allocation_limit, SubstrateCapabilityKind (S96)
- [x] **SubstrateType expansion** -- 4→8 variants: IntegratedGpu, Npu, Tpu, Fpga, Dsp, Quantum (S96)
- [x] **God file splits (round 3)** -- dispatch.rs, detection.rs, engine.rs, protocols/lib.rs, specialized_templates.rs (S96)
- [x] **BTSP Phase 2 (S198)** -- handshake on all UDS accept paths (tarpc + daemon JSON-RPC servers)
- [x] **Health triad shapes (S198)** -- liveness / readiness / check JSON-RPC responses aligned
- [x] **musl-static release binary (S198)** -- ~11MB x86_64 PIE stripped, validated
- [x] **API orphan resolved** -- crates/api/ ByobApi extracted to container crate (S96)
- [x] **V4L2 unsafe docs** -- All SAFETY comments on unsafe blocks (S96)
- [x] **Debris cleanup** -- root tests/ stubs, stale checklists, false-positive TODOs (S95)
- [x] **management/resources re-added** -- real ResourceManager (S95; sysinfo → `toadstool-sysmon` S137)

### Cross-Repo Debt

- [x] **D-S20-003**: neuralSpring `evolved/` migration — **RESOLVED** (neuralSpring V89 completed; `evolved/` removed)
- [x] **D-S18-002**: cubecl transitive `dirs-sys` — **RESOLVED** (cubecl fully removed; `dirs-sys-next` now only via wasmtime-cache, feature-gated)

---

## Completed This Session (S90-S206)

### Session S206: Lint Evolution + Dep Hygiene + Feature Cleanup (Apr 28, 2026)
- **Lint evolution** — All ~40 production bare `#[allow(...)]` evolved to `#[allow(..., reason = "...")]`: 17 `unsafe_code` module allows in hw-safe/gpu/display/plugin crates, plus ~23 clippy/deprecated/async_fn_in_trait allows across auto_config, cli, distributed, integration, management, neuromorphic, runtime, security crates.
- **Dependency unification** — `humantime-serde`, `rand`, `tokio-util`, `temp-env` added to `[workspace.dependencies]` and 20+ crate Cargo.toml files updated to `{ workspace = true }`.
- **Stale feature removal** — GPU crate: `spirv`/`jit`/`testing` features and optional deps (`spirv`, `cranelift-jit`, `wasmtime`) removed (never referenced in source). Testing crate: `integration-tests`/`benchmarks` features and `wiremock` dep removed.
- **`test-mocks` off by default** — removed from `toadstool` core `default` features; production builds no longer compile `InMemoryAuthBackend`/`InMemoryAgentBackend`. Testing crate explicitly enables via `features = ["test-mocks"]`.
- 7,841 lib tests, 0 failures, clippy clean, fmt clean.

### Session S205: Phase 55 — Encrypted Compute Dispatch + Discovery Socket (Apr 28, 2026)
- **Encrypted compute dispatch** — `DispatchHandler` now optionally holds a Tower `SecurityClient`; when present (NUCLEUS composition), payloads are encrypted via `crypto.encrypt` with the `compute` purpose key before dispatch to coralReef, and results are decrypted via `crypto.decrypt` on return. Standalone mode (no BearDog) continues with plaintext dispatch.
- **`DISCOVERY_SOCKET` wired** — new env var as highest-precedence tier for coordination/discovery capability resolution in `resolve_capability_socket_fallback()`. `SocketPathEnv` updated, `query_providers()` now resolves via `"discovery"` capability.
- **Purpose key retrieval** — `SecurityClient::retrieve_purpose_key()` calls `secrets.retrieve("nucleus:{family}:purpose:{purpose}")` on BearDog; key cached lazily on first dispatch.
- **`base64` dependency** added to `toadstool-distributed` (workspace unified).
- 9 new tests: 5 discovery socket precedence, 2 purpose key retrieval, 2 encrypted dispatch path.
- 7,841 lib tests, 0 failures, clippy clean, fmt clean.

### Session S204: Deep Debt Evolution — Safety Docs, Constants, Dep Hygiene, Mock Isolation (Apr 26, 2026)
- `ffi_loader.rs` SAFETY docs (13 blocks — last file without them; all 49 unsafe blocks now documented)
- Hardcoded `toadstool-main`/`toadstool-primary` → `INSTANCE_ID`/`PRIMAL_NAME` constants; mDNS duplicate → `TOADSTOOL_SERVICE_TYPE`
- `serde_yaml_ng` workspace-unified (5 crates); unused `humantime-serde` removed; `rustix` aligned; stale WASM comment fixed
- `InMemoryAgentBackend` gated to test-only (`#[cfg(any(test, feature = "test-mocks"))]`)
- Bare `#[allow]` → `#[allow(reason)]` in 10 production sites
- `deny.toml`: stale `BSD-3-Clause-Clear` removed; `zstd-sys` ban activated; `ring` clarify documented
- 7,832 lib tests, 0 failures, clippy clean, fmt clean

### Session S175: Deep Debt Evolution — Crypto Errors + eprintln→tracing + Lint Attr Evolution (Apr 21, 2026)
- **`NoopCryptoProvider` capability-based errors** — all 5 error-returning methods now use `NOOP_MSG` constant with capability-based guidance (`"no crypto provider registered; register a provider via crypto.provider.register capability"`). Matches `NoopCloudProvider` pattern from S174.
- **`eprintln!` → `tracing`** — 6 `eprintln!` calls in `universal/capabilities.rs` migrated to structured `tracing::warn!`/`tracing::info!` with span fields (`adapter_index`, `adapter_name`, `selector`). Added `tracing` as workspace dep to `toadstool-runtime-universal`.
- **`#[allow]` → `#[expect]` evolution** — 13 bare `#[allow]` evolved to `#[expect]` with reasons across: distributed (`gpu.rs` detection stubs, `federation/mod.rs` re-exports, `network/metrics.rs` field), neuromorphic (`pcie.rs` struct fields), management (`internal.rs` baseline metrics). Preventive `#[allow]` with reasons kept where lints don't fire (nvpmu VFIO/power_manager casts, server handler `unused_async`).
- All quality gates green: `cargo check --workspace` clean, `cargo clippy --workspace -- -D warnings` 0 warnings, 7,818 lib tests pass, armv7 cross-arch clean.

### Session S174: Deep Debt Evolution — Edge Clippy Clean + Server Tests + Lint Evolution (Apr 20, 2026)
- **Edge crate clippy clean** — 231 warnings → 0:
  - Crate-level `#![allow(missing_docs, reason)]` for 211 hardware enum variants (self-documenting by name)
  - 4 RPITIT signature mismatches aligned (`RuntimeEngine` impl methods match `impl Future` trait syntax)
  - `DiscoveryFuture` and `MetricsFuture` type aliases extracted for complex `Pin<Box<dyn Future>>` return types
  - `#[expect(dead_code, reason)]` added to 15 stored-for-lifecycle fields (discovery configs, execution handles, device IDs)
  - 3 unfulfilled `#[expect(dead_code)]` removed from public constructors
  - `MicrocontrollerArch::x86` renamed to `X86` (upper camel case)
  - `Vec::new()` + push → `vec![...]` initializer in discovery service
  - Unused `ToadStoolResult` import removed from `discovery/serial.rs`
  - Unused `Pin`, `Future` imports removed from `lib.rs`
- **Server test compilation fixed** — 2 `create_executor` calls in `unibin_execution_coverage_tests.rs` updated to pass `&UnibinExecutionConfig::from_env()` (new 2-arg signature).
- **Lint attributes evolved**:
  - `ffi_loader.rs`: `#[allow(dead_code)]` → `#[expect(dead_code, reason = "held for drop side-effect")]`
  - `v4l2/types.rs`: `#![allow(missing_docs, dead_code)]` → added `reason` parameter
  - `nvpmu/lib.rs`: `#[allow(unsafe_code)]` on dma/vfio modules → added `reason` parameter
- **4 orphaned `[build-dependencies]`** removed from `Cargo.toml` files (edge, secure_enclave, specialty, python) — all had no `build.rs`.
- **`NoopCloudProvider`** error messages evolved from bare `"noop"` to capability-based guidance (`"no cloud provider registered; register via cloud.provider.register capability"`).
- All quality gates green: `cargo check --workspace` clean, `cargo clippy -p toadstool-runtime-edge -- -D warnings` 0 warnings, `cargo clippy --workspace` clean, 7,818 lib tests pass, 0 failures.

### Session S173: Deep Debt Evolution — Edge Compilation + Smart Refactoring (Apr 19, 2026)
- **Runtime/edge compilation fixed** — 61 errors resolved: error constructor API alignment (`discovery_error`→`runtime`, `execution_error`→`execution`, etc.), `platform_paths` module path correction (`toadstool::`→`toadstool_common::`), `Box<dyn CommunicationProtocol>` Clone workaround (borrow-through-lock pattern), `serialport` `RwLock`→`Mutex` migration (`SerialPort` is `Send` but not `Sync`), `Display` impl for `EdgePlatform`, UUID `v5` feature gate, `Arc<Self>` borrow-escape fix for `start_continuous_discovery`.
- **Smart-refactored 3 large specialty files** into directory modules:
  - `cpu6502.rs` (828L) → `cpu6502/{mod,alu,decode,tests}.rs`
  - `emulator_impls.rs` (717L) → `emulator_impls/{mod,mos6502,z80,tests}.rs`
  - `programmer_impls.rs` (712L) → `programmer_impls/{mod,init,generic,eprom,tests}.rs`
- **Lint evolution**: `#[allow(dead_code)]` → `#[expect(dead_code, reason)]` in `sandbox/proc.rs` and `embedded/programmers.rs` with explicit justifications.
- **Test fix**: `edge_config_tests::test_edge_runtime_config_default` assertion updated for XDG-compliant cache path (was hardcoded `/tmp/toadstool_edge_cache`).
- All quality gates green: 102 edge tests + 16 specialty tests + 98 sandbox tests passing. Clippy clean.

### Session S203s: Stadial Parity Gate Cleared (Apr 16, 2026)
- **Stadial parity gate cleared** — ~32 finite-implementor traits converted from `dyn Trait` dispatch to **enum dispatch + RPITIT**. Zero finite-implementor `dyn` remaining.
- **~864 `Pin<Box<dyn Future>>`** cascaded away when traits moved to RPITIT.
- **`RuntimeEngine` genericized** across 7 runtime crates with `RuntimeEngineDispatch` enum in server.
- **Gate verification**: `cargo deny check bans` PASS, clippy clean, 5,200+ tests verified.
- **Remaining dyn (justified)**: infant discovery plugin registry, PrimalIntegration, MessageHandler, testing utilities.

### Session S203r: async-trait Full Deprecation (Apr 16, 2026)
- **`async-trait` fully deprecated** — all ~91 `#[async_trait]` annotations evolved to manual `Pin<Box<dyn Future>>` (dyn-dispatched) or native AFIT (non-dyn), and subsequently to enum dispatch + RPITIT (S203s), across 55+ files in 13 crates. Zero runtime behavior change.
- **Banned in `deny.toml`** — `async-trait` added to `[bans.deny]` with `wrappers = ["axum", "axum-core", "config", "wiggle"]` for transitive deps.
- **Removed from all Cargo.toml** — workspace dependency + 12 individual crate dependencies eliminated.
- **DEBT.md D-ASYNC-DYN-MARKERS → RESOLVED** S203r.
- **Clippy clean** — `type_complexity` resolved via `BoxFuture` type aliases; `used_underscore_binding` fixed in conditional branches.
- **22,061 tests, 0 failures**, clippy 0 warnings, fmt 0 diffs.

### Session S203q: Root Doc Cleanup + Debris Audit (Apr 16, 2026)
- Root docs (README, CONTEXT, DOCUMENTATION, NEXT_STEPS) aligned to S203p state.
- Stale migration banner removed from `cli/templates/specialized_templates/mod.rs`.
- Config port evolution comment updated. Build artifacts cleaned (25GB).

### Session S203i: Deep Debt — Massive Test Extraction + Hardcoding Evolution (Apr 14, 2026)
- **52 production files** refactored via test extraction (~10K lines moved to companion files). Production files >500L reduced from 38→25 (remaining are pure production code — hardware drivers, type defs; no extractable test blocks, all <700L).
- **Hardcoding evolution**: `CORALREEF_URL`/`CORALREEF_SOCKET` dispatch notes → capability-neutral guidance. `FallbackEndpoints` literal `"localhost"` → `DEFAULT_HOSTNAME` constant.
- All quality gates green. Clippy 0 warnings. 21,700+ tests, 0 failures.

### Session S203h: benchScale — TCP Idle Timeout (Apr 14, 2026)
- **TCP idle timeout**: `TCP_IDLE_TIMEOUT_SECS` (300s default, env configurable). `tokio::time::timeout` wraps on all TCP read loops (JSON-RPC + tarpc). `TCP_NODELAY` on all accepted streams.
- Resolves primalSpring benchScale exp082 (half-open connection held indefinitely).

### Session S203g: Deep Debt — Test Extraction + Deprecated Removal + Idiomatic Evolution (Apr 13, 2026)
- **12 production files >540 LOC** refactored via test extraction (26 total across S203c/e/g).
- **6 deprecated zero-caller items removed**: `localhost_endpoint`, `METRICS_PORT`, `capability_typical_provider` module, `get_primal_default_port` wrappers, `TarpcClient::address()`.
- **Async GPU discovery evolved**: blocking `std::thread::sleep` poll loop → `tokio::sync::oneshot` + `tokio::time::timeout` (async-native, no executor blocking).
- **Forward dispatch clone optimization**: full JSON object clone → empty Map fallback.
- All quality gates green. Clippy 0 warnings. 21,700+ tests, 0 failures.

### Session S203f: wetSpring V143 Validation — Capability Surface (Apr 13, 2026)
- **`compute.execute` promoted** to direct JSON-RPC route (closes wetSpring PG-05 gap).
- **Pipeline methods** (`dispatch.pipeline.submit`, `dispatch.pipeline.status`) added to `capabilities.list`.
- **plasmidBin metadata** expanded from 6 to 46 callable methods, `min_ipc_version = "2.0"`.

### Session S203d/e: LD-04 BTSP Auto-Detect + Network Centralization + File Refactoring (Apr 12, 2026)
- BTSP first-byte auto-detection for plain-text clients (primalSpring). `PrependByte<S>` adapter.
- 8 hardcoded network constants centralized. 5+2 env-dependent tests hardened. 10 large files refactored.

### Session S202: Deep Debt Execution — Capability-Based Evolution (Apr 11, 2026)
- **Hardcoded literal evolution**: 3 production `"toadstool"` literals → `PRIMAL_NAME` constant (self_identity.rs, bear_dog/client.rs, identity.rs). `"coral_reef_available"` JSON-RPC key → `"shader_compiler_available"`.
- **Primal-name doc evolution**: ~15 production doc comments evolved from primal names (BearDog, NestGate, Songbird, Squirrel) to capability-based wording. Serde aliases and legacy mapping tables retained.
- **Dead code removal**: `proxy_to_barracuda` legacy alias removed (dead_code, no callers).
- **Smart refactoring**: `jsonrpc_server.rs` — extracted `dispatch_or_parse_error()` helper, DRY'd 3 duplicated parse-error patterns.
- **Dependency evolution**: `serialport` in `toadstool-runtime-specialty` made optional behind `serial-transport` feature.
- **Unsafe audit**: 34 unsafe blocks confirmed — all in hw containment (mmap, ioctl, volatile MMIO, DMA), all genuinely necessary, all SAFETY-documented. No blocks removable.
- **Mock audit**: Zero production mocks found. All Mock* types gated behind `#[cfg(test)]` or `test-mocks` feature.
- **Large file audit**: Most >500-line files are test-only. `jsonrpc_server.rs` (659) was the main prod candidate (now DRY'd).

### Session S201: primalSpring Gap Closure & Coverage Push (Apr 11, 2026)
- **primalSpring April 11 downstream audit**: Confirmed pipeline scheduling (`compute.dispatch.pipeline.submit`) fully resolved in S199. Stale audit entry closed. D-RUSTIX-DISPLAY-038 confirmed genuinely blocked; D-ASYNC-DYN-MARKERS (now RESOLVED S203s).
- **Coverage push: +46 tests**: Wire L3 structural validation (14), dispatch types Display/serde/equality (12), security hardening submodules: rate_limiter (6), intrusion detection (7), input_validator (13), audit logger (7). All pure-logic, zero hardware deps.
- Verification: `cargo check` + `cargo clippy -D warnings` green, all 46 new tests passing.

### Session S198: TS-01, BTSP Phase 2, Health Triad, OpenCL Deprecation, musl (Apr 9, 2026)
- **TS-01 RESOLVED**: coralReef / shader-compiler discovery in `visualization_client.rs` — unified `capability.discover` (removed `CORALREEF_SOCKET`/`URL`, `coralreef-core.json`, coralreef directory scan).
- **BTSP Phase 2 WIRED**: Handshake enforced on all UDS accept paths (`tarpc_server.rs`, `daemon/jsonrpc_server.rs`; pure JSON-RPC already had it).
- **Health triad**: `health.liveness` → `{"status":"alive"}`; `health.readiness` → `{"status":"ready","version":...}`; `health.check` → full envelope.
- **OpenCL deprecated**: `ocl` removed; OpenCL code paths stubbed; `GpuFramework::OpenCl` deprecated.
- **Refactors**: Six large files → module dirs (handler/core, tarpc_server, interned_strings, ecosystem/types, storage, cloud_provider_trait), all <500 lines.
- **Discovery**: `SocketPathEnv` hints + `resolve_capability_socket_fallback` for primal socket resolution.
- **BearDog**: `auth.token.refresh` — real async RPC (placeholder evolved).
- **Embedded**: `thiserror` platform-specific errors (stubs still placeholder behavior).
- **Unsafe hardening**: nvpmu `VfioIrqSetPayload`, V4L2 fd validation, hw-safe debug asserts, secure_enclave `madvise` checks.
- **musl-static**: ~11MB x86_64 PIE stripped binary built and validated.
- **Workspace**: 228 files changed, net −5,157 lines; 0 clippy warnings, 0 fmt diffs, 0 test failures; **21,700+** tests.

### Session S194: Deep Debt — Capability-Based Field/Type/Doc Evolution (Apr 8, 2026)
- **S194 (Apr 8, 2026)**: Renamed `nestgate_integration` → `storage_integration` (with `#[serde(alias)]`), `NestGateMount` → `StorageMount` in production return types. Updated doc comments across tarpc_client, CLI banner, auth types, storage types, orchestration discovery, visualization client. Renamed primal-named test functions to capability-based. Updated test data. ~400 intentional legacy-compat refs remain. 21,526+ tests, 0 failures.

### Session S193: Headless GPU Architecture + Deep Debt Cleanup (Apr 8, 2026)
- **S193 (Apr 8, 2026)**: Headless GPU crash isolation — `discover_gpus_via_wgpu()` runs in `std::thread::spawn` with `catch_unwind` and 5s timeout. `select_backends()` restricts to Vulkan when `TOADSTOOL_HEADLESS=1`. `gpu_guards::is_headless()` for test gating. BTSP field renames: `beardog_required` → `security_required`, `nestgate_integration` → `storage_integration` in `BiomeSecurity`. Cross-primal doc cleanup continues.

### Session S192: GAP-MATRIX-12 — BTSP Insecure Guard (Apr 8, 2026)
- **S192 (Apr 8, 2026)**: `validate_insecure_guard()` at server startup refuses when both `FAMILY_ID` + `BIOMEOS_INSECURE=1` are set. `is_btsp_required()` returns true when `FAMILY_ID` is set. BTSP client awareness logging at startup. +11 tests (9 unit, 2 integration).

### Session S191: Wire Standard L3 + Deep Debt Audit (Apr 8, 2026)
- **S191 (Apr 8, 2026)**: Wire Standard L3 `cost_estimates` (55+ methods, energy/time/compute model) and `operation_dependencies` (20+ chains) added to `capabilities.list`. Last 4 user-visible primal names removed. Stale root `biome.yaml` deleted. Fresh audit: 0 production TODOs, 0 user-facing primal names, all unsafe in containment, all mocks gated. 21,514 tests, 0 failures.

### Session S190: Wire Standard L2 Compliance (Apr 8, 2026)
- **S190 (Apr 8, 2026)**: `health.liveness` → `"status": "alive"`, `capabilities.list` → wire envelope, `identity.get` → `domain` + `license`. Separated `compute.capabilities` from `capabilities.list`.

### Session S189: GAP-MATRIX-05 + Debris (Apr 7, 2026)
- **S189 (Apr 7, 2026)**: Server mode docs rewritten (67 methods, 11 namespaces). Deleted stale `biome-production.yaml`. Un-ignored sys-crate test. Fixed broken doc links.

### Session S188: Cross-Primal Doc Cleanup (Apr 5, 2026)
- **S188 (Apr 5, 2026)**: Cross-primal doc cleanup — capability-based language in 61 files across all crates. Replaced primal names (Songbird, BearDog, NestGate, Squirrel, CoralReef, BarraCuda) with capability-based language in doc comments, error messages, test assertions. Cross-primal refs 550→425 (remaining are all backward-compat: serde aliases, env var fallbacks, interned string constants, capability mapping tables). Full audit confirmed: unsafe at FFI boundaries only (15 blocks), thread::sleep at hardware boundaries only (8 sites), production mocks fully gated, all production files <700L. All quality gates green: fmt, clippy, doc 0 warnings, 21,512 tests pass.

### Session S187: Deep Debt Execution (Apr 5, 2026)
- **S187 (Apr 5, 2026)**: Deep debt execution — production mocks isolated behind `#[cfg(any(test, feature = "test-mocks"))]` in server/distributed/integration. 56 test `block_on` → `#[tokio::test]`. Cross-primal name evolution: `SongbirdProtocol` → `CoordinationTransport`, `BearDogSecurityProvider` → `DistributedSecurityProvider`, `NestGateResult` → `StorageServiceResult` + dozens more. 5,104 → 550 cross-primal refs in production (89% reduction; remainder intentional legacy compat). Test runtime 9m → 2m30s via removed RUST_TEST_THREADS throttle, cfg!(test) mDNS/TCP timeouts, ServiceDiscovery cache-aware refresh, nvpmu poll-loop, watchdog Condvar, server exponential backoff. External dep audit: all *-sys deps transitive, rustix already adopted, 0 C deps in workspace.

### Session S155b: Coverage Expansion + Quality Gate Evolution (Mar 15, 2026)
- **Tests**: 20,843 (was 20,285). Clippy pedantic clean. Dependency audit clean. Unsafe audit clean.
- **Coverage**: ~83% line (182K lines instrumented). Target 90%.
- **Next steps identified**: Push coverage 83%→90% (hardware mocks needed); property-based testing for computation modules; multi-primal integration test infrastructure.

### Session S154: Deep Audit + Quality Gate Evolution (Mar 14, 2026)
- **Tests**: 20,285 (was 20,262), 222 ignored. 49 new targeted tests (templates, network_config, hardware, mdns_discovery).
- **Coverage**: 83.09% line (target 90%). Clippy pedantic clean. Fmt 0 diffs. Doc warnings 0.
- **Refactoring**: hw_learn.rs (985→9 modules), wgpu_backend.rs (974→4 modules). All files under 1000 lines (largest: 451).
- **Examples**: 5 examples evolved to capability-based discovery.
- **Specs**: PRIMAL_CAPABILITY_SYSTEM.md (REST→JSON-RPC 2.0).
- **Unsafe**: 20 crates upgraded deny→forbid. SAFETY comments added to akida-driver + runtime/gpu.
- **Mocks**: v4l2/vfio unwrap→expect with # Panics docs. Display: V4L2 struct initializers modernized.

### Session S144: Last Mile Deep Debt (Mar 10, 2026)
- **PCIe switch topology**: `pcie_topology.rs` — `PciBridge`, `GpuPairTopology`, `PcieTopologyGraph` for sysfs-based bridge chain discovery, shared switch detection, contention-aware bandwidth estimation. Integrated into `PcieLink` (with `via_switch`, `hops`, `contention_factor`) and `WorkloadRouter` (topology-aware `route_multi_gpu`).
- **Deprecated API migration (20+ files)**: `primals::TOADSTOOL` → `primal_identity::PRIMAL_NAME`. `primals::BEARDOG` → `capabilities::CRYPTO`. `primals::SONGBIRD` → `capabilities::COORDINATION`. `primals::NESTGATE` → `capabilities::STORAGE`. `EnvironmentConfig` deprecated fields → direct env vars. All `#[allow(deprecated)]` removed.
- **Dead code audit (47 instances)**: All `#[allow(dead_code)]` upgraded to `#[allow(dead_code, reason = "...")]` with explicit justification.
- **Ignored test evolution**: `slow-tests` feature flag across `auto_config`, `cli`, `testing`. `gpu_guards` module for safe wgpu test skipping on NVIDIA proprietary drivers.
- **coralReef multi-device compile**: `MultiDeviceCompileRequest`, `DeviceTarget`, `MultiDeviceCompileResponse`. `compile_wgsl` with `target_device`. `compile_wgsl_multi`. `shader.compile.wgsl.multi` JSON-RPC endpoint.
- All quality gates green: 0 fmt, 0 clippy, all tests pass.

### Session S143: Cross-Spring Absorption (Mar 10, 2026)
- **SPIR-V codegen safety** (`spirv_codegen_safety.rs`, renamed from `nvvm_safety.rs` S147): Absorbed from hotSpring v0.6.25. Root cause: naga SPIR-V codegen. `NvvmPoisoningRisk`/`SpirvCodegenRisk`, `PrecisionTier`, `HardwareCalibration`, `PrecisionBrain`.
- **Workload routing** (`workload_routing.rs`): Cross-spring Kokkos parity thresholds. `WorkloadRouter` with 10 patterns.
- **Brain interrupt pattern** (`workload_health.rs`): `AttentionState`, `WorkloadAnomaly`, `InterruptAction`, `WorkloadHealthMonitor`.
- **Deep debt**: Removed hardcoded primal names, memory constants, paths, UID. Added `SubstrateCapabilities::memory_capacity_bytes` / `memory_bandwidth_bps`.

### Session S142: Hardware-First Evolution (Mar 10, 2026)
- **Hardware test infrastructure**: `scripts/run-hardware-tests.sh`, `.github/workflows/hardware.yml`.
- **GPU sysmon telemetry**: `discover_gpus()`, `GpuTelemetry`, `PcieTopology`.
- **PCIe P2P transport**: `PcieTransport` for GPU-to-GPU paths.
- **Streaming JSON-RPC**: `transport.open`, `transport.stream`, `transport.status`.
- **Multi-tenant orchestrator**: `ResourceOrchestrator`, `DeploymentModel`, `TenantQuota`.
- **Mock hardware backends**: `MockGpuAdapter`, `MockNpuBackend`, `MockHardwareFleet`.

### Session S133 (Mar 8, 2026)
- **Ada Lovelace reclassification**: GPU adapter classification updated for Ada architecture.
- **f64_zeros_risk**: f64 shared-memory zeros risk tracking and mitigation.
- **fused_ops_healthy()**: Fused operations health check added.
- **14 ecology.* methods**: New ecology domain JSON-RPC methods for ecosystem integration.
- **NUCLEUS discovery**: NUCLEUS capability discovery and routing.
- **deploy graph routing**: Deploy graph routing and workload placement.
- **20 semantic methods**: Semantic method registry expanded 71→91.
- **Spring versions**: hotSpring v0.6.23, groundSpring V99, neuralSpring V90/S132, wetSpring V99, airSpring v0.7.5.
- **Coverage**: ~86% line (~150K production lines), 20,262 tests, 0 failures.

### Session S131+: Spring Sync + Deep Debt Evolution (Mar 7, 2026)
- **Spring pin update**: All 5 springs updated — groundSpring V95→V96, neuralSpring V87→V89, wetSpring V97d→V97e, airSpring V071→V0.7.3.
- **`#[allow]` → `#[expect]` evolution**: Production lint suppressions evolved to `#[expect(lint, reason)]`. 3 stale suppressions discovered and removed (`cast_sign_loss` that didn't fire, `cast_possible_truncation` on lossless `char as u32`, `dead_code` on used field). `dead_code` on struct fields kept as `#[allow]` (fires in lib but not lib test).
- **Absorption tracker comprehensive update**: New P3 items (coralReef E2E milestone, Fp64Strategy regression tracking), SCS-CN/Stewart/Blaney-Criddle marked DONE, historical items consolidated.
- **Deep debt scan**: All files <1000L, all unsafe=hardware FFI, no production hardcoding, mocks test-isolated, C deps optional.
- **IPC namespace resolution**: toadStool is canonical proxy for `science.*`; springs may also call barraCuda directly.
- **Coverage**: ~86% line (~150K production lines), 20,262 tests, 0 failures.

### Session S130+: Deep Debt Execution (Mar 7, 2026)
- **Unsafe audit**: All ~70+ blocks justified (V4L2/VFIO/GPU FFI, aligned alloc, secure enclave). No safe alternatives.
- **Dependency audit**: **Zero always-on C/FFI deps** (sysinfo eliminated S137). `aes-gcm` optional behind `dev-crypto` feature. All others optional/feature-gated. Already evolved to pure Rust.
- **Hardcoding→constants**: Production primal names in `integrator_impl.rs` evolved to `well_known::*` constants.
- **#[allow] audit**: All 9 justified, 6 comments added, 2 `unused_self` documented.
- **Clone audit**: 14 hot-path patterns documented (tarpc_server, unibin/capabilities, cross_gate → Arc evolution tracked).
- **CI pedantic gate**: Added `clippy::pedantic` to CI workflow.
- **Coverage**: 83.28% → 83.89%, 240 new tests, 19,777 total.
- **Flaky test fix**: `test_recovery_under_chaos` stabilized.

### Session S130+: Clippy Pedantic Clean (Mar 7, 2026)
- **Clippy pedantic clean**: Full workspace `cargo clippy --workspace --all-targets -- -D warnings -W clippy::pedantic` passes with zero errors, zero warnings. 12 iterative passes of auto-fix + manual corrections across 1,868 .rs files (~30 pedantic lint categories resolved).
- **Test count**: 19,536 tests passing, 0 failures.
- **Corrupted test attributes**: 3 CLI test files with sed-corrupted `#[tokio::test]` attributes repaired.

### Session S130: Cross-Spring Shader Rewiring + Provenance (Mar 7, 2026)
- **coralReef proxy**: `shader.compile.*` stubs evolved to real coralReef proxy handlers with capability-based discovery. Later: compile removed from toadStool (S169); **TS-01 / S198** — `visualization_client.rs` uses unified `capability.discover` only (legacy `CORALREEF_*` env, manifest, dir scan removed). Dynamic `coral_reef_available` in capabilities. Graceful naga-only fallback when compiler unavailable.
- **Cross-spring provenance**: `cross_spring_provenance.rs` with 17+ documented flows across all 5 springs. `CrossSpringFlow` struct, `cross_spring_matrix()`, `provenance_json()`. New `toadstool.provenance` JSON-RPC method for ecosystem introspection.
- **Capability port**: `SHADER_COMPILER` added to `capability_fallback` module (port 8090).
- **Tests**: 31 new tests — 12 shader proxy tests, 6 benchmark validation tests, 13 provenance integration tests. Cross-spring WGSL samples from all 5 domains validated.
- **Documentation**: Cross-spring evolution narrative handoff doc at wateringHole.

### Session S129: Deep Debt Execution + Coverage + C Dep Evolution (Mar 7, 2026)
- **C dependency evolution**: `flate2` switched to `rust_backend` (eliminates miniz-sys C dep); `procfs` disabled default features; corrected "Pure Rust" comments on `sysinfo`/`drm`/`evdev`.
- **Capability-based port resolution**: New `capability_fallback` module with COORDINATION/SECURITY/STORAGE/PLATFORM/ECOSYSTEM ports. `resolve_capability_or_legacy_port()` for graceful migration. All `env_config/network.rs`, `config_utils/network.rs`, `primal_discovery_complete` updated.
- **God file refactoring (round 4)**: `ipc/server.rs` 987→428L (extracted platform.rs, dispatch.rs), `container/lib.rs` 981→582L (extracted docker.rs, engine.rs, types.rs), `ecosystem.rs` 963→556L (extracted ecosystem_types.rs, ecosystem_network.rs), `handler/mod.rs` 832→610L (extracted science.rs, core.rs), `nestgate/client.rs` 824→555L (extracted artifacts.rs, pipelines.rs, utils.rs).
- **BYOB API state ownership**: `ByobApi::router(self)` vs `ByobApi::routes()` split — clean state management.
- **Zero-copy hot paths**: `ExecutionStatus::Failed.error` → `Cow<'static, str>`, `RuntimeType::Custom` → `Arc<str>`, const assertions for frame protocol.
- **Coverage expansion**: 200+ new tests across 5 batches. 19,109 tests passing, 0 failures, 203 intentional GPU hardware ignores.
- **Long-running test debt**: `dispatch_coverage_tests.rs` completely rewritten (594s → 0.48s, 1,237x speedup). Sleep durations reduced in chaos/protocol tests.
- **Generated artifacts**: Removed `actual_gpu_validation.json`, `pipeline_validation_actual_hardware.json` from git.
- **`toadstool-testing` dep**: Removed from examples crate (unused).
- Verification: `cargo fmt` ✅ `cargo clippy -D warnings` ✅ `cargo doc` ✅ `cargo test` ✅ (19,109 pass, 0 fail)

### Sessions S95–S96: Spring Absorption + Sovereign Pipeline + Debris Cleanup (Mar 6, 2026)
- **Sovereign pipeline**: `HardwareFingerprint` (estimated TFLOPS f32/f64, sovereign_capable flag), `is_sovereign_capable()`, `safe_allocation_limit` (NVK PTE fault mitigation), 12-variant `SubstrateCapabilityKind` (F64Native, Df64Emulation, Spmv, Eigen, Cg, Fft, MdForce, MonteCarlo, NnInference, ReservoirCompute, Fhe, SubgroupOps).
- **SubstrateType expansion**: 4→8 variants (Cpu, Gpu, IntegratedGpu, Npu, Tpu, Fpga, Dsp, Quantum) with `is_batch_oriented()` / `is_latency_oriented()` helpers.
- **God file splits (5)**: `dispatch.rs` (1252L→7 modules), `detection.rs` (1004L→3 modules), `engine.rs` (1098L→2 modules), `protocols/lib.rs` (985L→2 modules), `specialized_templates.rs` (924L→4 modules).
- **API orphan resolved**: `crates/api/` ByobApi route logic extracted to `crates/runtime/container/src/byob_routes.rs`; toadstool-api dependency removed from container.
- **V4L2 unsafe documentation**: All `unsafe` blocks in `v4l2/device.rs` documented with `// SAFETY:` comments.
- **Hardcoded IP evolved**: `0.0.0.0` fallback → `TOADSTOOL_DISCOVERY_BIND_ADDR` env var.
- **Debris cleanup**: Root `tests/` stubs removed (fossilized to ecoPrimals/fossil/). Stale `✅ COMPLETE` checklists cleaned from 11 files. False-positive TODO in `input/parser.rs` removed. Sprint/date doc comments cleaned in test files.
- **management/resources re-added**: Real `ResourceManager` re-added to workspace (sysinfo → `toadstool-sysmon` S137).
- **Clippy pedantic**: Resolved across workspace. `cargo clippy --lib -- -W clippy::pedantic` clean.
- **Spring absorption tracker**: Updated to current spring versions (hotSpring v0.6.17, groundSpring V80, neuralSpring V86/S128, wetSpring V97d, airSpring V071).

### Session 94b: Deep Execution + Spring Absorption
- **NpuDispatch trait** (`toadstool-core::npu_dispatch`): generic `NpuDispatch` trait + `AkidaNpuDispatch` adapter. Vendor-agnostic, capability-based, zero-copy `Cow` input. `NpuModelHandle`, `DispatchResult`, `NpuCapability` enum, `NpuInfo` struct.
- **NpuParameterController trait** (`toadstool-core::npu_controller`): absorbed from hotSpring — generic NPU-driven parameter tuning. `ParameterSuggestion<P>`, `SafetyClamp<P>`, `SuggestionSource`, `ControllerError`. Springs implement for domain-specific tuning.
- **GpuAdapterInfo** (`toadstool-runtime-universal`): exposes driver name, driver_info, vendor/device ID, backend, device type, workgroup limits, max buffer size, and shader-f64 support. barraCuda uses this for `GpuDriverProfile` (NVK detection, f64 workarounds).
- **Multi-adapter GPU selection**: `TOADSTOOL_GPU_ADAPTER` env var with comma-separated fallback (index, name substring, "auto"). Absorbed from hotSpring's `adapter.rs` pattern.
- **NestGate mock → real RPC**: `store_artifact`/`retrieve_artifact` evolved from hardcoded stubs to real JSON-RPC calls with graceful fallback when storage service unavailable.
- **Placeholder crate removed**: `management/resources` excluded from workspace (empty crate polluting build graph).
- **Production mock audit**: Complete — all remaining stubs are either error-returning (correct behavior for unimplemented hardware), test-gated, or documented heuristic models.
- **External dependency audit**: Workspace clean — all non-Rust deps behind optional features on excluded crates. No `build.rs` files.
- **Large file audit**: Production code well under 1000L limit. 812L and 806L files are ~490L production + ~320L tests.
- Verification: `cargo fmt` ✅ `cargo clippy -D warnings` ✅ `cargo doc` ✅ `cargo test` ✅ (all pass, 0 fail)

### Session 94: Deep Debt Execution — Fossilization + Deletion + Refactoring
- Removed dead `barracuda` dependency from `core/toadstool/Cargo.toml` (zero imports; barracuda is a peer primal)
- Fossilized `crates/barracuda/` (15MB, 1,790 files) to `ecoPrimals/fossil/toadStool/barracuda-fossil-S94b/`
- Deleted `manual_jsonrpc` module entirely (8 files + integration tests); `pure_jsonrpc` is canonical
- Smart-refactored `vfio.rs` (971L) into `vfio/` directory: `types.rs`, `ioctl.rs`, `dma.rs`, `mod.rs`
- Updated all doc references (ManualJsonRpcServer → pure_jsonrpc::JsonRpcHandler)
- Audited production panics/unwraps: all in test code (clean production)
- All files under 1000 lines (largest: 936 line test file)
- Verification: `cargo fmt` ✅ `cargo clippy -D warnings` ✅ `cargo doc` ✅ `cargo test` 17,986 pass, 0 fail

### Session 93: D-DF64 Transfer & Root Doc Cleanup
- Transferred D-DF64, D-CD (ComputeDispatch), DF64 transcendentals, arch-specific polynomial selection, naga-IR optimizer evolution, and barraCuda budding Phases 1-4 to barraCuda team ownership
- Created formal handoff: `wateringHole/handoffs/TOADSTOOL_S93_DF64_HANDOFF_MAR03_2026.md`
- Cleaned NEXT_STEPS.md to focus on toadStool-owned remaining work (D-NPU, D-COV, D-SOV, smart refactoring)
- Deleted 12 stale docs/debris files (~90 KB): orphan txt, completed migration guides, self-congratulatory status reports
- Root docs synchronized: STATUS, README, QUICK_REFERENCE, BREAKING_CHANGES, DOCUMENTATION, EVOLUTION_TRACKER, SPRING_ABSORPTION_TRACKER

### Session 92: Sovereignty Deprecation Sweep & Audit Continuation
- Deprecated `get_socket_path_for_service`, `get_primal_default_port`, `capability_typical_provider` with `#[deprecated(since = "0.92.0")]`
- Migrated NestGate client to `get_socket_path_for_capability` (3 callsites)
- Added `EcosystemDiscoverer::find_pattern_by_capability()` for capability-based lookup
- Neutralized 5 BearDog user-facing strings in access control manager
- `version_info()` → "Pure Rust (ecoPrimals sovereign pattern)"
- Removed dead middleware.rs + 7 test files (~131 KB)
- +47 tests → 5,369 total (monitoring, templates, installer, connection, wasm_ops, session)
- ecoBin `pure-rust` build verified: zero C FFI deps
- Fixed `bail!` macro undefined on `#[cfg(not(feature = "wasm"))]` path
- Extracted `verify_sha256()` as standalone fn for testability
- Audited: 0 production `todo!()`, 0 `unimplemented!()`, 0 FIXME, 0 HACK

### Session 90: Deep Audit, REST Removal, Sovereignty Evolution
- Fixed SIGSEGV in runtime-universal (wgpu catch_unwind + timeout)
- Unified 37 Cargo.toml license fields to workspace. 2,780+ SPDX headers added/normalized.
- Capability-based trust model. `get_socket_path_for_capability()` API added.
- Removed all REST routes + handlers + 8 test files. JSON-RPC only.
- Arc-cached compiled kernels, moved Vec, Arc<str> version on hot paths.
- PyO3 feature-gated. Python runtime optional in CLI.
- Documented all unsafe blocks in akida-driver.
- Rewrote handlers_basic_tests.rs (15 JSON-RPC integration tests).
- 5,322 tests, 0 failures.

### Session 89: barraCuda Primal Budding
- Full barracuda crate extracted to `ecoPrimals/barraCuda/` (956 .rs, 767 WGSL, 61 tests)
- `toadstool-core` gated behind `#[cfg(feature = "toadstool")]` — 1 file: `device/toadstool_integration.rs`
- `akida-driver` gated behind `#[cfg(feature = "npu-akida")]` — `npu/ml_backend.rs` + `npu/ops/` + bridge callsites
- `DeviceSelection`/`HardwareWorkload` extracted to `device/mod.rs` (zero external deps)
- `barracuda-core` wired: `BarraCudaPrimal::start()` runs device discovery, health reports adapter info
- Standalone quality: `cargo check/clippy/test` all pass (2,832 tests, 0 failures)
- MSRV 1.87 (code uses `is_multiple_of`)
- toadStool completely unchanged
- Pushed to GitHub: `ecoPrimals/barraCuda`

## Completed (S87-S88)

### Session 87: Deep Debt Resolution + Idiomatic Concurrent Rust + Code Quality
- TODO(afit) → NOTE(async-dyn): 75 instances across 52 files (reclassified from debt to architectural decision)
- gpu_helpers.rs: 663 lines → 3 submodules (buffers.rs, bind_group_layouts.rs, pipelines.rs)
- Unsafe code audit: All ~60+ sites documented; all verified necessary
- Hardware verification: 3 pre-existing failures fixed; 13/13 pass
- Hotspring fault tests: 6 pre-existing failures fixed — input validation, relaxed GPU assertions, device capability checks
- FHE shader fixes: u64_mod_simple + mod_mul; 19 FHE tests pass; MatMul/FHE validation; chaos test moduli constrained
- Device-lost recovery: BarracudaError::is_device_lost() + with_device_retry test helper

## Completed (S84–S86)

### Session 86: ComputeDispatch Batch 7 + Production Stub Evolution
- 12 ops → ComputeDispatch (determinant, mse_loss, dice, quantize, dequantize, bce_loss, permute, movedim, logsumexp, index_add, tensor_split, concat)
- wgpu_backend.rs: magic numbers → real `device.limits()` queries
- deployment.rs: 10 placeholder stubs → capability-discovery documentation
- Full ops audit: corrected remaining count from ~57 to ~139

### Sessions 84–85: ComputeDispatch Batches 5–6 + Hydrology + Probes
- 21 ops → ComputeDispatch across two sessions
- hydrology.rs god file → hydrology/ directory module
- experimental.rs stub → real FPGA/neuromorphic/quantum probes
- mDNS constants extracted; frameworks.rs echo → proper error

### Session 80: Nautilus Absorption + BatchedEncoder + Nelder-Mead GPU
- `barracuda::nautilus` module (7 files, 22 tests) — standalone bingoCube evolutionary reservoir computing
- `ai.nautilus.*` 8 JSON-RPC methods wired into daemon (feature-gated `nautilus`)
- `BatchedEncoder` — single `CommandEncoder` for multi-op GPU pipelines (2 tests)
- `fused_mlp` — MLP forward pass via BatchedEncoder (single submit across layers)
- Batch Nelder-Mead GPU — N parallel optimizations, batched simplex shader ops
- `StatefulPipeline<S>` + `WaterBalanceState` — day-over-day state tracking
- `GpuDriverProfile` sin/cos F64 workarounds (Taylor preamble for NVK, 4 tests)
- `NeighborMode::PrecomputedBuffer` — 2D/3D/4D periodic lattice precomputation (6 tests)
- `BatchedMultinomialGpu` alignment — `cumulative_probs` + `seed` (groundSpring V37)
- ComputeDispatch: 76→95 ops (4 migration batches, 19 ops)
- Socket resolution consolidated: 4 call sites → `toadstool_common::primal_sockets` API
- Confirmed existing: `SparseGemmF64`, IPC multi-transport

### Session 79: ESN MultiHeadEsn + ExportedWeights + SpectralAnalysis
- 36-head `MultiHeadEsn` with 6 `HeadGroup` variants, `head_disagreement()` uncertainty
- `ExportedWeights` aligned with hotSpring: input_size, reservoir_size, output_size, leak_rate, head_labels
- `SpectralAnalysis` extensions: spectral_bandwidth, spectral_condition_number, classify_spectral_phase
- ComputeDispatch: 5 more ops → 76 total
- bitcast<f64> fixes in 2 WGSL shaders → storage buffer approach

## Completed (S78)

### Session 78: Deep Debt + Dependency Evolution
- Wildcard re-exports narrowed in 7 more crates (sandbox, wasm, edge discovery/toolchain/comms/deployment). Total: 13.
- `legacy_primal_to_capabilities` / `legacy_primal_primary_capability` removed from primal_capabilities.rs (no callers).
- `libc` fully removed from akida-driver — rustix for VFIO ioctls. Custom VfioIoctlReturn/VfioIoctlPtr wrappers.
- async-trait → native AFIT in security/sandbox (SandboxManager). Total: 5 crates.
- ComputeDispatch: 5 more ops (eq, map, dotproduct, dropout, split). Total: 71.
- ~40 new tests (api ~20, auto-config ~9, server ~11).
- 5 ToadStoolError doc links fixed.
- Compile bottleneck analysis done.

## Completed (S74 through S75)

### Session 75: Module Architecture + Build Streamlining
- 6 god files smart-refactored: primal_integration.rs (1,163L→5 modules), capability_provider.rs (746L→5 modules), primals/lib.rs (580L→7 modules), opencl_impl.rs (831L→6 modules), env_overrides.rs (726L→9 modules), os_layer/compat.rs (766L→7 modules)
- Wildcard `pub use *` narrowed in 6 crates: toadstool, distributed, server, gpu, universal, orchestration
- pollster removed from toadstool + universal
- 3 evolved backends gated behind `#[cfg(test)]`
- TYPES_REFERENCE.md updated with Module Structure Reference

### Session 74: Deep Debt Evolution — Dependencies + Capabilities + Resilience
- serde_yaml → serde_yaml_ng across workspace
- async-trait → native AFIT in 4 crates
- pollster → tokio_block_on in barracuda (dependency removed)
- Hardcoded primal names → capability-based language + type aliases
- Edge platform stubs → genuine hardware probing
- Discovery stubs → real mDNS/k8s/docker/registry probing
- 3 god files refactored: workload.rs, unified.rs, precision/mod.rs
- GPU test resilience: catch_unwind wrappers for NVK driver panics
- WgpuDevice::poll_safe() for device-lost recovery
- Net -3,828 lines across 182 files

### Previously Completed (S68–S71)
- **S71**: 6 GPU dispatch structs, DF64 transcendental suite (15 functions), 32 ComputeDispatch migrations, 6 god files refactored, net -9,192 lines
- **S70+++**: builder.rs refactored, EcosystemCaller deleted, monitoring evolved to real sysinfo
- **S70+/++**: 7 WGSL shaders, sovereignty evolution, Fp64Strategy::Concurrent, +37 tests
- **S70**: 15 stubs → real implementations, all env tests → temp_env, +150 tests
- **S69++**: metalForge streaming, manual_jsonrpc → pure_jsonrpc, 34 ComputeDispatch ops
- **S69/69+**: 5 spring handoffs absorbed, 30+ WGSL shaders, anyhow eliminated
- **S68+++**: chrono eliminated (28 crates), unsafe 47→45, ~400 lines dead code

---

See [CHANGELOG.md](CHANGELOG.md) for full completed session history.
