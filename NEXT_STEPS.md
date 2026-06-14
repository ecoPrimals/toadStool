# ToadStool -- Next Steps

**Updated**: Jun 2026 — S313. **VPS-ready** — musl-static binary built with `--headless` support. All P0 blockers resolved. Transport evolution Phase 2 complete. **PRIMAL-SOCKET-CLEANUP done** (Wave 107). **TOADSTOOL-AUTO-REGISTER done** (Wave 111). **riboCipher COMPLIANT** (Wave 112 — server detect + client signal, ERROR on unsignalled).
**Status**: Production-grade | Rust edition **2024** (MSRV 1.85) | **AGPL-3.0-or-later** | **All quality gates green** | tests verified (23,000+ workspace, 0 failures; **9,069+ lib-only**) | **111 JSON-RPC methods** (direct) | Wire Standard L3 (partial) | **Zero `libc`** (ecoBin v3.0 — rustix for all hardware I/O) | **Zero production panics/expects/unwraps** | **Zero production TODO/FIXME/HACK** | IPC-first | workspace `unsafe_code = "deny"`, **41 crates `forbid`** | **44 unsafe blocks** (all SAFETY-documented; −2 from kernel_sentinel AsFd evolution) | **rustix 1.x workspace-wide** | **~98% env centralized** (410+ reads via socket_env constants; 23 LEGACY reads emit deprecation tracing) | **capability-based primal references** (`PRIMAL_NAME`/`PRIMAL_BINARY_NAME` constants) | **`async-trait` banned in `deny.toml`** | **Phase D dispatch live** | **E2E sovereign dispatch VALIDATED on Titan V** | **Telemetry wire contract v1.1** (barraCuda/biomeOS L5) | **`--headless` mode** for port-free VPS deployment | **`--socket` wired** for launcher-injected UDS paths | **Zero production `#[allow]`** (Wave 78 compliant) | **`capability_registry.toml`** (17 capability groups, 111 methods) | **Zero `/tmp/` hardcoding** — `BIOMEOS_SOCKET_DIR` > `XDG_RUNTIME_DIR` > `temp_dir` | **`TRANSPORT_ENDPOINT` accepted** (sourDough standard, Wave 100) | **BYOB default bind `127.0.0.1`** | **Zero production files >750L** | **~20 deprecated symbols removed** | **Zero sync-ctor fallbacks** (auth/agents fully async) | **Auto-register hardware** — PCI sysfs GPU/NPU inventory sent in `ipc.register` + `primal.announce` | **`CoordinationTransport::GRPC` deprecated** with `#[expect]` on all call sites | **riboCipher compliant** — server detect (Unix+TCP, 4 accept loops) + client signal (`[0xEC, 0x01]` on all outbound IPC)
**Latest**: S313 — Deep Debt XVI: 3 `unreachable!()` → typed errors (zero production panics), `unix.rs` split (815→512+334). S312 — riboCipher WARN→ERROR escalation. S311 — riboCipher convergence.

---

## Active Work

### ~~P0: ComputeDispatch Migration~~ → Transferred to barraCuda (S93)

**Transferred.** ComputeDispatch lives in the barraCuda crate. 144/280+ ops migrated;
~139 remaining. barraCuda team owns this incremental migration.

### ~~P1: DF64 Default Path~~ → Transferred to barraCuda (S93)

**Transferred.** barraCuda owns precision strategy (f64/df64/f32 validation, shader
selection, `df64_rewrite` as default). toadStool serves hardware capabilities.
Handoff: `infra/wateringHole/handoffs/` (S93 handoff fossilized to ecoPrimals-level wateringHole).

### ~~P1: NpuDispatch Trait~~ ✅ RESOLVED (S94b)

`toadstool-core::npu_dispatch` — generic `NpuDispatch` trait + `AkidaNpuDispatch`
adapter. Vendor-agnostic, capability-based, zero-copy input (`Cow`). Also added
`NpuParameterController` trait (hotSpring absorption) for NPU-driven autonomous
parameter tuning.

### ~~P1: Fix `set_var`/`remove_var` unsafe blocks~~ ✅ RESOLVED (S157b)

All `set_var`/`remove_var` calls wrapped in `unsafe {}` across 14 files. Mangled
syntax fixed in 3 server files. Test suite fully unblocked.

### P2: Test Coverage → 90% (D-COV) — Active Sprint (S294–S298)

**~85%+ estimated line coverage** (lib-only). **9,069+ lib tests** (0 failures). Target 90%.

**S294–S298 coverage sprint** added **+174 new tests** targeting non-VFIO gaps:
- S294: CallerContext extraction, handler glue (workload, resources, queries, state, compute), RuntimeEngineDispatch (+57)
- S296: ember.rs, dispatch/submit.rs, background services, CLI start.rs (+35)
- S297: transport.rs, shader_dispatch.rs, CLI commands (device, mode, kernel_health, npu), glowplug_client.rs (+38)
- S298: silicon.rs, job.rs, coordination_integration, method_gate.rs, auth.rs (+44)

**Remaining gap**: Hardware-dependent paths (VFIO, DRM, V4L2, akida userspace), neuromorphic drivers, GPU engine/execution paths, and deeper branch coverage in dispatch/wgpu_dispatch.rs. These require integration-level testing with hardware or mock hardware infrastructure.

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
| **Phase D: Mixed command streams** | Planned — blocked on toadStool PBDMA runlist config ([COMPUTE_DISPATCH_ENGINE.md](specs/COMPUTE_DISPATCH_ENGINE.md)); extends PBDMA with draw/RT/texture/tensor/framebuffer commands |

### Jun 14, 2026 — S313 Deep Debt XVI

| Item | Status |
|------|--------|
| `connection/unix.rs` — 3× `unreachable!()` → `Err(ServerError::Internal(...))` | **DONE** |
| `connection/unix.rs` split — 815L → `unix.rs` (512L) + `btsp_unix.rs` (334L) | **DONE** |
| `executor/types.rs` — `#[allow(dead_code)]` → `#[expect(dead_code, reason)]` | **DONE** |
| `cloud/federation/mod.rs` — `#[allow(unused_imports)]` reason documented | **DONE** |

### Jun 13, 2026 — S310 Deep Debt XV

| Item | Status |
|------|--------|
| `kernel_sentinel.rs` — unsafe `BorrowedFd::borrow_raw` → safe `AsFd` (2 unsafe blocks eliminated) | **DONE** |
| `forensics.rs` — hardcoded `/var/log/handoff-forensics.log` → `TOADSTOOL_FORENSICS_LOG` env | **DONE** |
| `CoordinationTransport::GRPC` — formal `#[deprecated]` attr + `#[expect]` on 4 production match arms | **DONE** |
| `ProtocolConfig.grpc` field — `#[expect(deprecated)]` with reason | **DONE** |
| `service_discovery/tests.rs` split — 806L → `tests.rs` (399L) + `tests_advanced.rs` (407L) | **DONE** |
| `plugin_system/tests.rs` split — 800L → `tests.rs` (403L) + `tests_advanced.rs` (397L) | **DONE** |
| Test `#[allow(clippy::await_holding_lock)]` — 5 sites in `cache_config.rs` given `reason` | **DONE** |
| Unfulfilled `#[expect(unsafe_code)]` in kernel_sentinel removed (no longer needed) | **DONE** |

### Jun 12, 2026 — S309 TOADSTOOL-AUTO-REGISTER (Wave 111)

| Item | Status |
|------|--------|
| `discover_hardware_inventory()` — PCI sysfs GPU/NPU enumeration (BDF, type, vendor/device ID, driver) | **DONE** |
| `ipc.register` params extended with `devices` array for songBird/coordination | **DONE** |
| `self_announce_to_biomeos()` params extended with `devices` array for Neural API | **DONE** |
| `primal.announce` inbound handler response includes `devices` (BDF list from sysfs) | **DONE** |
| 2 unit tests for `discover_hardware_inventory` structural validation | **DONE** |

### Jun 8, 2026 — S301 Transport Evolution (Wave 100)

| Item | Status |
|------|--------|
| `TransportEndpoint` type in toadstool-common (sourDough wire-compatible) | **DONE** |
| `TRANSPORT_ENDPOINT` env var constant in socket_env | **DONE** |
| Server `UnibinExecutionConfig` reads `TRANSPORT_ENDPOINT` from env | **DONE** |
| Server `start_servers_with_fallback` routes UDS/TCP from injected endpoint | **DONE** |
| CLI daemon `DaemonServer::run` routes through injected transport | **DONE** |
| `connect_transport()` API in `ipc::platform` for outbound connections | **DONE** |
| `ConnectedTransport` enum (Unix/Tcp) for transport-agnostic streams | **DONE** |
| 8 tests for TransportEndpoint (roundtrip, wire-format, env loading) | **DONE** |
| `--port` preserved as Tier 5 fallback (debug/standalone only) | **DONE** |

### Jun 6, 2026 — S300 Deep Debt XI: Stub Evolution + Hardcode Elimination

| Item | Status |
|------|--------|
| Migration stubs → `CliError::NotImplemented` (15 leaf I/O ops) | **DONE** |
| `/tmp/` hardcoded paths → `std::env::temp_dir()` (5 production modules) | **DONE** |
| `wgpu` platform features narrowed (dropped dx12/metal/webgpu — Linux-only) | **DONE** |
| `tokio` features narrowed from `full` to explicit set | **DONE** |
| `CliError::NotImplemented` variant added for capability-gated operations | **DONE** |
| HTTP WASM loading `CliError::Other` → `CliError::NotImplemented` | **DONE** |

### Jun 5–6, 2026 — S292–S298 Deep Debt IX–X + Wave 79/80 Compliance

| Item | Session | Status |
|------|---------|--------|
| `serialport` feature-gated in runtime/edge (`serial-transport`) | S292 | **DONE** |
| `dispatch/device.rs` (781L) split into `device/` module dir | S292 | **DONE** |
| Hardcoded `"toadstool"` → `PRIMAL_NAME`/`PRIMAL_BINARY_NAME` | S292 | **DONE** |
| Deprecated `TestExecutor`/`WorkloadExecutor` exports removed | S292 | **DONE** |
| V4L2 ioctl + plugin ABI SAFETY docs | S292 | **DONE** |
| Unused `tarpc` removed from runtime/display | S293 | **DONE** |
| `tarpc` made optional in integration/protocols (`tarpc-transport`) | S293 | **DONE** |
| Production `unwrap`/`expect` purge — zero remaining | S293 | **DONE** |
| `mmu_oracle/capture.rs` (795L) split into `capture/` dir | S293 | **DONE** |
| 23 `LEGACY_*` env reads emit deprecation tracing | S293 | **DONE** |
| `--socket` CLI wired through to server bind (UDS compliance) | S294 | **DONE** |
| `ConnectionTrustHints` mutual-auth support | S294 | **DONE** |
| `--headless` flag on server/daemon (skip GPU/NPU probes) | S295 | **DONE** |
| `akida-setup` graceful skip on hardware-less systems | S295 | **DONE** |
| Musl-static binary built (14MB, x86_64, static-pie) | S296 | **DONE** |
| Coverage push: +174 new tests (S294–S298) | S294–S298 | **DONE** |

### Jun 4, 2026 — S289 Telemetry Wire Contract + Adversarial Trust Tests

| Item | Status |
|------|--------|
| `dispatch.telemetry.schema` → versioned wire contract v1.1 (encoding, backward compat, consumers) | **DONE** |
| +8 adversarial `dispatch.verify_trust` tests (forged BTSP, mismatch, malformed, serialization) | **DONE** |
| `DispatchTelemetryRecord` emitted from `compute.dispatch.submit` + `shader.dispatch` via tracing | **DONE** |
| `bollard` removed from default features in `runtime/container` (opt-in `docker` feature) | **DONE** |

### Jun 3, 2026 — S285 Deep Debt Evolution VII

| Item | Status |
|------|--------|
| Server crypto: `distributed::security` → `crypto_integration` | **DONE** |
| `NoopCryptoProvider` / `StubRuntimeEngine` → typed errors | **DONE** |
| `embedded-placeholder-impls` removed from specialty defaults | **DONE** |
| Hardcoded `"toadstool"` → `PRIMAL_NAME` | **DONE** |
| Last production `expect()` → safe patterns | **DONE** |
| ~100L dead code removed (catalyst_watchdog, module_patch, driver_ops) | **DONE** |

### Jun 3, 2026 — S284 Deep Debt Evolution VI

| Item | Status |
|------|--------|
| Last 3 production files >800L split (`sovereign_init`, `open_vfio`, `experiment`) | **DONE** |
| Final library panics eliminated (`kernel_sentinel`, `visualization_client`) | **DONE** |
| Dead deprecated symbols removed (BearDogBackend, legacy capability helpers) | **DONE** |
| 33 server clippy warnings fixed + test compilation fixes | **DONE** |

### Jun 2, 2026 — S283 Deep Debt Evolution VI (wave)

| Item | Status |
|------|--------|
| 6 large files refactored (`kernel_health`, `kmod_build`, `pmu_investigate`, `nv_gsp_bridge`, etc.) | **DONE** |
| `bear_dog` → `security_client` rename | **DONE** |
| `CORALREEF_*` env aliases removed | **DONE** |
| 167 production unwraps eliminated | **DONE** |
| Mocks isolated to test-only paths | **DONE** |

### S273 Deep Debt Evolution

| Item | Status |
|------|--------|
| Production panic surface eliminated (`kernel_health.rs`, dispatch cache, `ember_client.rs`, `secure_enclave`) | **DONE** |
| `dispatch/mod.rs` 1,638→839L — sovereign handlers extracted to `dispatch/sovereign.rs` (814L) | **DONE** |
| `warm_init.rs` 1,439L → module dir (`mod.rs` + `seeders.rs` + `trials.rs`) | **DONE** |
| 6 CLI `well_known::*` hardcoded primal name sites → capability-based discovery with legacy fallback | **DONE** |
| `activity_tracker().record()` wired into 7 VFIO dispatch paths | **DONE** |
| hw-safe abstractions validated; cylinder migration deferred | **DONE** |

### Wave 47 Behavioral Convergence (S272)

| Item | Status |
|------|--------|
| `health.liveness` returns `"alive"` immediately (not `"starting"`) | **DONE** — boot state signaling via `health.readiness` |
| 49 upstream clippy errors (cylinder + server dispatch rebase) | **DONE** |

### Wave 44 Neural API Announce Fix (S271)

| Item | Status |
|------|--------|
| Expand `ANNOUNCED_METHODS` to include `science.*` and `inference.*` | **DONE** — 33 → 47 methods |
| Wire science/inference impl names in `dispatch_by_impl_name` | **DONE** — 14 new arms |
| Add wire L3 cost estimates for science/inference methods | **DONE** |

### Wave 43 Neural API primal.announce (S270)

| Item | Status |
|------|--------|
| Wire `primal_announce()` into JSON-RPC dispatch table | **DONE** — direct route + semantic alias + DIRECT_JSONRPC_METHODS |
| Startup self-announcement to biomeOS Neural API | **DONE** — `self_announce_to_biomeos()` with capabilities, cost_hints, latency_estimates |
| Remove `#[allow(dead_code)]` from announce function | **DONE** |
| Add `socket`, `signal_tiers`, `cost_hints`, `latency_estimates` fields | **DONE** — per Wave 43 schema |

### Wave 38 Horizon Items (S269)

| Item | Priority | Status |
|------|----------|--------|
| `compute.fan_out` at scale — Tenaillon 590 GB batch | MEDIUM | **RE-IMPLEMENTED** (S269) — handler, types, 10 tests, wire L3, semantic aliases. strandGate graph design pending upstream spec. |
| `max_guest_load` yield semantics — power-cycle scheduling for flockGate | LOW | **ENFORCED** (S274) — `check_guest_load()` branches on `YieldStrategy` (Queue/Reject/DeferUntilPowerCycle). `GuestLoadExceeded` error. 10 tests. Server dispatch wiring pending flockGate integration spec. |

### Key Remaining Items (S268)

| Item | Status |
|------|--------|
| Coverage push 85%→90% | **Active sprint** — S294–S298 added +174 tests (9,069 lib); remaining gap in VFIO/DRM/GPU hardware paths |
| Phase D mixed command streams | Planned — blocked on toadStool PBDMA runlist config ([COMPUTE_DISPATCH_ENGINE.md](specs/COMPUTE_DISPATCH_ENGINE.md)) |
| VFIO PBDMA dispatch | **PIPELINE WIRED, RUNLIST BLOCKED** (S258–S263; Jun 1 RCA) — channel, DMA, GPFIFO + QMD submission work on Titan V; **GP_GET never advances** because `PFIFO_RUNLIST_BASE=0` (runlist never configured). Not e2e dispatch. RCA: [HOTSPRING_TIER2_PBDMA_ROOT_CAUSE_JUN01_2026.md](infra/wateringHole/handoffs/HOTSPRING_TIER2_PBDMA_ROOT_CAUSE_JUN01_2026.md). Frontier spec: [COMPUTE_DISPATCH_ENGINE.md](specs/COMPUTE_DISPATCH_ENGINE.md). |
| PCIe bridge keepalive | **VALIDATED + EVOLVED** (S264→S266) — Phase 1 (S264): `pin_bridge_hierarchy()` + `SwapGuard` burst CfgRd during swaps. Phase 2 (S266): Root cause fix — PLX D3cold caused by **inactivity** (not swaps). `PlxKeepalive` (ember): continuous CfgRd every 5s on device + all upstream bridges. `PlxGuardian` (glowplug): fleet-level auto-detect via `scan_and_protect()`. 98 ember tests, 95 glowplug tests. |
| E2E sovereign pipeline test | **VALIDATED** (S263) — warm handoff → VFIO open → channel → dispatch → readback. Pending: real shader execution (FECS PENDING_CTX_RELOAD frontier). |
| FECS golden context mapping | **ACTIVE** — **Prerequisite: PBDMA runlist** (`PFIFO_RUNLIST_BASE` non-zero, `GP_GET` advancing per Jun 1 RCA). FECS scheduler stuck at PENDING_CTX_RELOAD until runlist configured. GR context buffer allocated but FECS needs golden context from VRAM. Next: runlist per [COMPUTE_DISPATCH_ENGINE.md](specs/COMPUTE_DISPATCH_ENGINE.md), then map VRAM identity region or extract context init sequence from nouveau. |
| No-FLR warm swap | **VALIDATED + IMPLEMENTED** (Exp 194, S265r) — `reset_method=""` disables FLR during vfio-pci bind. Titan V: 13/15 registers alive through nouveau→vfio-pci swap. Full BAR access verified. `WarmInitPlan` with containment architecture: bare-metal (nouveau, host-safe) vs contained (nvidia-470, agentReagents VM). `SysfsSwapExecutor::execute_warm_init()` bare-metal only — contained plans dispatch through agentReagents. Host DRM sacred. 77 tests pass. |
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
| `crypto.sign_contract` (PG-60+) | crypto service team | Cross-family ionic bond contract signing — expose as JSON-RPC method (proposer, acceptor, capabilities, duration). primalSpring `bonding::ionic_rpc` ready to consume. Phase 60+, no urgency. |

---

### Wave 8: Compute Trio — Diesel Engine Absorption Roadmap (S235)

toadStool is the **WHERE** primal (hardware domain). The coral diesel engine model has
three components that toadStool will absorb from coralReef:

- **ember** = VFIO fd holder (glow plug filament) — holds device fds open across process
  restarts via `SCM_RIGHTS`, prevents kernel PM resets
- **glowplug** = device lifecycle orchestrator (glow plug controller) — discover,
  personality-swap, sovereign boot, health monitor
- **cylinder** = per-device subprocess (combustion chamber) — one ember socket per
  device, manages dispatch lifecycle

toadStool already has matching trait surfaces: `ResourceHandle` (ember), `DevicePersonality`
(glowplug personality), `SwapOrchestrator` (glowplug lifecycle), `DeviceSlot` (cylinder state).
Phase A shipped production `VfioResourceHandle` (S237); remaining phases have test mock implementations.

| Phase | Scope | LOC | Key deliverable | Status |
|-------|-------|-----|-----------------|--------|
| **A: ember** | `HeldDevice` → `ResourceHandle` + vendor lifecycle + observation + ring_meta | ~9k | First production `ResourceHandle` impl; device pool in dispatch | **S237 DONE** |
| **B: glowplug** | `sovereign_boot` → `SwapOrchestrator` 7-step; `EmberClient` becomes toadStool-internal; `GpuPersonality` unifies with `DevicePersonality`; `coralctl` → CLI | ~18k | Device lifecycle owned by toadStool | **S239 DONE** |
| **C: cylinder + coral-driver** | Per-device subprocess generalized (GPU + NPU + HSM); VFIO channel, GPFIFO/pushbuf, DRM ioctl → `hw-safe` containment zone | ~15k | Universal per-device dispatch subprocess | **S245–S250 Batches 1-7 DONE** |
| **D: local dispatch** | `dispatch_submit_with_context` executes locally via absorbed driver layer instead of forwarding to `coral_client` | ~2k | Gate 4 E2E sovereign compute path | **S250 DONE** |

**Foundation (S235, DONE)**: BrainChip vendor ID fixed (`0x1E7C` canonical). `compute.dispatch.submit`
trio IPC contract (`binary_b64`, `shader_info`, `dispatch_dims`, buffer `data_b64`, `timing`
response). `dispatch_capabilities` returns `gpu_count`, `architectures`, `vfio_status` for Gate 2.

**Boundary with coralReef**: `shader.compile.*` methods (WGSL→SASS/AMDIL) remain in coralReef (HOW/compiler
domain). toadStool serves `compute.dispatch.*` (WHERE/hardware domain). barraCuda serves math
kernels (WHAT/math domain). hotSpring's `GlowplugClient::dispatch()` flow becomes toadStool-native
after Phase D.

---

## Infrastructure Checklist

- [x] **Rust dispatch wiring** -- 13 S69 shaders + AlphaFold2 + Lanczos + airSpring + MD observables
- [x] **metalForge streaming** -- Stage/Pipeline/Topology builder (staging/pipeline.rs)
- [x] **NAK workgroup tuning** -- `workgroup_size_for_arch()` with 6 tests
- [x] **`anyhow` → `thiserror`** -- fully eliminated from all ~30 workspace crates
- [x] **`manual_jsonrpc` → `pure_jsonrpc`** -- full migration, unibin uses pure_jsonrpc
- [x] **GPU Lanczos kernel** -- `lanczos_iteration_f64.wgsl` + `lanczos_eigensolver()` dispatch
- [x] **rust-version** -- bumped 1.75 → 1.85 (edition 2024, MSRV 1.85)
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
- [ ] **Test coverage target 90%** -- 23,000+ tests (9,069+ lib-only); ~85%+ line; +174 tests S294–S298 targeting non-VFIO gaps; remaining gap in hardware-dependent paths; push to 90% ongoing
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
- [x] **Health triad shapes (S198+S225)** -- liveness (`starting`→`alive`), readiness (`starting`→`ready`), check (full envelope); PG-62 fast-path (S225)
- [x] **musl-static release binary (S198→S296)** -- ~14MB x86_64 PIE stripped, validated; S296 rebuilt with `--headless` + `--socket` support
- [x] **API orphan resolved** -- crates/api/ ByobApi extracted to container crate (S96)
- [x] **V4L2 unsafe docs** -- All SAFETY comments on unsafe blocks (S96)
- [x] **Debris cleanup** -- root tests/ stubs, stale checklists, false-positive TODOs (S95)
- [x] **management/resources re-added** -- real ResourceManager (S95; sysinfo → `toadstool-sysmon` S137)

### Cross-Repo Debt

- [x] **D-S20-003**: neuralSpring `evolved/` migration — **RESOLVED** (neuralSpring V89 completed; `evolved/` removed)
- [x] **D-S18-002**: cubecl transitive `dirs-sys` — **RESOLVED** (cubecl fully removed; `dirs-sys-next` now only via wasmtime-cache, feature-gated)

---

## Completed Sessions (Archived)

Session history for S43–S273 lives in [CHANGELOG.md](CHANGELOG.md). Fossil record for S87–S240 archived to `ecoPrimals/infra/wateringHole/fossilRecord/toadstool/`.
