# ToadStool

**Sovereign Compute Hardware** | Pure Rust | ecoBin | Jun 2026 | S285 | v0.2.0

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
| `cargo test --workspace` | **23,000+ tests, 0 failures** (9,156+ lib-only), **~222** ignored (hardware-gated); full workspace ~7m |
| Doctests | All passing (common, core, server, cli, testing, display) |
| Standalone clone test | Pull to any machine, `cargo test` works (GPU-optional, CPU fallback, device-lost resilient) |
| `unsafe` blocks | **46 actual** (all in hw-safe/GPU/VFIO/display/plugin containment crates); **all SAFETY-documented**; workspace `unsafe_code = "deny"`, **41 crates `forbid`** + 5 hw crates with narrow `#[allow(unsafe_code, reason)]`; **all lint attrs have `reason =`** |
| Production panics/unwraps | **0** production `unwrap()` / `expect()` / `panic!()` (S282–S285: all paths evolved to Result) |
| Production stubs / test mocks | Stubs evolved to real implementations or typed errors (`NoProviderRegistered`, `NoEngineRegistered`); **embedded-placeholder** opt-in via `embedded-placeholder-impls` feature (S285 — removed from default features); **auth test mocks** (`InMemoryAuthBackend`) isolated under **`#[cfg(any(test, feature = "test-mocks"))]`**; **`test-mocks` removed from default features** (S206 — production builds exclude mock code) |
| Production `Box<dyn Error>` | 0 in core crates -- all typed errors (thiserror) |
| Production TODOs / FIXME / HACK | 0 in production code |
| Dead code | ~400+ lines removed (REST, middleware, dead modules); **~80** justified `#[allow]` remain (conditional compilation, deprecated compat) |
| External deps eliminated | `chrono` (28 crates) + `log` (2) + `instant` + `anyhow` (core) + `pollster` + `serde_yaml` + `libc` (akida-driver→rustix) + `sysinfo` (15 crates→toadstool-sysmon) + `caps` + `console` + `indicatif` + `figment` + `handlebars` + 23 phantom deps. S164: dep dedup (linfa/ndarray/mockall/env_logger). S166: `ed25519-dalek` (→BearDog RPC), `regex` (→`str::contains`), `parking_lot` (→`std::sync`). S169: `pyo3` (FFI), `gbm`, `linfa`, `hmac`, `indicatif` removed |
| Hardcoded primal names | **0** user-visible; **~400** intentional legacy-compat refs remain (env fallbacks, serde aliases, parse_type); all new code is capability-first per `CAPABILITY_BASED_DISCOVERY_STANDARD.md` v1.2 |
| `async-trait` migration | **DEPRECATED** — fully removed and banned in `deny.toml` (S203r). **Stadial parity gate cleared (S203s)**: ~32 traits converted from `dyn` dispatch to **enum dispatch + RPITIT**. Zero finite-implementor `dyn` remaining. |
| Wildcard re-exports | Narrowed in 13 crates (explicit `pub use` reduces recompilation cascade) |
| Hardcoded ports/localhost | 0 inline literals -- config constants + capability-based discovery |
| Hardware transport | Implemented | DRM display, V4L2 capture, serial — frame protocol + router |
| JSON-RPC surface | **88** JSON-RPC methods (direct) + semantic registry |
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

### JSON-RPC Methods (83 direct + semantic registry; S186+)

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
| Workspace tests | **23,000+**, 0 failures (9,156+ lib-only) |
| Lib-only line coverage | ~83.6% |
| Full workspace test time | ~7m (unlimited parallelism, `cfg!(test)` fast timeouts; GPU crates have NVK resilience wrappers) |
| `unsafe` blocks | **46 actual** (all in hw-safe/GPU/VFIO/display/plugin containment crates); **all SAFETY-documented**; workspace `unsafe_code = "deny"`, **41 crates `forbid`** + 5 hw crates with narrow `#[allow(unsafe_code, reason)]` |
| Production panics/unwraps | **0** production `unwrap()` / `expect()` / `panic!()` (S282–S285: all paths evolved to Result) |
| Production `Box<dyn Error>` | 0 in core crates -- all typed errors (thiserror) |
| Production stubs | Typed error returns (`NoProviderRegistered`, `NoEngineRegistered`, etc.); test-only mocks **`#[cfg(test)]`** only |
| Production `todo!()`/`unimplemented!()`/`dbg!()` | 0 |
| Production FIXME / HACK | 0 |
| Dead code removed | ~400+ lines (REST handlers, middleware, dead modules); **~80** justified `#[allow]` remain (conditional compilation, deprecated compat) |
| Hardcoded localhost/ports/URLs in prod | 0 -- config constants + capability-based discovery |
| External deps eliminated | `chrono`, `log`, `instant`, `anyhow` (core), `pollster`, `serde_yaml`, **`libc`** (S281→S282: zero libc, all mmap/ioctl via rustix), `sysinfo`, `caps`, `console`, `indicatif`, `figment`, `handlebars` + 23 phantom deps. S164: dep dedup. S166: `ed25519-dalek`/`regex`/`parking_lot`. S169: `pyo3`, `gbm`, `linfa`, `hmac`, `indicatif` |
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
- **S285 (Jun 3, 2026)**: **Deep Debt Evolution VII: Security Migration + Stub Evolution + Capability Naming** — Server encrypt/decrypt migrated `distributed::security` → `crypto_integration` (zero deprecated security callers). `NoopCryptoProvider`/`StubRuntimeEngine` → typed errors (`NoProviderRegistered`, `NoEngineRegistered`). `embedded-placeholder-impls` removed from specialty defaults. Hardcoded `"toadstool"` → `PRIMAL_NAME`. ~100L dead code removed; last production `expect()` → safe patterns. **Full workspace clippy clean, all tests pass.**
- **S284 (Jun 3, 2026)**: **Deep Debt Evolution VI: Large File Splits + Deprecated Cleanup + Final Panic Elimination** — Last 3 production files >800L split by concern (`sovereign_init` 991→7 modules, `open_vfio` 949→6, `experiment` 911→5). Final 2 library panics eliminated (`kernel_sentinel`, `visualization_client`). Dead deprecated symbols removed (BearDogBackend, legacy capability helpers). 33 server clippy fixes + test compilation fixes. **0 production files >800L, zero production library panics.**
- **S282 (May 28, 2026)**: **Deep Debt Evolution V: Complete Unsafe Hardening + Env Centralization + Panic Elimination** — 28 unsafe SAFETY doc gaps closed (12 files). 4 production panic paths evolved to Result. 110 raw env::var sites migrated (+56 new socket_env constants). libc::mmap→rustix::mm. 8 cylinder + 13 server clippy fixes. `PatchStrategy` → idiomatic `impl FromStr`. **178 lib tests, zero clippy, zero libc, ~98% env centralized.**
- **S281 (May 28, 2026)**: **Deep Debt Evolution IV: libc Elimination + Workspace Consolidation** — libc eliminated from cylinder (last C binding on hardware path). rm_trigger.rs → rustix::ioctl. rustix consolidated to workspace dep across 10 crates. +33 socket_env constants, 47 env::var sites migrated. **Zero libc in workspace.**
- **S280 (May 28, 2026)**: **Wave 59 Env Centralization + Clippy Allow Evolution** — Deleted orphan env_overrides.rs (342L). +73 socket_env constants. 117 env::var sites migrated across 30 files. Fixed 5 P0 bare #[allow(clippy::)].
- **S279 (May 27, 2026)**: **Deep Debt Evolution III: Panic Path Elimination + Capability Hardening** — All P0/P1 production panic paths eliminated. Legacy capability→primal roundtrip helpers deprecated. **9,156+ lib tests, zero clippy.**
- **S279 (May 27, 2026)**: **Exp 229: Catalyst Channel** — Full RM compute channel before warm swap (FECS ACR blocker). rm_trigger --channel 16-step Volta recipe.
- **S278 (May 27, 2026)**: **Deep Debt Evolution Sprint: Module Extraction + C→Rust + ABI Absorption** — Split 7 oversized files into module directories (sovereign_handoff 2,860L→11 modules, module_patch 2,020L→11, compute_device 2,072L→11 with gr_ungating/pbdma dedup, sovereign_stages 1,861L→7, guarded_sysfs 1,561L→5, channel/mod 1,117L→4, handler/sovereign 1,004L→6). Ported 4 userspace C tools to Rust bins (rm_trigger, sovereign_acr_boot, sovereign_pmu_boot, capture_pmu_falcon). Created `nv/registers/` (12 domain submodules) and `nv/rm_abi.rs` (canonical RM ABI types from coral-kmod). Evolved `StubGspBridge` → `NoopGspBridge` with capability guidance. Gated AMD Vega behind feature. Fossilized coral-kmod. **705 cylinder tests, zero clippy, zero userspace C.**
- **S276 (May 26, 2026)**: **Deep Debt Evolution II — Unwrap Elimination, Sovereign Split, memmap2 Removal** — Eliminated remaining production unwrap/expect/unreachable: sovereign.rs 2x unwrap, mmio_region.rs expect, dma.rs Drop expect, diagnostic interpreter 6x expect, permissions.rs expect, dispatch unreachable!(). `handler/sovereign.rs` (1,003L, 11 handlers) → module directory (init/snapshot/capture). `memmap2` removed from hw-safe — `safe_mmap.rs` rewritten on rustix. 3 stale primal-name type aliases deprecated. `ipc.register` capability list aligned to Node Atomic set. 13 upstream clippy warnings absorbed. **88+ JSON-RPC methods. 9,158+ lib tests, zero clippy.**
- **S275 (May 25, 2026)**: **Wave 49: Ecosystem Tightening** — Showcase fossilized (35 files → fossilRecord). wateringHole consolidated (36 handoffs mirrored, archive/ created). Stale deploy patterns fixed (4 files → plasmidBin). Startup latency optimized (deferred wgpu, pre-bound socket). toadstool.toml HTTP-era template fossilized. Docs fossil-tagged.
- **S273 (May 24, 2026)**: **Deep Debt Evolution — Production Panic Surface Eliminated + Module Extraction** — Eliminated remaining production panic surface: 29 `unwrap()` in `kernel_health.rs` → error propagation, `.expect()` in dispatch cache → `Result`, 5 `.expect()` in `ember_client.rs` → `?`, 2 fallible `Default` impls removed from `secure_enclave`. Refactored `dispatch/mod.rs` 1,638→839L — 7 sovereign handlers extracted to `dispatch/sovereign.rs` (814L). Refactored `warm_init.rs` 1,439L → module dir (`mod.rs` + `seeders.rs` + `trials.rs`). 6 CLI `well_known::*` hardcoded primal name sites migrated to capability-based discovery with legacy fallback. `activity_tracker().record()` wired into 7 VFIO dispatch paths; `#[allow(dead_code)]` removed. `health.liveness` always-alive behavior documented (S272). hw-safe abstractions validated; cylinder migration deferred. **88 JSON-RPC methods. 9,131+ lib tests, 700 cylinder tests, zero clippy.**
- **S259 (May 13, 2026)**: **Universal Sovereign Dispatch: Last Mile + Deep Debt** — Wired `device.vfio.open` + `device.vfio.roundtrip` JSON-RPC endpoints with `ember.vfio.*` semantic aliases. `NvVfioComputeDevice::dispatch()` → QMD-based (generation-aware via SM version). `try_vfio_nvidia` auto-calls `open_vfio()` after warm FECS detection. `TOADSTOOL_SOCKET_MODE` env var on tarpc socket. Refactored `compute_device.rs` 825→753L via helper extraction. Fixed 6 `#[expect]` without `reason`. **79 JSON-RPC methods. 8,837 lib tests, zero clippy.**
- **S258 (May 13, 2026)**: **PBDMA Dispatch Wiring** — Full `ComputeDevice` trait impl for `NvVfioComputeDevice`: `alloc`/`upload`/`dispatch`/`readback`/`sync` via VFIO DMA + GPFIFO. Two-stage FECS/VFIO gate model. `open_vfio()` initializes full dispatch state.
- **S256–S257 (May 13, 2026)**: **FECS Warm-State + Deep Debt** — `CPUCTL_HALTED` bit fix (0x10→0x20). `probe_warm_fecs()` for warm-preserved FECS detection. VFIO factory path in `create_cylinder_device_factory()`.
- **S255 (May 13, 2026)**: **hotSpring S243 Audit Response + Deep Debt Sweep** — All 4 hotSpring S243 blockers confirmed resolved. Added `ember.swap` + `sovereign.boot` semantic aliases. Method count corrected 74→77. Benchmark refactored 820→756L via `LatencyStats` extraction. **77 JSON-RPC methods. 8,832 lib tests, zero clippy.**
- **S254 (May 13, 2026)**: **Phase D Factory + NvVfioComputeDevice** — `create_cylinder_device_factory()` — BDF → sysfs DRM → driver → `ComputeDevice` (AMD live, NV FECS-gated). `LocalDeviceFactory` registered at `DispatchHandler` construction. **8,832 lib tests, zero clippy.**
- **S253 (May 13, 2026)**: **Phase C Complete + Deep Debt Sweep** — `VfioResourceHandle` `Option<i32>` → `OwnedFd`. SwapOrchestrator real quiesce/persist/restore. `toadstool device` CLI (swap/list/status/warm). 5 `CORALREEF_*` env vars deprecated. 13 `#[allow(deprecated)]` → `#[expect(deprecated, reason)]`. **8,827 lib tests, zero clippy.**
- **S252 (May 13, 2026)**: **Diesel Engine Migration Batch 1–2** — `device.swap` + `device.warm_catch` JSON-RPC handlers. 6 MMIO/Falcon RPCs. `SysfsBar0Rw`. `TOADSTOOL_RUN_DIR` socket layout. **8,827 lib tests, zero clippy.**
- **S251 (May 13, 2026)**: **hotSpring Sovereign Compute Evolution — Daemon Parity Gaps C1–C7** — Wired full buffer lifecycle (alloc→upload→dispatch→sync→readback→free) into `try_local_dispatch`. Integrated `shader.dispatch` with Phase D local path. Wired `ember.reacquire` JSON-RPC handler. Added `device.*` → `ember.*` semantic aliases for diesel-mode callers. Updated dispatch capabilities phase from B to D. Documented `GspBridge` boundary with hotSpring hardware validation context (FECS dispatch readback root cause). **67 JSON-RPC methods** (direct) + 3 new `device.*` semantic aliases. **8,809 lib-only tests, zero clippy.**
- **S250 (May 12, 2026)**: **Deep Debt Evolution — Legacy Env Deprecation, Sentinel Cleanup, Stub Docs** — Added `#[deprecated]` to 13 legacy primal-named env var constants (`BEARDOG_SOCKET`, `SONGBIRD_SOCKET`, etc.) with migration guidance. Added `tracing::warn!` deprecation notices for 4 legacy env vars in runtime fallback paths. Replaced `NO_HISTORY_SENTINEL_SECS = 999` magic number with `Duration::MAX`. Evolved `DEFAULT_SCAN_SUBNET` from hardcoded `192.168.1.0` to env-configurable via `TOADSTOOL_SCAN_SUBNET`. Clarified sentinel/null-object docs on `StubRuntimeEngine`, `NoopCloudProvider`, `NoopCryptoProvider`. Updated `StubRuntimeEngine` version string `"stub"` → `"unregistered"`. Full multi-dimensional audit: zero `todo!()`/`unimplemented!()`, all unsafe idiomatic, `cargo deny check bans` passes clean. **8,809 lib-only tests, zero clippy.**
- **S249+ (May 12, 2026)**: **Pass 12-14 Execution — Phase C Batches 5-7, Phase D, toadstool.validate** — Absorbed 68 VFIO channel files + `hardware_guard.rs` (Batch 5, 499→520 cylinder tests). Created `GspBridge` trait + `StubGspBridge` for GSP-dependent firmware ops; `falcon_pio.rs` for standalone PIO uploads (Batch 6). Absorbed `bar0.rs` with re-exported `ApplyError`/`RegisterAccess` (Batch 7). Wired local dispatch cutover: `try_local_dispatch()` via `ComputeDevice` trait before `coral_client` IPC forward (Phase D). Implemented `toadstool.validate` JSON-RPC method — Tier 2 Science API pre-flight (`valid`, `gpu_available`, `precision_tier`, `warnings`, `required_capabilities`). Semantic alias `runtime.workload.validate`. **67 JSON-RPC methods.** **8,809 lib-only tests, zero clippy.**
- **S238 (May 11, 2026)**: **Deep Debt — Magic Numbers, println→tracing, deny.toml, JH-2 Audit** — Consolidated 20+ duplicated magic numbers into named constants: new `common::defaults` module for distributed subsystem, container port/resource/image constants, host_config defaults, edge runtime constants. Migrated `akida-models` zoo `println!` to structured `tracing::info!`. Fixed 3 stale `deny.toml` comments about ring lockfile status. Audited and confirmed JH-2 envelope enforcement is complete: all 3 dimensions (`mem_mb`, `cpu_cores`, `max_timeout_ms`) checked in `enforce_envelope`, called by submit + shader + pipeline stages. **JH-2: FULLY RESOLVED.**
- **S237 (May 11, 2026)**: **Wave 8 Phase A: coral-ember Absorption** — Absorbed coralReef `coral-ember` hardware lifecycle modules into `toadstool-ember`: vendor lifecycle (NVIDIA 4 variants, AMD 2, Intel, BrainChip, Generic), observation types, ring metadata, sysfs abstraction, error types. Created `VfioResourceHandle` — first production `ResourceHandle` implementation. Wired `device_pool` into `compute.dispatch.submit` path with `acquire_device_handle()`. `compute.dispatch.capabilities` reports `ember.held_devices` and `ember.phase`. 90 ember tests, 76 dispatch tests, clippy clean.
- **S236 (May 11, 2026)**: **Deep Debt — Magic Numbers, Match Safety, Test Refactor** — Extracted magic numbers to named constants. Eliminated `unreachable!()` in `nvpmu/dma.rs`. Smart-refactored `dispatch/tests.rs` into 4 submodules. 72 dispatch tests, clippy clean.
- **S235 (May 11, 2026)**: **Wave 8 Compute Trio Foundation** — BrainChip vendor ID fixed. Trio-standard IPC contract for `compute.dispatch.submit`. Gate 2 capabilities. Absorption roadmap (Phases A-D). +9 tests.
- **S234 (May 11, 2026)**: **IPC Env Var Expansion Contract** — Documented that JSON-RPC methods are "pre-resolved only": `${VAR}`/`$VAR` expansion is CLI-only (`load_workload_file`). IPC callers must send fully resolved values. Added `compute.execute` to METHODS.md (was missing). Code-level doc comments on `submit_workload` and `dispatch_submit_with_context`. README IPC contract section added.
- **S233 (May 8, 2026)**: **DF-2 Fix + Tier 3 `eprintln!` Migration** — `JsonRpcHandler` now reads `TOADSTOOL_AUTH_MODE` env var at startup: `"enforcing"` or `"enforced"` triggers `GateMode::Enforcing` (previously hardcoded to `permissive()`). `auth.mode` reports `"enforcing"` when deployed with env var set. Migrated production `eprintln!` to `tracing` in `performance/manager.rs` (`tracing::warn!`) and `cli/commands/npu.rs` (`tracing::error!`). Standalone CLI binaries (`nvpmu-apply`, examples) retain `eprintln!` (no tracing subscriber). Test code `eprintln!` left as-is (idiomatic for test diagnostics). +2 tests (enforcing gate mode, anonymous denial code). **DF-2: RESOLVED. Tier 3 `eprintln!`: RESOLVED.**
- **S232 (May 8, 2026)**: **JH-2 Phase 2 — Full Envelope Enforcement** — `cpu_cores` enforcement (workgroup threads ≤ cores × 1024), `max_timeout_ms` enforcement, and `CallerContext` now threaded through **all dispatch paths**: `compute.dispatch.submit`, `shader.dispatch`, `compute.dispatch.pipeline.submit` (each internal stage inherits caller context). Pipeline stages no longer bypass envelope checks. `ResourceEnvelope` gains `max_timeout_ms` field. +7 tests (774 total in server). **JH-2 toadStool: COMPLETE** — all 4 dimensions enforced (mem, cpu, timeout, method allowlist) across all 3 dispatch entry points.
- **S231 (May 8, 2026)**: **JH-2 Phase 1 — Resource Envelope Enforcement at Dispatch** — `ResourceEnvelope` type, `CallerContext`, `enforce_envelope()` wired into `compute.dispatch.submit`. Gate `check_with_context()` enforces method allowlists.
- **S230 (May 8, 2026)**: **Error Code Alignment** — `-32000`/`-32001` per ecosystem standard. Workload codes relocated.
- **S229 (May 8, 2026)**: **JH-0 — MethodGate Adoption** — Pre-dispatch capability gate. All methods classified Public/Protected. 3 `auth.*` methods. +21 tests.
- **S228 (May 7, 2026)**: **primalSpring Audit Response** — toadStool gap (short timeout sensitivity) confirmed RESOLVED (PG-62). Dispatch timeout documentation added to README. Zero open gaps from audit.
- **S226 (May 7, 2026)**: **Deep Debt Audit — Workspace Clippy Clean + Lint Hygiene** — Full deep debt survey: **zero** production files >800 LOC, **zero** production TODOs/FIXMEs, all 46 unsafe blocks at kernel FFI boundaries with SAFETY docs, all mocks test-gated, all primal names intentional legacy compat. Fixed 8 clippy errors in `integration-primals` (4 `unused_async` → removed `async`, 2 redundant closures, 1 debug formatting, 1 `map_unwrap_or`). Bare `#[allow(dead_code)]` in `distributed/network/metrics.rs` given reason. `deny.toml` stale `ring` comment corrected. `auto_config` `items_after_statements` fixed (S225). **Workspace `cargo clippy -- -D warnings`: zero errors.** 22,843 tests.
- **S225 (May 7, 2026)**: **PG-62 — Health Liveness Fast-Path + Startup Reorder** — `health.liveness` now returns `{"status":"starting"}` during initialization and `{"status":"alive"}` once fully ready (via `Arc<AtomicBool>` readiness flag). `health.readiness` likewise returns `"starting"` → `"ready"`. Discovery registration + biomeOS scan moved **after** listener spawn so the socket accepts connections immediately. Recommended caller timeout: **≥3 seconds** (documented). BearDog cross-family `crypto.sign_contract` (PG-60+) tracked for Phase 60+. +5 tests. 22,843 tests.
- **S224 (May 7, 2026)**: **PG-55 — `--bind` Flag + Localhost Default (Security)** — Added `--bind host:port` CLI flag to `server`/`daemon` subcommands. Default TCP bind changed from `0.0.0.0` to `127.0.0.1`. Priority: CLI `--bind` > `TOADSTOOL_BIND_ADDRESS` env > localhost default. +5 tests. 22,838 tests.
- **S223 (May 6, 2026)**: **Deep Debt — Smart Refactor + Sleep Elimination + Test Speed** — Smart-refactored `btsp/json_line.rs` (905→478 LOC + `relay.rs` 255 + `negotiate.rs` 189; zero files >800 LOC). Eliminated 4 test sleeps: `thread::sleep` → deterministic time manipulation (backdated timestamps, `tokio::time::pause`). Evolved `intrusion.rs` to `tokio::time::Instant` for testable time. Fixed `auto_config` ecosystem discovery blocking 10s on mDNS daemon startup/shutdown and DNS resolution in tests — `cfg!(test)` early-returns skip network I/O (237 tests: 10.02s→0.24s, 41x speedup). Confirmed zero production `unwrap()`. Stale `/tmp/ecoPrimals/discovery/` comments evolved to capability-based language. +12 tests (resource_validator/analysis). 22,833 tests.
- **S222 (May 5, 2026)**: **primalSpring ironGate Audit — Sandbox + Env Expansion + Specs + Discovery** — Gap 2: sandbox `working_dir` override via `trusted_directories` (was hardcoded `/tmp`). Gap 8: `${VAR}`/`$VAR` env expansion in workload TOMLs. `display.composite` + `transport.bridge` specs added (PG-42). Discovery hierarchy documented. `convert_security_context` now parses isolation level. +26 tests. 22,821 tests.
- **S221 (May 4, 2026)**: **Deep Debt — Capability Names + Dep Hygiene + Stub Evolution + Coverage** — All `barraCuda/coralReef` primal names in errors/deprecations evolved to capability-based language. reqwest 0.12→0.13 (ring→aws-lc-rs). `verify_migration_success` dead code fixed. Unsafe count reconciled 49→46. +20 tests (daemon routes 11, zero_config config 9). 22,580 tests.
- **S220 (May 4, 2026)**: **primalSpring Phase 58 Audit Response — Coverage Push + Stub Evolution** — Responded to 4-item audit. Phase 3 encryption confirmed RESOLVED (S215+S218). +22 tests (wasm/metrics, stub_runtime_engine, os_layer/manager, container/engine). `OSLayerManager` fallback evolved from synthetic success to `not_supported`. 22,560 tests.
- **S219 (May 3, 2026)**: **Deep Debt — Production Stubs + Lock Safety + Coverage** — 3 remaining production stubs evolved to typed errors (coordination health, legacy compat, monitoring mutex). `/tmp/biomeos-runtime` fallback configurable via `BIOMEOS_RUNTIME_DIR` env. 98 new tests across ember/glowplug crates. 22,538 tests.
- **S218 (May 3, 2026)**: **BTSP Phase 3 Transport Switch Verification** — Closed primalSpring audit finding: verified negotiate→encrypted framing transition. 15 new E2E tests incl. full negotiate→encrypted frame exchange.
- **S216-S217 (May 2, 2026)**: **Deep Debt — Flaky Tests + Orphan Recovery + Coverage** — MQ transport stub→`not_supported`. `ResourceOrchestrator` 12 `.expect("poisoned")`→`Result`. Flaky `primal_sockets` tests fixed. 6 orphaned `integration-primals` modules recovered. 35+ new tests.
- **S215 (May 2, 2026)**: **BTSP Phase 3: Encrypted Channel (ChaCha20-Poly1305)** — Server-side `btsp.negotiate` + encrypted framing. HKDF-SHA256 directional key derivation. primalSpring wire-compatible.
- **S214 (May 1, 2026)**: **PG-46 Socket Reuse + BTSP Phase 3 Assessment** — `ConnectedJsonRpcClient` for Unix stream reuse. BTSP Phase 3 readiness audit. `BTSP_RPC_TIMEOUT` reduced to 2s.
- **S207–S213 (Apr 28–30, 2026)**: **Deep debt + BTSP hardening** — Self-registration via `DISCOVERY_SOCKET` (`ipc.register`). PG-46 BTSP handshake timeout (5s budget). All lint attrs given `reason=` (~42 sites). 23 Cargo.toml dep unifications. Auth issuer → capability-based. GPU stubs → capability URIs. ~100 new tests (Phase 56c audit). hw-safe `expect()`→`Result`.
- **S173–S176 (Apr 19–23, 2026)**: **Deep debt + BTSP relay** — BTSP JSON-line handshake relay (Phase 45c). Edge crate clippy clean (231→0). `NoopCryptoProvider`/`NoopCloudProvider` → capability-based error guidance. 3 large specialty files → directory modules. Runtime/edge compilation fixed (61 errors). `#[allow]` → `#[expect(reason)]`. Clippy 0 warnings.
- **S203–S203t (Apr 12–16, 2026)**: **Composition Elevation Sprint** — 19 sub-sessions of intensive evolution. `async-trait` fully removed and banned (91 annotations → native AFIT/RPITIT). 32 traits → enum dispatch. Dispatch wire contract standardized. 52+ production files refactored via test extraction (zero >500 LOC). `compute.execute` route promoted. TCP idle timeout (300s). primalSpring LD-04/LD-05 resolved. Dual-socket naming (`compute.sock` + `compute-tarpc.sock`). All production mocks cfg-gated. Real Linux sandbox ops (rustix). Plugin system → `libloading` dlopen. Env interning complete (~55 `socket_env` constants). 22,061→7,784 tests across phases, clippy clean throughout.
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

**Last Updated**: Jun 2026 — S285. **23,000+** workspace tests, 0 failures (9,156+ lib-only). ~83.6% lib-only line coverage (target 90%). **88 JSON-RPC methods** (direct) + semantic registry. AGPL-3.0-or-later. **Zero `libc`** (ecoBin v3.0 — all hardware I/O via rustix). Zero userspace C. **46 unsafe blocks** — all SAFETY-documented; workspace `unsafe_code = "deny"`, **41 crates `forbid`** + 5 hw crates with narrow `#[allow(unsafe_code, reason)]`. **Zero production panics** (S284–S285: all paths evolved to Result). Zero production TODO/FIXME/HACK. **~98% env centralized** (410+ reads via `socket_env::` constants). Rust 1.85+ (edition 2024). **Phase D dispatch live** (S254–S263). **Capability-based discovery compliant** per `CAPABILITY_BASED_DISCOVERY_STANDARD.md` v1.3.

---

Part of [ecoPrimals](https://github.com/ecoPrimals) — sovereign compute for science and human dignity.
