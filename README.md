# ToadStool

**Sovereign Compute Hardware** | Pure Rust | ecoBin | Jun 2026 | S317 | v0.2.0

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
| `cargo test --workspace` | **23,000+ tests, 0 failures** (9,069+ lib-only default; +1,289 behind `legacy-coordination`), **~222** ignored (hardware-gated); full workspace ~7m |
| Doctests | All passing (common, core, server, cli, testing, display) |
| Standalone clone test | Pull to any machine, `cargo test` works (GPU-optional, CPU fallback, device-lost resilient) |
| `unsafe` blocks | **44 actual** (all in hw-safe/GPU/VFIO/display/plugin containment crates); **all SAFETY-documented** (S310: −2 via kernel_sentinel AsFd evolution); workspace `unsafe_code = "deny"`, **41 crates `forbid`** + 5 hw crates with narrow `#[allow(unsafe_code, reason)]`; **all lint attrs have `reason =`** |
| Production panics/unwraps | **0** production `unwrap()` / `expect()` / `panic!()` / `unreachable!()` (S282–S290: all paths evolved to Result; S313: 3 `unreachable!()` → typed errors) |
| Production stubs / test mocks | Stubs evolved to real implementations or typed errors (`NoProviderRegistered`, `NoEngineRegistered`); **embedded-placeholder** opt-in via `embedded-placeholder-impls` feature (S285 — removed from default features); **auth test mocks** (`InMemoryAuthBackend`) isolated under **`#[cfg(any(test, feature = "test-mocks"))]`**; **`test-mocks` removed from default features** (S206 — production builds exclude mock code) |
| Production `Box<dyn Error>` | 0 in core crates -- all typed errors (thiserror) |
| Production TODOs / FIXME / HACK | 0 in production code |
| Dead code | ~400+ lines removed (REST, middleware, dead modules); **zero production `#[allow]`** (S291 — all converted to `#[expect]` with `reason`; ~13 test-only `#[allow]` remain) |
| External deps eliminated | `chrono` (28 crates) + `log` (2) + `instant` + `anyhow` (core) + `pollster` + `serde_yaml` + `libc` (akida-driver→rustix) + `sysinfo` (15 crates→toadstool-sysmon) + `caps` + `console` + `indicatif` + `figment` + `handlebars` + 23 phantom deps. S164: dep dedup (linfa/ndarray/mockall/env_logger). S166: `ed25519-dalek` (→security service RPC), `regex` (→`str::contains`), `parking_lot` (→`std::sync`). S169: `pyo3` (FFI), `gbm`, `linfa`, `hmac`, `indicatif` removed. S288: `modbus` (feature-gated `modbus-transport`, not default). S289: `bollard` (feature-gated `docker`, not default). S292: `serialport` (feature-gated `serial-transport`, not default). S293: `tarpc` removed from display (unused), made optional in protocols (`tarpc-transport`) |
| Hardcoded primal names | **0** user-visible; install/launcher paths use `PRIMAL_BINARY_NAME` (S292); **~400** intentional legacy-compat refs remain (env fallbacks, serde aliases, parse_type); all new code is capability-first per `CAPABILITY_BASED_DISCOVERY_STANDARD.md` v1.3 |
| `async-trait` migration | **DEPRECATED** — fully removed and banned in `deny.toml` (S203r). **Stadial parity gate cleared (S203s)**: ~32 traits converted from `dyn` dispatch to **enum dispatch + RPITIT**. Zero finite-implementor `dyn` remaining. |
| Wildcard re-exports | Narrowed in 13 crates (explicit `pub use` reduces recompilation cascade) |
| Hardcoded ports/localhost | 0 inline literals -- config constants + capability-based discovery |
| Hardware transport | Implemented | DRM display, V4L2 capture, serial — frame protocol + router |
| JSON-RPC surface | **112** JSON-RPC methods (direct) + semantic registry |
| License | AGPL-3.0-or-later -- root LICENSE file + SPDX headers on all files |
| File size limit | Non-hardware production files target **< 500 lines**. **0 production files >750L** (S284 split large files, S303+S306+S307 tightened gate to 750L); test-only files in `tests/` directories may exceed limit. |
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

Callers probing `health.liveness` should use a timeout of **≥3 seconds** (recommended: 5s for composition startup). `health.liveness` always returns `{"status":"alive"}` — if the caller can reach the handler, the process is alive. Boot-phase signaling is handled by `health.readiness` (`"starting"` → `"ready"`). The socket accepts connections immediately upon listener bind (prebind + early health responder, S272/Wave 47), so callers get a fast response during cold start. If BTSP handshake is required, add its budget (5s default, overridable via `BTSP_HANDSHAKE_TIMEOUT_SECS`).

| Probe | During init | After ready |
|-------|-------------|-------------|
| `health` | `{"status":"starting","primal":"toadstool","version":"..."}` | `{"status":"alive","primal":"toadstool","version":"..."}` |
| `health.liveness` | `{"status":"alive"}` | `{"status":"alive"}` |
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

### JSON-RPC Methods (112 direct + semantic registry; S286+ adds `dispatch.verify_trust`, `dispatch.telemetry.schema`)

Surface trimmed to hardware orchestration and IPC boundaries. **Removed from this repo** (S169): `inference.*` / Ollama-style AI (→ intelligence service), **`shader.compile.*`** (→ visualization service), **`science.*`** / **`ecology.*`** / **`discovery.*`** / **`deploy.*`** relays (→ orchestration and peers). **Kept**: **`shader.dispatch`** (dispatch compiled binary to GPU; compile happens in visualization service).

| Domain | Methods | Notes |
|--------|---------|-------|
| `health` | `health` | GuideStone-mandated bare probe: `{status, primal, version}` |
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
|   +-- client/                    ToadStool client library (tarpc + JSON-RPC client builders)
|   +-- auto_config/               Zero-config auto-detection (sysfs, env, discovery)
|   +-- integration-tests/         Cross-crate integration tests
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
| Workspace tests | **23,000+**, 0 failures (9,069+ lib default; +1,289 legacy-coordination) |
| Lib-only line coverage | ~85%+ |
| Full workspace test time | ~7m (unlimited parallelism, `cfg!(test)` fast timeouts; GPU crates have NVK resilience wrappers) |
| `unsafe` blocks | **44 actual** (all in hw-safe/GPU/VFIO/display/plugin containment crates); **all SAFETY-documented**; workspace `unsafe_code = "deny"`, **41 crates `forbid`** + 5 hw crates with narrow `#[allow(unsafe_code, reason)]` |
| Production panics/unwraps | **0** production `unwrap()` / `expect()` / `panic!()` / `unreachable!()` (S282–S290: all paths evolved to Result; S313: 3 `unreachable!()` → typed errors) |
| Production `Box<dyn Error>` | 0 in core crates -- all typed errors (thiserror) |
| Production stubs | Typed error returns (`NoProviderRegistered`, `NoEngineRegistered`, etc.); test-only mocks **`#[cfg(test)]`** only |
| Production `todo!()`/`unimplemented!()`/`dbg!()` | 0 |
| Production FIXME / HACK | 0 |
| Dead code removed | ~400+ lines (REST handlers, middleware, dead modules); **zero production `#[allow]`** — all converted to `#[expect]` with `reason` (S291); ~13 test-only `#[allow]` remain |
| Hardcoded localhost/ports/URLs in prod | 0 -- config constants + capability-based discovery |
| External deps eliminated | `chrono`, `log`, `instant`, `anyhow` (core), `pollster`, `serde_yaml`, **`libc`** (S281→S282: zero libc, all mmap/ioctl via rustix), `sysinfo`, `caps`, `console`, `indicatif`, `figment`, `handlebars` + 23 phantom deps. S164: dep dedup. S166: `ed25519-dalek`/`regex`/`parking_lot`. S169: `pyo3`, `gbm`, `linfa`, `hmac`, `indicatif`. S288: `modbus` (feature-gated `modbus-transport`). S289: `bollard` (feature-gated `docker`, not default) |
| Env centralization | **~98%** (~410+ env reads via `socket_env::` constants); <10 raw `env::var("...")` remaining (S282–S285) |
| Default test timeout | 5s (unit: 2s, integration: 30s, chaos: 20s) |
| Hardware transports | 3 | Display (DRM), Capture (V4L2), Serial (feature-gated) |

---

## Evolution

**We are still evolving.** barraCuda (separate primal) owns all math and shaders. ToadStool focuses on hardware discovery, capability probing, and workload orchestration. All 5 spring handoffs absorbed.

### Active / Next
- **Test coverage** -- pushing toward 90% target; 23,000+ tests (9,069+ lib); ~85%+ lib-only line (185K lines instrumented); remaining gap: hardware-dependent paths (VFIO, DRM, V4L2), specialty runtimes
- **Sovereign VFIO dispatch** -- NVIDIA VFIO PBDMA dispatch wired via QMD (S258–S259); `device.vfio.open` + `device.vfio.roundtrip` JSON-RPC endpoints live; e2e validated on Titan V (S263)
- **DF64 / ComputeDispatch** -- transferred to barraCuda team (S93); toadStool serves hardware capabilities
- **Sovereign compiler Phase 4+** -- register pressure estimation, loop software pipelining (barraCuda)
- **NUCLEUS crypto integration** -- compute payloads encrypted via Tower `crypto.encrypt`/`crypto.decrypt` (S205); **self-registration with coordination service** via `DISCOVERY_SOCKET` + `ipc.register` at startup (S207)

### Recently Completed
- **S315 (Jun 14, 2026)**: **Wave 113 Compliance** — Bare `"health"` JSON-RPC method added (`{status, primal, version}` — guideStone-mandated shape). Early-health responder now accepts riboCipher `[0xEC, 0x01]` prefix. Wave 113 REJECT enforced: unsignalled connections on all accept loops (Unix, TCP, BTSP) now return `-32600` error instead of legacy fallback. MITO/NUCLEAR tiers send error response instead of silent close. Tests updated to use riboCipher signal.
- **S317 (Jun 15, 2026)**: **Deprecated Symbol Evolution II** — 6 deprecated symbols deleted: `IntelligenceBackend::new`, `SecurityBackend::new`, `SocketStorageBackend::new` (→test-only `new_test`), `SecurityClient::new` (production callers migrated to `new_async`), `invoke_http` (HTTP match arm inlined as error), `setup_websocket_federation` (trait method + 7 dead tests removed). Clippy `if_not_else` fixed in `unix.rs`. ~25 `#[expect(deprecated)]` test attrs cleaned.
- **S316 (Jun 15, 2026)**: **Deep Debt XVII: File Splits + Dead Symbol Deletion** — `cpu_resource.rs` split (749→673L): dispatch enums extracted to `compute_dispatch.rs`. `glowplug_client.rs` split (729→635L): serde types extracted to `glowplug_types.rs`. `TOADSTOOL_ENABLE_GRPC` constant deleted (zero callers since S314). Unfulfilled `#[expect(dead_code)]` removed from `executor/types.rs` (code now alive).
- **S314 (Jun 14, 2026)**: **Deprecated Symbol Evolution** — `node_type::{BEARDOG,SONGBIRD,NESTGATE}` deleted (zero production callers). `FeatureFlags::enable_grpc` field removed (dead — populated but never read). `DISTRIBUTED_URL` + `get_distributed_storage_url()` dead API bundle removed. `TOADSTOOL_ENABLE_GRPC` env constant deprecated.
- **S313 (Jun 14, 2026)**: **Deep Debt XVI** — 3 production `unreachable!()` → typed `ServerError::Internal` (zero production panics). `unix.rs` split: 815L → `unix.rs` (512L) + `btsp_unix.rs` (334L). `#[allow(dead_code)]` → `#[expect]` in `executor/types.rs`. Federation re-export `#[allow]` documented.
- **S312 (Jun 13, 2026)**: **riboCipher Wave 112 escalation** — legacy unsignalled connections upgraded from WARN → ERROR on all 4 accept loops. Wave 113 will REJECT.
- **S311 (Jun 13, 2026)**: **riboCipher transport signal convergence (Wave 111)** — server-side detection on all JSON-RPC accept loops (Unix + TCP, 4 loops), client-side `[0xEC, 0x01]` signal on all outbound IPC (register, discover, announce, `UnixJsonRpcClient`, `ConnectedJsonRpcClient`). Tier 2/3 stubs present.
- **S310 (Jun 13, 2026)**: **Deep Debt XV** — kernel_sentinel.rs unsafe eliminated (BorrowedFd → AsFd, −2 blocks), forensics.rs path env-configurable, `CoordinationTransport::GRPC` formally deprecated, test file splits, test `#[allow]` hygiene.
- **S309 (Jun 12, 2026)**: **TOADSTOOL-AUTO-REGISTER (Wave 111 P2)** — PCI sysfs GPU/NPU hardware inventory wired into `ipc.register` + `primal.announce` payloads.
- **S308 (Jun 10, 2026)**: **PRIMAL-SOCKET-CLEANUP (Wave 107 P2)** — `BIOMEOS_SOCKET_DIR` wired into all socket/discovery-file resolution chains. Zero `/tmp` writes when `BIOMEOS_SOCKET_DIR` is set.
- **S307 (Jun 10, 2026)**: **Deep Debt XIV** — `registers.rs` split (pri/cg/pclock), `pm4.rs` test extract, `swap.rs` test extract, stale test cleanup (25 tests removed), unfulfilled lint hygiene. **Zero production files >750L.**
- **S306 (Jun 9, 2026)**: **Deep Debt XII–XIII** — `bar_cartography.rs` + `amd/ioctl.rs` file splits, `ServiceMeshType` enum removed.
- **S305 (Jun 9, 2026)**: **Deprecated Symbol Evolution** — `AuthManager`/`AgentDeploymentManager` sync ctor migration to async. 13 `#[allow]` → `#[expect]` with reasons. Zero sync-ctor fallbacks.
- **S304 (Jun 8, 2026)**: **Deep Debt XI** — Category A deprecated symbols removed (`DiscoveredService`, `ServiceType`, stale test helpers).
- **S303 (Jun 8, 2026)**: **Deep Debt X-XI** — `page_tables.rs` split. `#[allow]`/`#[expect]` hygiene pass. File-size gate tightened to 750L.
- **S301–S302 (Jun 8, 2026)**: **Transport Evolution** — `TRANSPORT_ENDPOINT` accepted (sourDough wire-compatible). `connect_transport()` for outbound. `IpcClient::from_transport_endpoint()` bridge. BYOB default bind `127.0.0.1`.
- **S300 (Jun 6, 2026)**: **Deep Debt X** — `/tmp` path hardcoding eliminated. `temp_dir()` used for platform-agnostic fallback.
- **S298 (Jun 6, 2026)**: **Coverage Push IV** — +44 tests (9,069 lib). silicon/job/coordination/method_gate/auth tests.
- **S294–S297 (Jun 5–6, 2026)**: **Coverage Push I–III + VPS Compliance** — +174 tests. `--socket` wired, `--headless` mode, musl-static binary. **9,069+ lib tests.**
- **S289–S293 (Jun 4–5, 2026)**: **Deep Debt VIII–X + Telemetry** — `tarpc` gating, unwrap purge, cylinder splits, LEGACY deprecation tracing, telemetry wire contract v1.1, bollard/serialport feature-gated, `CallerContext` threading, zero production panics.
- **S284–S288 (Jun 3, 2026)**: **Deep Debt VI–VIII** — Last 3 files >800L split, BearDog aliases removed, `modbus` gated, SAFETY docs, security migration, typed stub errors. Zero production files >800L.
- **S278–S283 (May 27–31, 2026)**: **Deep Debt I–V + Module Sprint** — Split 7 oversized files, ported 4 C tools to Rust, libc eliminated, env centralized (~98%), unsafe hardened, zero clippy.
- **S90–S198 (Mar–Apr 2026)**: Full evolution history from REST API deletion through capability-based discovery, unsafe containment (89→46 blocks), dependency sovereignty, and coverage expansion (19K→21.5K tests). See [CHANGELOG.md](CHANGELOG.md).

See [CHANGELOG.md](CHANGELOG.md) for full session-by-session detail.

---

## Active Debt (toadStool)

| ID | Description | Status |
|----|-------------|--------|
| D-COV | Test coverage → 90% | Active — 23,000+ tests (9,069+ lib); ~85%+ lib-only line (185K instrumented); remaining gap: hardware-dependent paths (VFIO, DRM, V4L2, akida) |
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

**Last Updated**: Jun 2026 — S317. **23,000+** workspace tests, 0 failures (9,069+ lib default; +1,289 legacy-coordination). ~85%+ lib-only line coverage (target 90%). **112 JSON-RPC methods** (direct) + semantic registry. AGPL-3.0-or-later. **Zero `libc`** (ecoBin v3.0 — all hardware I/O via rustix). **44 unsafe blocks** — all SAFETY-documented; workspace `unsafe_code = "deny"`, **41 crates `forbid`**. **Zero production panics.** Zero production TODO/FIXME/HACK. **~98% env centralized.** **Zero `/tmp` hardcoding** — `BIOMEOS_SOCKET_DIR` > `XDG_RUNTIME_DIR` > `temp_dir` (S308). **`TRANSPORT_ENDPOINT` accepted** (S301–S302). **Zero production files >750L** (S316: cpu_resource split 749→673, glowplug_client split 729→635). **Zero production `#[allow]`**. Rust 1.85+ (edition 2024). **Phase D dispatch live** (S254–S263). **Capability-based discovery compliant** per `CAPABILITY_BASED_DISCOVERY_STANDARD.md` v1.3. `ProtectSystem=strict` compatible (S308). **Auto-register hardware** (S309). **riboCipher REJECT** — Wave 113 enforced: unsignalled connections rejected with error response (S315). **GuideStone `health` method** — `{status, primal, version}` (S315).

---

Part of [ecoPrimals](https://github.com/ecoPrimals) — sovereign compute for science and human dignity.
