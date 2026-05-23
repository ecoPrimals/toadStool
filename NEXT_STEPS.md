# ToadStool -- Next Steps

**Updated**: May 2026 — S268 (Kernel Health Preflight: 3-layer `autoconf.h` mismatch detection in `cylinder::vfio::kernel_health`. Blocks warm handoff / DKMS builds on corrupted build env. `sovereign.kernel_health` RPC + `toadstool kernel-health` CLI. 700 cylinder tests.)
**Status**: Production-grade | Rust edition **2024** (MSRV 1.85) | **AGPL-3.0-or-later** | **All quality gates green** | tests verified (23,000+ workspace, 0 failures; 9,126+ lib-only) | **88 JSON-RPC methods** | Wire Standard L3 (partial) | Zero C FFI deps (ecoBin v3.0) | **Zero production panics/expects** | **Zero production TODO/FIXME/HACK** | **Zero production unreachable!()** | IPC-first | workspace `unsafe_code = "deny"`, **41 crates `forbid`** | **46 unsafe blocks** (all in hw containment, all SAFETY-documented) | **rustix 1.x workspace-wide** | **capability-based primal references (no hardcoded names)** | **`async-trait` DEPRECATED** (banned in `deny.toml`) | **`deny.toml` ring + async-trait + zstd-sys bans active** | **Phase C complete — all blocking items resolved (S253)** | **Phase D dispatch live — QMD-based VFIO PBDMA dispatch wired (S258–S263)** | **`OwnedFd` VFIO fd ownership (S253)** | **`toadstool device` CLI (S253)** | **CORALREEF_* env vars deprecated with TOADSTOOL_* primaries (S253)** | **Zero `#[allow(deprecated)]` remaining** | **700 cylinder tests** | **E2E sovereign dispatch VALIDATED on Titan V (warm handoff)**
**Latest**: S268 — **Kernel Health Preflight**: `kernel_health.rs` 3-layer build env check (autoconf freshness, struct probe, RELA cross-check). Integrated into sovereign handoff step 0d, DKMS build guard, `sovereign.kernel_health` RPC, `toadstool kernel-health` CLI. Post-fix audit: all 20 DKMS + 10 installed modules clean. S267 — Sovereign driver rotation via diesel engine.
**Previous**: S266 — PLX keepalive root cause fix. S265r — Driver Lab + Containment. S264 — PCIe bridge keepalive. S263 — CPUCTL_ALIAS breakthrough, GR context scheduler, warm handoff on Titan V.

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

### P1: Test Coverage → 90% (D-COV) — Ongoing (S164)

**~83.6% line coverage** (lib-only, 185K lines instrumented). **22,900+ tests** (0 failures, 8,849+ lib-only). Target 90%.

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
| `max_guest_load` yield semantics — power-cycle scheduling for flockGate | LOW | **TYPES SHIPPED** (S269) — `GuestLoadPolicy` + `YieldStrategy` on `TenantQuota`. Orchestrator enforcement pending flockGate integration spec. |

### Key Remaining Items (S268)

| Item | Status |
|------|--------|
| Coverage push 83%→90% | Ongoing — hardware mocks needed for remaining gaps |
| Phase D mixed command streams | Planned — requires coralReef FECS firmware loading |
| VFIO PBDMA dispatch | **VALIDATED** (S258–S263) — GPFIFO + QMD dispatch works e2e on Titan V via warm handoff. FECS alive via CPUCTL_ALIAS. DMA roundtrip confirmed. |
| PCIe bridge keepalive | **VALIDATED + EVOLVED** (S264→S266) — Phase 1 (S264): `pin_bridge_hierarchy()` + `SwapGuard` burst CfgRd during swaps. Phase 2 (S266): Root cause fix — PLX D3cold caused by **inactivity** (not swaps). `PlxKeepalive` (ember): continuous CfgRd every 5s on device + all upstream bridges. `PlxGuardian` (glowplug): fleet-level auto-detect via `scan_and_protect()`. 98 ember tests, 95 glowplug tests. |
| E2E sovereign pipeline test | **VALIDATED** (S263) — warm handoff → VFIO open → channel → dispatch → readback. Pending: real shader execution (FECS PENDING_CTX_RELOAD frontier). |
| FECS golden context mapping | **ACTIVE** — FECS scheduler stuck at PENDING_CTX_RELOAD. GR context buffer allocated but FECS needs golden context from VRAM. Next: map VRAM identity region or extract context init sequence from nouveau. |
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
| `crypto.sign_contract` (PG-60+) | BearDog team | Cross-family ionic bond contract signing — expose as JSON-RPC method (proposer, acceptor, capabilities, duration). primalSpring `bonding::ionic_rpc` ready to consume. Phase 60+, no urgency. |

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
- [ ] **Test coverage target 90%** -- 22,900+ tests (8,849+ lib-only); ~83.6% line; mock hardware layers for V4L2/VFIO (MockV4l2Device, MockVfioDevice); push to 90% ongoing
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
- [x] **musl-static release binary (S198)** -- ~11MB x86_64 PIE stripped, validated
- [x] **API orphan resolved** -- crates/api/ ByobApi extracted to container crate (S96)
- [x] **V4L2 unsafe docs** -- All SAFETY comments on unsafe blocks (S96)
- [x] **Debris cleanup** -- root tests/ stubs, stale checklists, false-positive TODOs (S95)
- [x] **management/resources re-added** -- real ResourceManager (S95; sysinfo → `toadstool-sysmon` S137)

### Cross-Repo Debt

- [x] **D-S20-003**: neuralSpring `evolved/` migration — **RESOLVED** (neuralSpring V89 completed; `evolved/` removed)
- [x] **D-S18-002**: cubecl transitive `dirs-sys` — **RESOLVED** (cubecl fully removed; `dirs-sys-next` now only via wasmtime-cache, feature-gated)

---

## Completed Sessions (Archived)

Session history for S43–S266 lives in [CHANGELOG.md](CHANGELOG.md). Fossil record for S87–S240 archived to `ecoPrimals/infra/wateringHole/fossilRecord/toadstool/`.
