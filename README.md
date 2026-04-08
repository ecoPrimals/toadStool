# ToadStool

**Sovereign Compute Hardware** | Pure Rust | ecoBin | April 2026

---

## What Is This?

**ToadStool** is the hardware infrastructure primal — the **WHERE** in the Compute Trio (barraCuda = WHAT, toadStool = WHERE, coralReef = HOW). It discovers GPUs, NPUs, CPUs at runtime via sysfs/PCIe. JSON-RPC 2.0 + tarpc IPC over Unix sockets. GPU job queue with cross-gate routing. **All-silicon pipeline**: discovers every functional unit on the GPU die (shader cores, tensor cores, RT cores, TMUs, ROPs, rasterizer, depth buffer, tessellator, video encoder) and routes work to the cheapest unit that meets the requested tolerance.

**Key principles:**
- **Every piece of silicon** -- a GPU has 8+ special-purpose computers; toadStool discovers and routes to all of them
- **Tolerance-based routing** -- springs specify math tolerance, toadStool picks hardware; never the reverse
- **Capability-based discovery** -- primals discover each other at runtime by capability, not name
- **Self-knowledge only** -- ToadStool knows its own identity; everything else is discovered
- **ecoBin compliant** -- single binary, pure Rust, cross-architecture, cross-platform

**BarraCuda** (compute math) is a separate primal at `ecoPrimals/barraCuda/`. ToadStool provides hardware discovery and capability probing; barraCuda dispatches shaders and owns all math.

---

## Ecosystem Role

```
NUCLEUS = Security + Coordination + Compute + Storage
Tower   = Security + Coordination     <- communication + crypto
Node    = Tower  + ToadStool          <- us -- sovereign compute
Nest    = Tower  + Storage            <- storage
```

**biomeOS grade**: Node Atomic READY -- ToadStool A++ socket-standardized. **Wire Standard L3** (partial): `cost_estimates` + `operation_dependencies` on `capabilities.list`.

**Deployment**: Tower starts first (security service → coordination service), then ToadStool. Socket: `$XDG_RUNTIME_DIR/biomeos/toadstool.sock`. ToadStool discovers other primals at runtime by capability, not by name.

---

## Quality Gates

| Gate | Status |
|------|--------|
| `cargo build --workspace` | Clean |
| `cargo fmt --all -- --check` | 0 diffs |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 warnings |
| `cargo doc --workspace --no-deps` (RUSTDOCFLAGS="-D warnings") | 0 warnings |
| `cargo test --workspace` | **21,514 tests, 0 failures** (S191), 103 ignored (hardware-gated); full workspace ~3m30s |
| Doctests | All passing (common, core, server, cli, testing, display) |
| Standalone clone test | Pull to any machine, `cargo test` works (GPU-optional, CPU fallback, device-lost resilient) |
| `unsafe` blocks | **~66 actual** (all in hw-safe/GPU/VFIO/display containment crates); SAFETY-documented; **41 crates forbid, 6 deny** `unsafe_code` |
| Production panics/unwraps | **0** production `unwrap()` / `expect()` / `panic!()` |
| Production stubs / test mocks | Stubs evolved or typed errors; **auth test mocks** (`InMemoryAuthBackend`) isolated under **`#[cfg(any(test, feature = "test-mocks"))]`** |
| Production `Box<dyn Error>` | 0 in core crates -- all typed errors (thiserror) |
| Production TODOs / FIXME / HACK | 0 in production code |
| Dead code | ~400+ lines removed (REST, middleware, dead modules); dead_code attrs converted to `#[expect]` with reasons or `#[cfg(test)]` |
| External deps eliminated | `chrono` (28 crates) + `log` (2) + `instant` + `anyhow` (core) + `pollster` + `serde_yaml` + `libc` (akida-driver→rustix) + `sysinfo` (15 crates→toadstool-sysmon) + `caps` + `console` + `indicatif` + `figment` + `handlebars` + 23 phantom deps. S164: dep dedup (linfa/ndarray/mockall/env_logger). S166: `ed25519-dalek` (→BearDog RPC), `regex` (→`str::contains`), `parking_lot` (→`std::sync`). S169: `pyo3` (FFI), `gbm`, `linfa`, `hmac`, `indicatif` removed |
| Hardcoded primal names | **0** user-visible; **~425** intentional legacy-compat refs remain (env fallbacks, serde aliases, parse_type); all new code is capability-first per `CAPABILITY_BASED_DISCOVERY_STANDARD.md` v1.2 |
| `async-trait` migration | 5 crates migrated to native AFIT; remaining ~102 uses justified by `dyn Trait` dispatch; stale import removed (S163) |
| Wildcard re-exports | Narrowed in 13 crates (explicit `pub use` reduces recompilation cascade) |
| Hardcoded ports/localhost | 0 inline literals -- config constants + capability-based discovery |
| Hardware transport | Implemented | DRM display, V4L2 capture, serial — frame protocol + router |
| License | AGPL-3.0-or-later -- root LICENSE file + SPDX headers on all files |
| File size limit | All production files **< 400 lines** (S166: seven former monoliths split into module dirs) |
| Test concurrency | Unlimited parallelism (removed global throttle); zero `#[serial]`; test-time mDNS/TCP timeouts via `cfg!(test)`; zero fixed sleeps in non-chaos tests |
| Environment safety | All env-var tests use `temp_env` (thread-safe), zero `std::env::set_var` in tests |

---

## Hardware Capabilities

ToadStool discovers and exposes compute substrates. All math dispatch belongs to barraCuda.

### GPU Discovery and Probing

- **Multi-adapter selection** -- `TOADSTOOL_GPU_ADAPTER` env var (index, name substring, or `auto` for best f64 GPU)
- **Detailed adapter info** -- `GpuAdapterInfo` exposes driver, f64 support, workgroup limits, max buffer size for barraCuda's driver profiling
- **Cross-vendor** -- NVIDIA, AMD, Intel via WGPU/Vulkan; zero CUDA, zero ROCm
- **Cache-aware tiling** -- discovers L2/Infinity Cache sizes for optimal workload tiling

| Substrate | Largest Cache | Optimal Tile | Impact |
|-----------|---------------|--------------|--------|
| RTX 3090 | L2: 6 MB | 1 MB | 732 tiles/GB |
| RTX 4070 | L2: 48 MB | 11 MB | 92 tiles/GB |
| RX 6950 XT | Infinity: 128 MB | 29 MB | 35 tiles/GB |
| CPU (Zen 3) | L3: 32 MB | 7 MB | 138 tiles/GB |

### NPU Discovery and Dispatch

- **Generic `NpuDispatch` trait** -- vendor-agnostic neuromorphic compute interface
- **`AkidaNpuDispatch` adapter** -- Akida NPU via VFIO/kernel/mmap backends
- **`NpuParameterController` trait** -- NPU-driven autonomous parameter tuning (absorbed from hotSpring)
- **Capabilities**: inference, reservoir computing, on-chip learning, spiking networks, batch inference, power monitoring

### Distributed Workload Dispatch

- Cross-gate GPU routing across machines
- Distributed LLM inference (TinyLlama-1.1B: 39.85 tok/s across two gates with security-service encrypted tensor transport)
- Cloud cost estimation, compliance validation, federation

---

## Architecture

```
Applications (hotSpring, NUCLEUS inference, etc.)
       |
BarraCUDA (separate primal — ecoPrimals/barraCuda/)
  Math dispatch, shaders, precision strategy
  Consumes toadStool's hardware capabilities via IPC
       |
ToadStool: Hardware Discovery + Orchestration (THIS REPO)
  JSON-RPC 2.0 + tarpc IPC (Unix sockets)
  GPU/NPU/CPU discovery and capability probing
  GpuAdapterInfo (driver, f64, workgroups, buffer limits)
  NpuDispatch trait (generic neuromorphic compute)
  GPU Job Queue + Cross-Gate Routing
  Capability-based runtime discovery (self-knowledge only)
  Cloud cost / compliance / federation
  Shared error tracking (AtomicU64)
       |
  +--------+---------+--------+
  |        |         |        |
 GPU     GPU       GPU      NPU         CPU
 RTX    RTX 3090  RX 6950  Akida       WGPU
 4070   (NVIDIA)  XT (AMD) (inference)  software
(NVIDIA)                                rasterizer
       |
Hardware Transport Layer
HDMI Tx    V4L2 Rx    Serial     TransportRouter
(DRM)      (Capture)   (USB)     (any-to-any)
```

**Routing**: `Device::select_for_workload(&hint)` auto-routes to the optimal device. `Device::select_with_preference(Some(Device::CPU), &hint)` lets callers override. Auto-routing is smart; user choice is sovereign.

### IPC Architecture

- **Unix sockets** for all primal-to-primal communication
- **JSON-RPC 2.0** protocol with semantic method naming (`{domain}.{operation}[.{variant}]`)
- **tarpc** (0.34) for high-performance typed RPC
- **Capability-based discovery** -- `get_socket_path_for_capability()` replaces all name-based lookup
- **biomeOS socket standard**: `/run/user/$UID/biomeos/{primal}.sock`
- **Multi-family support**: `--family-id` flag for `toadstool-{family_id}.sock`
- **Self-knowledge principle**: ToadStool only knows its own identity (`PRIMAL_NAME`); external primals are discovered via capability, not name
- **Storage service integration** -- real JSON-RPC `storage.artifact.store`/`retrieve` with graceful fallback
- **Real-time events**: `compute.status` JSON-RPC polling or biomeOS/coordination service for event streaming

### JSON-RPC Methods (~67 dynamically built; S186)

Surface trimmed to hardware orchestration and IPC boundaries. **Removed from this repo** (S169): `inference.*` / Ollama-style AI (→ intelligence service), **`shader.compile.*`** (→ visualization service), **`science.*`** / **`ecology.*`** / **`discovery.*`** / **`deploy.*`** relays (→ orchestration and peers). **Kept**: **`shader.dispatch`** (dispatch compiled binary to GPU; compile happens in visualization service).

| Domain | Methods | Notes |
|--------|---------|-------|
| `toadstool.*` | `health`, `version`, `query_capabilities` | Canonical namespace |
| `toadstool.resources.*` | `estimate`, `validate_availability`, `suggest_optimizations` | Canonical namespace |
| `resources.*` | `estimate`, `validate_availability`, `suggest_optimizations` | biomeOS neural API routing aliases |
| `compute.*` | `health`, `version`, `capabilities`, `discover_capabilities`, `submit`, `status`, `result`, `cancel`, `list` | biomeOS Node Atomic aliases + GPU queue |
| `ai.*` | `local_inference`, `local_execute` | biomeOS ai_local capability |
| `ai.nautilus.*` | `status`, `observe`, `train`, `predict`, `screen`, `edges`, `shell.export`, `shell.import` | Evolutionary reservoir computing (feature-gated `nautilus`) |
| `gpu.*` | `gpu.query_info`, `gpu.query_memory`, `gpu.query_telemetry` | Hardware info |
| `gate.*` | `update`, `remove`, `list`, `route` | Distributed routing |
| `transport.*` | `discover`, `list`, `route` | Hardware transport discovery + routing |
| `shader.dispatch` | — | Sovereign pipeline: compiled binary in (base64 / array / `compile_result`), GPU via VFIO/DRM, optional readback |
| `provenance.query` | cross-spring flow matrix | (was `toadstool.provenance`; deprecated alias retained) |
| *Peer primals* | — | **Compile** (`shader.compile.*`), **LLM/inference**, **science / ecology / discovery / deploy** — call intelligence, visualization, orchestration, coordination services over capability sockets (not re-exported here) |

---

## Quick Start

```bash
# Build everything
cargo build --release

# Run all quality gates
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo doc --workspace --no-deps
cargo test --workspace --lib

# Per-crate coverage
cargo llvm-cov --lib -p toadstool-common --json
```

---

## Project Structure

```
toadStool/
+-- crates/
|   +-- toadstool-core/            Generic hardware traits (NpuDispatch, HardwareTransport, TransportRouter)
|   +-- core/
|   |   +-- common/                Shared types, constants, primal identity, ecosystem IDs, error types
|   |   +-- config/                Centralized configuration (env-aware, network config, port constants)
|   |   +-- toadstool/             Core runtime, IPC, scheduler, production hardening
|   |   +-- glowplug/              Hardware-agnostic device lifecycle (personality, swap, discovery)
|   |   +-- ember/                 Hardware-agnostic device holder (resources, journals, metadata)
|   |   +-- hw-safe/               Safe wrappers for hardware primitives (mmap, volatile MMIO, aligned alloc)
|   |   +-- nvpmu/                 NVIDIA PMU BAR0 access, DMA, VFIO
|   +-- server/                    JSON-RPC server, GPU job queue, cross-gate router
|   +-- (api/ fossilized S96 — ByobApi extracted to container, remainder to ecoPrimals/fossil/)
|   +-- cli/                       UniBin CLI (single binary, BYOB server subcommand)
|   +-- integration/               Inter-primal protocols (security, storage, coordination)
|   +-- distributed/               Multi-gate coordination, cloud cost/compliance/federation
|   +-- runtime/
|   |   +-- gpu/                   WGPU device management, unified memory, pinned memory
|   |   +-- universal/             Universal compute substrate (CPU backends, GpuAdapterInfo)
|   |   +-- adaptive/              Adaptive optimization, GPU fingerprinting
|   |   +-- display/               DRM/KMS backend + Hardware Transport (HDMI/capture/serial)
|   |   +-- edge/                  Edge device discovery (mDNS, filesystem), serial/TCP comms
|   |   +-- wasm/                  WebAssembly runtime (wasmi)
|   |   +-- container/             BYOB container runtime
|   +-- neuromorphic/              NPU drivers (Akida VFIO/kernel/mmap backends)
|   +-- ml/                        burn-inference (BERT, Whisper, Vision stubs with gated errors)
|   +-- security/                  Sandbox, policies, monitoring
|   +-- testing/                   Chaos, fault, property-based testing (proptest)
|   +-- management/                Analytics, monitoring, resources (real ResourceManager with toadstool-sysmon)
+-- (fossils at ecoPrimals/infra/wateringHole/fossilRecord/)
+-- showcase/                      Demos (RBF, neuromorphic, GPU, FHE)
+-- docs/                          Architecture, guides, audits, ADRs
+-- specs/                         Technical specifications
```

---

## Code Quality

### Deep Debt Principles

1. **Hardware layer, not math layer** -- ToadStool discovers and probes compute substrates; barraCuda dispatches math
2. **Modern idiomatic Rust** -- parameter-based APIs, zero global state mutation, thiserror 2.0
3. **Capability-based discovery** -- self-knowledge principle: only `PRIMAL_NAME` is known; everything else discovered at runtime via `get_socket_path_for_capability()`
4. **Zero-copy hot paths** -- `Cow<'a, str>` with `#[serde(borrow)]` on JSON-RPC types, `serde_json::from_slice`, `bytes::Bytes` on binary payloads
5. **No hardcoding** -- ports, hostnames, and primal names all config-driven or capability-discovered
6. **Mocks isolated to testing** -- all `#[cfg(test)]` gated (including `InMemoryAuthBackend`); production code is complete implementations or typed errors
7. **Honest documentation** -- no aspirational claims as facts; ML stubs return `ModelNotLoaded`/`ModelBackendRequired`
8. **Vendor-agnostic** -- WGPU/Vulkan for GPU discovery, any vendor works
9. **Sovereign compute** -- no vendor lock-in, pure Rust core
10. **100% unsafe documentation** -- every `unsafe` block has `// SAFETY:` comments (~36 blocks, all justified; 25 in containment zones)
11. **Shared error tracking** -- `AtomicU64` counter across all server transports

### Quality Metrics

| Metric | Value |
|--------|-------|
| Clippy pedantic warnings | 0 (workspace-wide `clippy::pedantic` clean; `#[expect]` evolution S131+) |
| Doc warnings | 0 |
| Build warnings | 0 |
| Workspace tests | **21,514** (S191), 0 failures |
| Full workspace test time | ~2m30s (unlimited parallelism, `cfg!(test)` fast timeouts; GPU crates have NVK resilience wrappers) |
| `unsafe` blocks | **~66 actual** (all in hw-safe/GPU/VFIO/display containment crates); SAFETY-documented; **41 crates forbid, 6 deny** `unsafe_code` |
| Production panics/unwraps | 0 blind `unwrap()`; infallible `expect()` only |
| Production `Box<dyn Error>` | 0 in core crates -- all typed errors (thiserror) |
| Production stubs | 0 blind stubs; test-only mocks **`#[cfg(test)]`** only |
| Production `todo!()`/`unimplemented!()`/`dbg!()` | 0 |
| Production FIXME / HACK | 0 |
| Dead code removed | ~400+ lines (REST handlers, middleware, dead modules); ~25 justified `#[allow(dead_code)]` remain |
| Hardcoded localhost/ports/URLs in prod | 0 -- config constants + capability-based discovery |
| External deps eliminated | `chrono`, `log`, `instant`, `anyhow` (core), `pollster`, `serde_yaml`, `libc`, `sysinfo`, `caps`, `console`, `indicatif`, `figment`, `handlebars` + 23 phantom deps. S164: dep dedup (linfa/ndarray/mockall/env_logger). S166: `md5`→`md-5`, `bollard` 0.18, `ed25519-dalek` (core+CLI→BearDog RPC), `regex` (→`str::contains`), `parking_lot` (→`std::sync`). S169: `pyo3`, `gbm`, `linfa`, `hmac`, `indicatif` |
| Default test timeout | 5s (unit: 2s, integration: 30s, chaos: 20s) |
| Hardware transports | 3 | Display (DRM), Capture (V4L2), Serial (feature-gated) |

---

## Evolution

**We are still evolving.** barraCuda (separate primal) owns all math and shaders. ToadStool focuses on hardware discovery, capability probing, and workload orchestration. All 5 spring handoffs absorbed.

### Active / Next
- **Test coverage** -- pushing toward 90% target; 21,512 tests (S188); ~80-85% lib-only line (185K lines instrumented); remaining gap: hardware-dependent paths, specialty runtimes
- **DF64 / ComputeDispatch** -- transferred to barraCuda team (S93); toadStool serves hardware capabilities
- **Sovereign compiler Phase 4+** -- register pressure estimation, loop software pipelining (barraCuda)

### Recently Completed
- **S188 (Apr 5, 2026)**: Cross-primal doc cleanup — capability-based language in 61 files; cross-primal references 550→425; remaining are all backward-compatibility (serde aliases, env var fallbacks, interned constants). Full audit: unsafe at FFI boundaries only, thread::sleep at hardware boundaries only, production mocks fully gated, all files <700L.
- **S185-186 (Apr 5, 2026)**: Unsafe evolution — full audit of all ~90 unsafe sites. Removed 4 redundant `unsafe impl Send/Sync` (LockedMemory, Bar0Access). Evolved akida-driver DMA to `OwnedFd`. Centralized ioctl dispatch into `do_ioctl` helpers (VFIO, V4L2, DRM). Generic `read_reg<T>`/`write_reg<T>` for volatile MMIO. Created `ExclusivePtr` newtype (auto-derives Send+Sync for memory types, -6 unsafe impls). Created `ContiguousBytes` trait (centralizes `from_raw_parts` into 2 blocks, -6 unsafe blocks). Evolved nvpmu DMA from `RawFd` to `OwnedFd`. **Net: ~59→~36 unsafe blocks, ~20→~15 unsafe impls.** All remaining unsafe pinned for ecosystem evolution (coralReef/barracuda replace CUDA/OpenCL). Zero behavioral changes.
- **S175 (Apr 3, 2026)**: Unsafe code reduction Phase 1+2. V4L2 ioctls extracted to `v4l2::ioctl` containment module (device.rs now pure safe Rust). GPU backend constructors (`with_device`/`with_context`) made safe. 6 `unsafe impl Send/Sync` consolidated into single `GpuPtr` newtype. `HugePageMemory` RAII type in hw-safe absorbs raw mmap/mlock from nvpmu/dma.rs. Consumer `unsafe {}` blocks reduced 80% (56→11). Total: 59 actual unsafe blocks (48 in containment zones). 0 clippy warnings.
- **S174 (Apr 3, 2026)**: Unsafe audit — deleted duplicate `VolatileSlice` from akida-driver (replaced with `hw-safe::VolatileMmio`). Added `dma_map_fd`/`dma_unmap_fd` safe wrappers in hw-safe. Net −10 unsafe blocks (89→79 grep, 77 actual). All quality gates green.
- **S172-5 (Apr 2, 2026)**: Capability-based discovery compliance per primalSpring audit. Evolved ~105 foreign primal references across 40 files: `ServiceDomainsConfig` fields (`songbird`→`coordination`, `beardog`→`security`, `nestgate`→`storage`, `squirrel`→`ai_processing`); `SOCKET_FILENAME` from `beardog.sock`→`security.sock` (Tier 3 capability-domain socket); `EndpointConfig` fields; `EcosystemServices` fields; `PrimalCapabilitiesConfig` fields; all with `#[serde(alias)]` for backward compat and legacy env var fallbacks. 0 clippy warnings, 0 doc warnings, 0 test failures.
- **S172 (Apr 2, 2026)**: Deep debt evolution plan (6 phases). Created `CapabilityDomain` enum (7 variants, replaces ~30 hardcoded primal name sites). Created `LockedMemory` RAII type in hw-safe. Typed ioctl wrappers in nvpmu/vfio.rs. BYOB health monitoring wired to background task. Replaced hand-rolled mmap with `memmap2` (eliminated 4 unsafe blocks). Smart-refactored 3 large files into submodules. +55 tests across hw_learn and transport handlers. Sysfs path discovery via toadstool-sysmon. Feature-gated embedded placeholders.
- **S171 (Apr 1, 2026)**: Ember absorption + unsafe evolution. Created `toadstool-hw-safe` (unsafe containment zone), `toadstool-glowplug`, `toadstool-ember` crates. Rewrote `GpuFirmwareProxy` → `GpuFirmwareAccess` (direct BAR0 reads, zero coral-ember dependency). Evolved `glowplug_client.rs` to toadStool-native sysfs-based service. Migrated mmap/alloc to hw-safe across akida-driver, nvpmu, gpu. All ~400 distributed crate missing_docs resolved. Hardcoding evolved (bind address, gate ID, configurator constants).
- **S170 (Mar 31, 2026)**: Concurrent test evolution. Fixed 16+ stale test failures, eliminated test sleeps, IPC compliance verification.
- **S169 (Mar 31, 2026)**: Primal overstep cleanup — removed Ollama, HTTP stack from server+cli (Songbird), shader **compile** proxy (coralReef), science/ecology/deploy relays (biomeOS); dropped pyo3/gbm/linfa/hmac/indicatif; **`shader.dispatch`** retained. Deep debt: capability-only `ports`/`network`, `InMemoryAuthBackend` test-only, `embedded/types` split, XDG + `temp_dir`, Unix-socket-first discovery, workspace dep inheritance, federation unwrap audit.
- **S168 (Mar 30, 2026)**: `shader.dispatch` JSON-RPC method — closes ludoSpring V35 / coralReef Iter 70 E2E gap (compile→dispatch→readback sovereign shader pipeline). Base64 + u8-array + compile_result input. Workspace-wide clippy zero-warning (~120+ fixes). Auth backend evolved to async-first. Server connection zero-copy. 11 specialty runtime files from 0% → covered. 18 new shader dispatch tests.
- **S166 (Mar 29, 2026)**: Deep debt execution + capability-based evolution + dependency sovereignty. Redundant `#![allow]` cleanup (29 `lib.rs` files; blanket `clippy::nursery` removed from server + cross-substrate-validation). Discovery: hardcoded primal names → capability IDs (`crypto`, `coordination`, `storage`, `routing`); `resolve_capability_socket_fallback()`; legacy names `#[deprecated]`; `ecosystem::capabilities` module. Production stubs: `crypto_lock` JSON permissions + delegation validation; `SubstrateConfig::validate()` (power budget, fallback order, capabilities). Smart-split 7 large files into module dirs (all production **< 400 lines**). `md5` → `md-5`; `bollard` 0.18 workspace-wide; orchestrator provider selection intersects compliance + sorts deterministically. **Dependency sovereignty**: `ed25519-dalek` removed from core + CLI (signing/verification delegated to BearDog via `crypto.sign`/`crypto.verify`/`crypto.public_key` JSON-RPC); `regex` removed (replaced with `str::contains()` pattern matching); `parking_lot` replaced with `std::sync::RwLock` in orchestration; `AuthBackend` trait extended with `sign_payload()` + `public_key()`; HTTP transport delegates to Songbird via `comms.http_forward` JSON-RPC; `hmac` removed; `mdns-sd` retained as feature-gated cold-start discovery. Net: 123 files changed, +1145/-8334 lines.
- **S164 (Mar 29, 2026)**: Dependency deduplication (linfa 0.7→0.8, ndarray 0.15→0.16, mockall 0.11→0.12, env_logger 0.10→0.11). Smart-refactored 5 large files into directory modules (execution.rs, capabilities.rs, client.rs, ecosystem/mod.rs, integration_impl.rs). +94 new tests across 7 lowest-coverage files (resource_validator 20→75%, discovery 57→88%, scheduler/execution 45→99%, orchestrator 43→100%, ecosystem 68→85%, client/core 54→85%, dispatch 40→70%). All quality gates green.
- **S163+ (Mar 22, 2026)**: GlowPlug ember client — device lifecycle RPCs (`ember.list`, `ember.status`, `ember.swap`, `ember.reacquire`). Initially a coral-ember proxy, evolved to toadStool-native in S171.
- **S163 (Mar 21, 2026)**: Deep code quality + dependency audit. 26 phantom deps removed across 10 crates (indicatif, figment, handlebars, nom, byteorder, csv, rand + 19 more). Zero-copy improvements: PrimalIdentity trait returns references not clones, PluginManager returns `&str` not `String`, protocol handler map uses `Arc<str>` keys, JSON payload serialization bypasses intermediate String. `#[allow(dead_code)]` evolved to `#[expect(dead_code, reason)]` or `#[cfg(test)]`. RUSTSEC-2025-0119 advisory eliminated (indicatif removed). Stale async-trait import removed. Clippy zero warnings. All tests pass.
- **S162 (Mar 21, 2026)**: Coverage expansion (81.64%→82.81%). +98 tests across barracuda, science domains, dispatch, transport, hw_learn, tarpc, unibin. Coverage script fix. SPDX sweep (38 files). Last production unwrap evolved.
- **S160 (Mar 20, 2026)**: Deep execution + coverage expansion. 9 broken tests fixed (neuromorphic detection, nested-runtime, transport assertions). +49 new tests across resource types, security policies/sandbox types, property-based testing. Hardcoded Akida specs → named constants. Dead `procfs` dep removed from 3 crates. 21,275 tests, 0 failures.
- **S159 (Mar 18, 2026)**: Deep audit and execution. All 694+ missing_docs warnings filled across 58 crates. Clippy `--workspace -D warnings` passes (0 errors). 3 compilation errors fixed (MockNpuDispatch, Arc<str>/String, paths module). JSON-RPC methods standardized to `domain.verb`. Hardcoded localhost evolved to named constants. Primal names → capability-based in auto_config. Zero-copy expanded (workload IDs → Arc<str>). Production stubs eliminated. All unsafe env::set_var → temp_env. Nested-runtime anti-patterns fixed.
- **S155b (Mar 15, 2026)**: Coverage expansion, clippy pedantic clean, dependency audit clean, unsafe audit clean. 21,156 tests. ~83% line coverage (182K lines instrumented). Next: push 83%→90% (hardware mocks), property-based testing for computation modules, multi-primal integration test infrastructure.
- **S147 (Mar 12, 2026)**: hw-learn wiring + sovereign compute hardening — 5 `compute.hardware.*` JSON-RPC methods (observe/distill/apply/share_recipe/status), nvpmu `Bar0Access` implements `RegisterAccess` trait (hw-learn→BAR0 bridge), `nvvm_safety.rs` renamed to `spirv_codegen_safety.rs` (naga root-cause per hotSpring v0.6.30), `FirmwareInventory` in `gpu.info` response (compute_viable/compute_blockers), PRIMAL_REGISTRY updated, genomeBin manifest enriched. Spring pins: hotSpring v0.6.30, neuralSpring V98/S145, coralReef Iter 35. 20,015 tests.
- **S146 (Mar 10, 2026)**: Deep evolution — nvvm_transcendental_risk in gpu.info, PrecisionBrain wired into compile_wgsl_multi, PcieTopologyGraph stable, SpringDomain +10 domains (SCREAMING_SNAKE_CASE), HealthSpring added, gpu_memory_estimate_bytes + route_with_vram, cross-spring provenance expanded. Spring pins: hotSpring v0.6.27, neuralSpring V96, wetSpring V109, healthSpring V19. 19,972 tests.
- **S145 (Mar 10, 2026)**: Spring absorption execution — PrecisionBrain (hotSpring v0.6.25), NvkZeroGuard (airSpring v0.7.5), 8 new WorkloadPatterns (neuralSpring S140 + healthSpring V14.1), 5 new capability domains, capability.call format standardization (ISSUE-003), Spring-as-Provider ProviderRegistry (ISSUE-007), ServerConfig port hardcoding evolved. 19,965 tests.
- **S144 (Mar 9, 2026)**: PCIe switch topology, deprecated API migration, dead code audit, coralReef multi-device compile, topology-aware MultiGpuPlacement. Flaky test fix, clippy pedantic sweep, hot-path clone elimination.
- **S140 (Mar 9, 2026)**: Deep debt evolution — hardcoding elimination, StreamingDispatchContext, barraCuda Sprint 2 API awareness, smart refactor science.rs (1139→828 LOC).
- **S134 (Mar 8, 2026)**: BearDog crypto delegation enforced. `dev-crypto` feature gate. Volatile MMIO safe abstraction. Property-based tests (proptest).
- **S133 (Mar 8, 2026)**: Cross-spring absorption from all 5 springs. Ada Lovelace reclassification, f64_zeros_risk, 14 ecology methods, NUCLEUS discovery, 20 semantic methods (71→91).
- **S130+ (Mar 7, 2026)**: Clippy pedantic zero workspace-wide. Unsafe audit. 240 new coverage tests. CI pedantic gate.
- **S128 (Mar 6, 2026)**: f64 shared-memory bug absorbed. `PrecisionRoutingAdvice`. `shader.compile.*` IPC.
- **S90–S96**: REST API deleted. barraCuda budded. Sovereign pipeline. NpuDispatch. ecoBin verified.

See [CHANGELOG.md](CHANGELOG.md) for full session-by-session detail.

---

## Active Debt (toadStool)

| ID | Description | Status |
|----|-------------|--------|
| D-COV | Test coverage → 90% | Active -- 21,514 tests (S191); ~80-85% lib-only line (185K instrumented); remaining gap: hardware paths |
| D-S20-003 | ~~neuralSpring `evolved/` migration~~ | **RESOLVED** -- neuralSpring V89 completed; `evolved/` removed |
| D-S18-002 | ~~cubecl transitive `dirs-sys`~~ | **RESOLVED** -- cubecl removed; dirs-sys only via wasmtime-cache (feature-gated) |

### Resolved (S94b)

| ID | Description |
|----|-------------|
| D-NPU | `NpuDispatch` trait + `AkidaNpuDispatch` adapter implemented |
| D-SOV | All 7 production callers migrated to capability-based discovery |

### Re-implemented (S95)

| ID | Description |
|----|-------------|
| management/resources | Real ResourceManager with toadstool-sysmon (pure Rust /proc, replaces sysinfo S137) |

### Transferred to barraCuda Team (S93)

| ID | Description |
|----|-------------|
| D-CD | ComputeDispatch migration (~139 remaining ops) |
| D-DF64 | DF64 as default precision path |
| W-001 | f64 transcendental polyfill (28 functions -- COMPLETE) |
| W-003 | NAK compiler scheduling gap (SM70 Volta) |

See [DEBT.md](DEBT.md) for full register and evolution paths.

---

## Documentation

| Document | Purpose |
|----------|---------|
| [DOCUMENTATION.md](DOCUMENTATION.md) | Navigation hub (guides, specs, audits) |
| [CHANGELOG.md](CHANGELOG.md) | Full session-by-session evolution history |
| [DEBT.md](DEBT.md) | Active debt register, workarounds, evolution paths |
| [NEXT_STEPS.md](NEXT_STEPS.md) | Roadmap and upcoming work |
| [CONTEXT.md](CONTEXT.md) | Public surface summary |

**Fossil record** — Session trackers archived under `ecoPrimals/infra/wateringHole/fossilRecord/toadstool/` (S166 snapshot): `TOADSTOOL_STATUS_S166.md`, `TOADSTOOL_EVOLUTION_TRACKER_S166.md`, `TOADSTOOL_QUICK_REFERENCE_S166.md`, `TOADSTOOL_SOVEREIGN_COMPUTE_S166.md`, `TOADSTOOL_SPRING_ABSORPTION_TRACKER_S166.md`, `TOADSTOOL_BREAKING_CHANGES_S166.md`, `UNSAFE_AUDIT_REPORT_S166.md`, `SOVEREIGN_COMPUTE_GAPS_S166.md`, `PURE_RUST_TRACKING_S166.md`.

---

**Last Updated**: April 8, 2026 — S191. **21,514** workspace tests, 0 failures. ~80-85% lib-only line coverage (target 90%). **~67 JSON-RPC methods** with **Wire Standard L3** (cost_estimates + operation_dependencies). AGPL-3.0-or-later. Zero C FFI deps (ecoBin v3.0). **~66 unsafe blocks** (all in hw-safe/GPU/VFIO/display containment crates); **41 crates forbid, 6 deny** `unsafe_code`. IPC-first JSON-RPC (Unix sockets). Capability symlinks (`compute.sock`). Rust 1.85+ (edition 2024, MSRV). **S191**: Wire Standard L3 cost model (energy/time/compute, 55+ methods). **S190**: Wire Standard L2 compliance. **S189**: GAP-MATRIX-05 server docs resolved. **Capability-based discovery compliant** per `CAPABILITY_BASED_DISCOVERY_STANDARD.md` v1.2.

---

Part of [ecoPrimals](https://github.com/ecoPrimals) — sovereign compute for science and human dignity.
