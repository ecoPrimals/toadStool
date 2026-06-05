# ToadStool

**Sovereign Compute Hardware** | Pure Rust | ecoBin | Jun 2026 | S294 | v0.2.0

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

**Deployment**: Tower starts first (security service → coordination service), then ToadStool. Sockets: `$XDG_RUNTIME_DIR/biomeos/compute.sock` (JSON-RPC) + `compute-tarpc.sock` (tarpc). ToadStool discovers other primals at runtime by capability, not by name.

---

## Quality Gates

| Gate | Status |
|------|--------|
| `cargo build --workspace` | Clean |
| `cargo fmt --all -- --check` | 0 diffs |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 warnings |
| `cargo doc --workspace --no-deps` (RUSTDOCFLAGS="-D warnings") | 0 warnings |
| `cargo test --workspace` | **23,000+ tests, 0 failures** (8,895+ lib-only default; +1,289 behind `legacy-coordination`), **~222** ignored (hardware-gated); full workspace ~7m |
| Doctests | All passing (common, core, server, cli, testing, display) |
| Standalone clone test | Pull to any machine, `cargo test` works (GPU-optional, CPU fallback, device-lost resilient) |
| `unsafe` blocks | **46 actual** (all in hw-safe/GPU/VFIO/display/plugin containment crates); **all SAFETY-documented** (confirmed S288); workspace `unsafe_code = "deny"`, **41 crates `forbid`** + 5 hw crates with narrow `#[allow(unsafe_code, reason)]`; **all lint attrs have `reason =`** |
| Production panics/unwraps | **0** production `unwrap()` / `expect()` / `panic!()` (S282–S290: all paths evolved to Result; diagnostic `write!` expects → `let _ = write!()`, S290) |
| Production stubs / test mocks | Stubs evolved to real implementations or typed errors (`NoProviderRegistered`, `NoEngineRegistered`); **embedded-placeholder** opt-in via `embedded-placeholder-impls` feature (S285 — removed from default features); **auth test mocks** (`InMemoryAuthBackend`) isolated under **`#[cfg(any(test, feature = "test-mocks"))]`**; **`test-mocks` removed from default features** (S206 — production builds exclude mock code) |
| Production `Box<dyn Error>` | 0 in core crates -- all typed errors (thiserror) |
| Production TODOs / FIXME / HACK | 0 in production code |
| Dead code | ~400+ lines removed (REST, middleware, dead modules); **zero production `#[allow]`** (S291 — all converted to `#[expect]` or deleted; ~13 test-only `#[allow]` remain) |
| External deps eliminated | `chrono` (28 crates) + `log` (2) + `instant` + `anyhow` (core) + `pollster` + `serde_yaml` + `libc` (akida-driver→rustix) + `sysinfo` (15 crates→toadstool-sysmon) + `caps` + `console` + `indicatif` + `figment` + `handlebars` + 23 phantom deps. S164: dep dedup (linfa/ndarray/mockall/env_logger). S166: `ed25519-dalek` (→security service RPC), `regex` (→`str::contains`), `parking_lot` (→`std::sync`). S169: `pyo3` (FFI), `gbm`, `linfa`, `hmac`, `indicatif` removed. S288: `modbus` (feature-gated `modbus-transport`, not default). S289: `bollard` (feature-gated `docker`, not default). S292: `serialport` (feature-gated `serial-transport`, not default). S293: `tarpc` removed from display (unused), made optional in protocols (`tarpc-transport`) |
| Hardcoded primal names | **0** user-visible; install/launcher paths use `PRIMAL_BINARY_NAME` (S292); **~400** intentional legacy-compat refs remain (env fallbacks, serde aliases, parse_type); all new code is capability-first per `CAPABILITY_BASED_DISCOVERY_STANDARD.md` v1.2 |
| `async-trait` migration | **DEPRECATED** — fully removed and banned in `deny.toml` (S203r). **Stadial parity gate cleared (S203s)**: ~32 traits converted from `dyn` dispatch to **enum dispatch + RPITIT**. Zero finite-implementor `dyn` remaining. |
| Wildcard re-exports | Narrowed in 13 crates (explicit `pub use` reduces recompilation cascade) |
| Hardcoded ports/localhost | 0 inline literals -- config constants + capability-based discovery |
| Hardware transport | Implemented | DRM display, V4L2 capture, serial — frame protocol + router |
| JSON-RPC surface | **111** JSON-RPC methods (direct) + semantic registry |
| License | AGPL-3.0-or-later -- root LICENSE file + SPDX headers on all files |
| File size limit | Non-hardware production files target **< 500 lines**. **0 production files >800L** (S284 split last 3: `sovereign_init`, `open_vfio`, `experiment`); test-only files in `tests/` directories may exceed limit. S278+S284 split oversized production files into module dirs. |
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

- **Dual-socket pattern** (separate sockets, separate protocols):
  - `compute.sock` — JSON-RPC 2.0 primary (biomeOS routes here; universal entry point)
  - `compute-tarpc.sock` — tarpc hot-path (Rust-to-Rust peers; optional performance channel)
  - Override: `TOADSTOOL_SOCKET` / `TOADSTOOL_TARPC_SOCKET` env vars
  - Family: `compute-{family_id}.sock` / `compute-{family_id}-tarpc.sock` via `--family-id`
- **JSON-RPC 2.0** protocol with semantic method naming (`{domain}.{operation}[.{variant}]`)
- **tarpc** (0.37) for high-performance typed RPC between Rust peers
- **Capability-based discovery** -- `get_socket_path_for_capability()` replaces all name-based lookup
- **Self-knowledge principle**: ToadStool only knows its own identity (`PRIMAL_NAME`); external primals are discovered via capability, not name
- **Storage service integration** -- real JSON-RPC `storage.artifact.store`/`retrieve` with graceful fallback
- **Real-time events**: `compute.status` JSON-RPC polling or biomeOS/coordination service for event streaming

### Health Probe Timeouts (PG-62)

Callers probing `health.liveness` should use a timeout of **≥3 seconds** (recommended: 5s for composition startup). During initialization, `health.liveness` returns `{"status":"starting"}` until the server is fully ready (discovery registered, biomeOS scanned), then transitions to `{"status":"alive"}`. The socket accepts connections immediately upon listener bind — before executor initialization completes — so callers receive a fast response even during cold start. If BTSP handshake is required, add its budget (5s default, overridable via `BTSP_HANDSHAKE_TIMEOUT_SECS`).

| Probe | During init | After ready |
|-------|-------------|-------------|
| `health.liveness` | `{"status":"starting"}` | `{"status":"alive"}` |
| `health.readiness` | `{"status":"starting","version":"..."}` | `{"status":"ready","version":"..."}` |
| `health.check` | Full envelope (always `"alive"`) | Full envelope |

### Dispatch Timeouts

`compute.dispatch.submit` and `shader.dispatch` accept an optional `timeout_ms` parameter. Defaults:

| Constant | Value | Override |
|----------|-------|----------|
| `DISPATCH_DEFAULT_TIMEOUT` | 5,000 ms | `timeout_ms` in request params |
| `WORKLOAD_EXECUTION_TIMEOUT` | 300 s (5 min) | `TOADSTOOL_EXECUTION_TIMEOUT` env |
| `TCP_IDLE_TIMEOUT` | 300 s | `TOADSTOOL_TCP_IDLE_TIMEOUT_SECS` env |

For GPU workloads, callers should set `timeout_ms` proportional to expected computation time. The 5s default is appropriate for small shader dispatches; large GPU jobs should pass explicit timeouts.

### IPC Contract: Pre-Resolved Values

All JSON-RPC methods expect **pre-resolved** parameter values. The server does **not** perform `${VAR}`/`$VAR` environment variable expansion on any string fields. Env expansion is a **CLI-only** convenience in `load_workload_file` for locally-authored TOML/JSON specs. IPC callers must send fully resolved paths, identifiers, and metadata values. In cross-primal composition, the server's process env differs from the caller's — implicit expansion would create ambiguity. Graph specs and composition callers should pre-expand variables on the client side. See `crates/server/src/pure_jsonrpc/METHODS.md` for full details.

### JSON-RPC Methods (111 direct + semantic registry; S286+ adds `dispatch.verify_trust`, `dispatch.telemetry.schema`)

Surface trimmed to hardware orchestration and IPC boundaries. **Removed from this repo** (S169): `inference.*` / Ollama-style AI (→ intelligence service), **`shader.compile.*`** (→ visualization service), **`science.*`** / **`ecology.*`** / **`discovery.*`** / **`deploy.*`** relays (→ orchestration and peers). **Kept**: **`shader.dispatch`** (dispatch compiled binary to GPU; compile happens in visualization service).

| Domain | Methods | Notes |
|--------|---------|-------|
| `toadstool.*` | `health`, `version`, `query_capabilities`, `validate`, `list_workloads` | Canonical namespace |
| `toadstool.resources.*` | `estimate`, `validate_availability`, `suggest_optimizations` | Canonical namespace |
| `resources.*` | `estimate`, `validate_availability`, `suggest_optimizations` | biomeOS neural API routing aliases |
| `compute.*` | `execute`, `health`, `version`, `capabilities`, `discover_capabilities`, `submit`, `status`, `result`, `cancel`, `list` | biomeOS Node Atomic aliases + GPU queue; `execute` = `toadstool.submit_workload` |
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
|   |   +-- edge/                  Edge device discovery (sysfs USB/BT, IPv6 procfs, mDNS), serial/TCP comms
|   |   +-- wasm/                  WebAssembly runtime (wasmi)
|   |   +-- container/             BYOB container runtime
|   +-- neuromorphic/              NPU drivers (Akida VFIO/kernel/mmap backends)
|   +-- ml/                        burn-inference (BERT, Whisper, Vision stubs with gated errors) (excluded from workspace — builds separately)
|   +-- security/                  Sandbox, policies, monitoring
|   +-- testing/                   Chaos, fault, property-based testing (proptest)
|   +-- management/                Analytics, monitoring, resources (real ResourceManager with toadstool-sysmon)
+-- (fossils at ecoPrimals/infra/wateringHole/fossilRecord/)
+-- (showcase/ fossilized S275 → fossilRecord/primals/toadStool/showcase_wave49/)
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
10. **100% unsafe documentation** -- every `unsafe` block has `// SAFETY:` comments (46 blocks, all justified; all in hw-safe/GPU/VFIO/display/plugin containment crates)
11. **Shared error tracking** -- `AtomicU64` counter across all server transports

### Quality Metrics

| Metric | Value |
|--------|-------|
| Clippy pedantic warnings | 0 (workspace-wide `clippy::pedantic` clean; `#[expect]` evolution S131+) |
| Doc warnings | 0 |
| Build warnings | 0 |
| Workspace tests | **23,000+**, 0 failures (8,895+ lib default; +1,289 legacy-coordination) |
| Lib-only line coverage | ~83.6% |
| Full workspace test time | ~7m (unlimited parallelism, `cfg!(test)` fast timeouts; GPU crates have NVK resilience wrappers) |
| `unsafe` blocks | **46 actual** (all in hw-safe/GPU/VFIO/display/plugin containment crates); **all SAFETY-documented**; workspace `unsafe_code = "deny"`, **41 crates `forbid`** + 5 hw crates with narrow `#[allow(unsafe_code, reason)]` |
| Production panics/unwraps | **0** production `unwrap()` / `expect()` / `panic!()` (S282–S290: all paths evolved to Result; diagnostic `write!` expects → `let _ = write!()`, S290) |
| Production `Box<dyn Error>` | 0 in core crates -- all typed errors (thiserror) |
| Production stubs | Typed error returns (`NoProviderRegistered`, `NoEngineRegistered`, etc.); test-only mocks **`#[cfg(test)]`** only |
| Production `todo!()`/`unimplemented!()`/`dbg!()` | 0 |
| Production FIXME / HACK | 0 |
| Dead code removed | ~400+ lines (REST handlers, middleware, dead modules); **~80** justified `#[allow]` remain (conditional compilation, deprecated compat) |
| Hardcoded localhost/ports/URLs in prod | 0 -- config constants + capability-based discovery |
| External deps eliminated | `chrono`, `log`, `instant`, `anyhow` (core), `pollster`, `serde_yaml`, **`libc`** (S281→S282: zero libc, all mmap/ioctl via rustix), `sysinfo`, `caps`, `console`, `indicatif`, `figment`, `handlebars` + 23 phantom deps. S164: dep dedup. S166: `ed25519-dalek`/`regex`/`parking_lot`. S169: `pyo3`, `gbm`, `linfa`, `hmac`, `indicatif`. S288: `modbus` (feature-gated `modbus-transport`). S289: `bollard` (feature-gated `docker`, not default) |
| Env centralization | **~98%** (~410+ env reads via `socket_env::` constants); <10 raw `env::var("...")` remaining (S282–S285) |
| Default test timeout | 5s (unit: 2s, integration: 30s, chaos: 20s) |
| Hardware transports | 3 | Display (DRM), Capture (V4L2), Serial (feature-gated) |

---

## Evolution

**We are still evolving.** barraCuda (separate primal) owns all math and shaders. ToadStool focuses on hardware discovery, capability probing, and workload orchestration. All 5 spring handoffs absorbed.

### Active / Next
- **Test coverage** -- pushing toward 90% target; 23,000+ tests (9,156+ lib); ~83.6% lib-only line (185K lines instrumented); remaining gap: hardware-dependent paths (VFIO, DRM, V4L2), specialty runtimes
- **Sovereign VFIO dispatch** -- NVIDIA VFIO PBDMA dispatch wired via QMD (S258–S259); `device.vfio.open` + `device.vfio.roundtrip` JSON-RPC endpoints live; e2e validated on Titan V (S263)
- **DF64 / ComputeDispatch** -- transferred to barraCuda team (S93); toadStool serves hardware capabilities
- **Sovereign compiler Phase 4+** -- register pressure estimation, loop software pipelining (barraCuda)
- **NUCLEUS crypto integration** -- compute payloads encrypted via Tower `crypto.encrypt`/`crypto.decrypt` (S205); **self-registration with coordination service** via `DISCOVERY_SOCKET` + `ipc.register` at startup (S207)

### Recently Completed
- **S294 (Jun 5, 2026)**: **Wave 79: UDS Compliance + Coverage Push** — Fixed P2 binary UDS compliance: `--socket` CLI arg now wired through to server bind (was dead code). Socket path resolution adds CLI override as precedence #0. `primal.announce` reports actual bound path. Coverage push: +57 tests (8,952 lib total). CallerContext extraction tests, handler glue tests (workload, resources, queries, state, compute), `RuntimeEngineDispatch` delegation tests. Mutual-auth support wired into `ConnectionTrustHints`. **8,952+ lib tests. Zero clippy.**
- **S293 (Jun 5, 2026)**: **Deep Debt X: tarpc Gating + Unwrap Purge + Cylinder Split + LEGACY Deprecation Tracing** — Removed unused `tarpc` from `runtime/display`. `tarpc` made optional in `integration/protocols` behind `tarpc-transport`. `mmu_oracle/capture.rs` (795L) smart-split into `capture/` module dir (bar0, types, walk). All production `unwrap`/`expect` purged: `cpu_resource.rs` degraded pool, `rm_trigger` ioctl, Akida deprecated MMIO wrappers removed, neuromorphic bin tools. 23 `LEGACY_*` env reads now emit `tracing::warn!` deprecation notices. **8,895+ lib tests. Zero clippy. Zero production panics.**
- **S292 (Jun 5, 2026)**: **Deep Debt IX: Feature Gates + Module Splits + Naming + SAFETY + Deprecated Cleanup** — `serialport` feature-gated (`serial-transport`, not default) in `runtime/edge`. `dispatch/device.rs` (781L) smart-split into `device/` module dir (vfio, gr_init, lifecycle). Hardcoded `"toadstool"` → `PRIMAL_NAME`/`PRIMAL_BINARY_NAME` constants. Deprecated `TestExecutor`/`WorkloadExecutor` exports removed from `server/lib.rs`. SAFETY docs added to V4L2 ioctl + plugin ABI. Deprecated coordination re-exports removed from `distributed/lib.rs`. **8,895+ lib tests. Zero clippy.**
- **S291 (Jun 5, 2026)**: **Wave 78 Parity: Capability Registry + Zero Production #[allow]** — Created `config/capability_registry.toml` (machine-readable, 17 capability groups, 111 methods). Eliminated all 77 production `#[allow]` attributes: 14 deleted (stale), 58 converted to `#[expect]`, 4 unsafe justified with `#[expect]`, 4 cfg-gated. **Zero production `#[allow]`. Wave 78 compliant.**
- **S290 (Jun 4, 2026)**: **CallerContext Threading + Coordination Feature Gate + Panic Hygiene** — `compute.fan_out` now enforces resource envelope and emits telemetry (was ignoring `CallerContext`). `distributed::coordination` module (~6.3k LOC) feature-gated behind `legacy-coordination` (not default). `sovereign_acr_boot` binary unwraps hardened. ~45 diagnostic `write!().unwrap()` sites evolved to `let _ = write!()`. **8,895+ lib tests (default) + 1,289 (legacy-coordination). Full workspace clippy clean.**
- **S289 (Jun 4, 2026)**: **Telemetry Wire Contract + Adversarial Trust Tests + Telemetry Emission + Bollard Feature Gate** — `dispatch.telemetry.schema` evolved to versioned wire contract v1.1 (encoding rules, backward compat, consumer list for barraCuda/biomeOS L5 perceptron). +8 adversarial `dispatch.verify_trust` tests (forged BTSP, gate_id mismatch, malformed params, trust level serialization roundtrip). `DispatchTelemetryRecord` now emitted from `compute.dispatch.submit` and `shader.dispatch` via structured tracing (`dispatch.telemetry` target). `bollard` removed from default features in `runtime/container` (opt-in via `docker` feature). **9,204+ lib tests. Full workspace clippy clean.**
- **S288 (Jun 3, 2026)**: **Deep Debt Evolution VIII: Panic Elimination + Naming + Feature Gates + Safety Docs** — Akida MMIO panicking wrappers removed; VFIO callers use `try_read32`/`try_write32`. `cpu_resource` Rayon pool and `rm_trigger` ioctl buffers evolved to Result. BearDog type aliases removed (`SecurityServiceIntegration`, `SecurityPermission`). `modbus` feature-gated (`modbus-transport`). SAFETY docs on all `Ioctl::output_from_ptr` impls. **Zero P0 panic paths. Full workspace clippy clean.**
- **S287 (Jun 3, 2026)**: **S286 Consolidation + Telemetry Consumer + Trust Test Coverage** — `verify_trust` semantics tightened; `auth.peer_info` returns `gate_id`/`trust_level`/`transport`. Ownership lifecycle fixes (`revert_to_local_owner`, `gate.update`/`gate.remove`). `DispatchTelemetryRecord::to_feature_vector()` for barraCuda ml.mlp_train. +16 targeted trust/telemetry tests.
- **S286 (Jun 3, 2026)**: **Cross-Gate Trust Verification + Dispatch Telemetry + Yield-to-Owner** — `dispatch.verify_trust` + `dispatch.telemetry.schema` JSON-RPC methods. `DispatchTrustLevel` + connection-layer trust in `CallerContext`. `GateOwnership` + `TOADSTOOL_HARDWARE_OWNER_GATE_ID`. Owner gate bypasses guest load limits. Provenance injection on cross-gate forward.
- **S285 (Jun 3, 2026)**: **Deep Debt Evolution VII: Security Migration + Stub Evolution + Capability Naming** — Server encrypt/decrypt migrated `distributed::security` → `crypto_integration` (zero deprecated security callers). `NoopCryptoProvider`/`StubRuntimeEngine` → typed errors (`NoProviderRegistered`, `NoEngineRegistered`). `embedded-placeholder-impls` removed from specialty defaults. Hardcoded `"toadstool"` → `PRIMAL_NAME`. ~100L dead code removed; last production `expect()` → safe patterns. **Full workspace clippy clean, all tests pass.**
- **S284 (Jun 3, 2026)**: **Deep Debt Evolution VI: Large File Splits + Deprecated Cleanup + Final Panic Elimination** — Last 3 production files >800L split by concern (`sovereign_init` 991→7 modules, `open_vfio` 949→6, `experiment` 911→5). Final 2 library panics eliminated (`kernel_sentinel`, `visualization_client`). Dead deprecated symbols removed (BearDogBackend, legacy capability helpers). 33 server clippy fixes + test compilation fixes. **0 production files >800L, zero production library panics.**
- **S282 (May 28, 2026)**: **Deep Debt Evolution V: Complete Unsafe Hardening + Env Centralization + Panic Elimination** — 28 unsafe SAFETY doc gaps closed (12 files). 4 production panic paths evolved to Result. 110 raw env::var sites migrated (+56 new socket_env constants). libc::mmap→rustix::mm. 8 cylinder + 13 server clippy fixes. `PatchStrategy` → idiomatic `impl FromStr`. **178 lib tests, zero clippy, zero libc, ~98% env centralized.**
- **S281 (May 28, 2026)**: **Deep Debt Evolution IV: libc Elimination + Workspace Consolidation** — libc eliminated from cylinder (last C binding on hardware path). rm_trigger.rs → rustix::ioctl. rustix consolidated to workspace dep across 10 crates. +33 socket_env constants, 47 env::var sites migrated. **Zero libc in workspace.**
- **S280 (May 28, 2026)**: **Wave 59 Env Centralization + Clippy Allow Evolution** — Deleted orphan env_overrides.rs (342L). +73 socket_env constants. 117 env::var sites migrated across 30 files. Fixed 5 P0 bare #[allow(clippy::)].
- **S279 (May 27, 2026)**: **Deep Debt Evolution III: Panic Path Elimination + Capability Hardening** — All P0/P1 production panic paths eliminated. Legacy capability→primal roundtrip helpers deprecated. **9,156+ lib tests, zero clippy.**
- **S279 (May 27, 2026)**: **Exp 229: Catalyst Channel** — Full RM compute channel before warm swap (FECS ACR blocker). rm_trigger --channel 16-step Volta recipe.
- **S278 (May 27, 2026)**: **Deep Debt Evolution Sprint: Module Extraction + C→Rust + ABI Absorption** — Split 7 oversized files into module directories (sovereign_handoff 2,860L→11 modules, module_patch 2,020L→11, compute_device 2,072L→11 with gr_ungating/pbdma dedup, sovereign_stages 1,861L→7, guarded_sysfs 1,561L→5, channel/mod 1,117L→4, handler/sovereign 1,004L→6). Ported 4 userspace C tools to Rust bins (rm_trigger, sovereign_acr_boot, sovereign_pmu_boot, capture_pmu_falcon). Created `nv/registers/` (12 domain submodules) and `nv/rm_abi.rs` (canonical RM ABI types from coral-kmod). Evolved `StubGspBridge` → `NoopGspBridge` with capability guidance. Gated AMD Vega behind feature. Fossilized coral-kmod. **705 cylinder tests, zero clippy, zero userspace C.**
- **S90–S198 (Mar–Apr 2026)**: Full evolution history from REST API deletion through BTSP Phase 2, capability-based discovery, unsafe containment (89→46 blocks), OpenCL deprecation, dependency sovereignty, 6502/Z80 emulators, primal overstep cleanup, cross-spring absorptions, and coverage expansion (19K→21.5K tests). See [CHANGELOG.md](CHANGELOG.md) for session-by-session detail.

See [CHANGELOG.md](CHANGELOG.md) for full session-by-session detail.

---

## Active Debt (toadStool)

| ID | Description | Status |
|----|-------------|--------|
| D-COV | Test coverage → 90% | Active — 23,000+ tests (9,156+ lib); ~83.6% lib-only line (185K instrumented); remaining gap: hardware-dependent paths (VFIO, DRM, V4L2, akida) |
| D-BTSP-PHASE3 | BTSP encrypted post-handshake channel | **RESOLVED** (S215+S218) — ChaCha20-Poly1305 encrypted channel implemented, transport switch verified |

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

**Last Updated**: Jun 2026 — S294. **23,000+** workspace tests, 0 failures (8,895+ lib default; +1,289 legacy-coordination). ~83.6% lib-only line coverage (target 90%). **111 JSON-RPC methods** (direct, `DIRECT_JSONRPC_METHODS`; S286+ adds `dispatch.verify_trust`, `dispatch.telemetry.schema`) + semantic registry. AGPL-3.0-or-later. **Zero `libc`** (ecoBin v3.0 — all hardware I/O via rustix). Zero userspace C. **46 unsafe blocks** — all SAFETY-documented (confirmed S288); workspace `unsafe_code = "deny"`, **41 crates `forbid`** + 5 hw crates with narrow `#[allow(unsafe_code, reason)]`. **Zero production panics** (S282–S293: all paths evolved to Result; Akida MMIO panicking wrappers removed S293). Zero production TODO/FIXME/HACK. **~98% env centralized** (410+ reads via `socket_env::` constants; 23 LEGACY reads now emit deprecation tracing S293). **`--socket` CLI wired for UDS compliance** (S294). Rust 1.85+ (edition 2024). **Phase D dispatch live** (S254–S263). **Capability-based discovery compliant** per `CAPABILITY_BASED_DISCOVERY_STANDARD.md` v1.3. **Telemetry wire contract v1.1** — `dispatch.telemetry.schema` documented for barraCuda/biomeOS L5 consumption (S289).

---

Part of [ecoPrimals](https://github.com/ecoPrimals) — sovereign compute for science and human dignity.
