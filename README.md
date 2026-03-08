# ToadStool

**Sovereign Compute Hardware** | Pure Rust | ecoBin | March 2026

---

## What Is This?

**ToadStool** is the hardware infrastructure primal. It discovers GPUs, NPUs, CPUs at runtime via sysfs/PCIe. JSON-RPC 2.0 + tarpc IPC over Unix sockets. GPU job queue with cross-gate routing. Ollama model lifecycle management. Distributed workload dispatch across machines. Cloud cost estimation, compliance validation, and federation.

**Key principles:**
- **Capability-based discovery** -- primals discover each other at runtime by capability, not name
- **Self-knowledge only** -- ToadStool knows its own identity; everything else is discovered
- **ecoBin compliant** -- single binary, pure Rust, cross-architecture, cross-platform
- **Hardware layer** -- discovers and exposes compute substrates; barraCuda dispatches math

**BarraCuda** (compute math) is a separate primal at `ecoPrimals/barraCuda/`. ToadStool provides hardware discovery and capability probing; barraCuda dispatches shaders and owns all math.

---

## Ecosystem Role

```
NUCLEUS = BearDog + Songbird + ToadStool + NestGate
Tower   = BearDog + Songbird          <- communication + crypto
Node    = Tower  + ToadStool          <- us -- sovereign compute
Nest    = Tower  + NestGate           <- storage
```

**biomeOS grade**: Node Atomic READY -- ToadStool A++ socket-standardized.

**Deployment**: Tower starts first (BearDog -> Songbird), then ToadStool. Socket: `$XDG_RUNTIME_DIR/biomeos/toadstool.sock`. ToadStool discovers other primals at runtime by capability, not by name.

---

## Quality Gates

| Gate | Status |
|------|--------|
| `cargo build --workspace` | Clean |
| `cargo fmt --all -- --check` | 0 diffs |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 warnings |
| `cargo doc --workspace --no-deps` | 0 warnings |
| `cargo test --workspace` | 19,840+ workspace tests (0 failures, 203 intentional ignores) |
| Doctests | All passing (common, core, server, cli, testing, display) |
| Standalone clone test | Pull to any machine, `cargo test` works (GPU-optional, CPU fallback, device-lost resilient) |
| `unsafe` blocks | ~70+ (GPU APIs + FFI/MMIO), all `// SAFETY:` documented |
| Production panics/unwraps | 0 blind `unwrap()`; infallible `expect()` only |
| Production stubs | 0 -- all evolved to real implementations (architecture stubs → typed enums/traits S128) |
| Production `Box<dyn Error>` | 0 in core crates -- all typed errors (thiserror) |
| Production TODOs / FIXME / HACK | 0 in production code |
| Dead code | ~400+ lines removed (REST, middleware, dead modules); ~25 justified `#[allow(dead_code)]` remain |
| External deps eliminated | `chrono` (28 crates) + `log` (2) + `instant` + `anyhow` (core) + `pollster` + `serde_yaml` + `libc` (akida-driver→rustix) |
| Hardcoded primal names | 0 -- all capability-based discovery via `get_socket_path_for_capability()` |
| `async-trait` migration | 5 crates migrated to native AFIT; remaining uses justified by `dyn Trait` dispatch |
| Wildcard re-exports | Narrowed in 13 crates (explicit `pub use` reduces recompilation cascade) |
| Hardcoded ports/localhost | 0 inline literals -- config constants + capability-based discovery |
| Hardware transport | Implemented | DRM display, V4L2 capture, serial — frame protocol + router |
| License | AGPL-3.0-or-later -- root LICENSE file + SPDX headers on all files |
| File size limit | All production files under 1000 lines (40+ god files smart-refactored into domain modules) |
| Test concurrency | All tests concurrent (`--test-threads=8`), zero `#[serial]`, zero fixed sleeps in non-chaos tests |
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
- Distributed LLM inference (TinyLlama-1.1B: 39.85 tok/s across two gates with BearDog encrypted tensor transport)
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
  Ollama Model Lifecycle (list/load/inference/unload)
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
- **NestGate integration** -- real JSON-RPC `storage.artifact.store`/`retrieve` with graceful fallback
- **Real-time events**: `compute.status` JSON-RPC polling or biomeOS/songbird coordination for event streaming

### JSON-RPC Methods (85+ dynamically built)

| Domain | Methods | Notes |
|--------|---------|-------|
| `toadstool.*` | `health`, `version`, `query_capabilities` | Canonical namespace |
| `toadstool.resources.*` | `estimate`, `validate_availability`, `suggest_optimizations` | Canonical namespace |
| `resources.*` | `estimate`, `validate_availability`, `suggest_optimizations` | biomeOS neural API routing aliases |
| `compute.*` | `health`, `version`, `capabilities`, `discover_capabilities`, `submit`, `status`, `result`, `cancel`, `list` | biomeOS Node Atomic aliases + GPU queue |
| `ai.*` | `local_inference`, `local_execute` | biomeOS ai_local capability |
| `ai.nautilus.*` | `status`, `observe`, `train`, `predict`, `screen`, `edges`, `shell.export`, `shell.import` | Evolutionary reservoir computing (feature-gated `nautilus`) |
| `gpu.*` | `info`, `memory` | Hardware info |
| `ollama.*` | `list_models`, `inference`, `load`, `unload` | Local LLM lifecycle |
| `gate.*` | `update`, `remove`, `list`, `route` | Distributed routing |
| `transport.*` | `discover`, `list`, `route` | Hardware transport discovery + routing |
| `shader.compile.*` | `wgsl`, `spirv`, `status`, `capabilities` | coralReef proxy (capability-based discovery, naga fallback) |
| `ecology.*` | 14 methods | Ecosystem integration (NUCLEUS discovery, capability routing) |
| `discovery.*` | NUCLEUS discovery, capability probing | Runtime capability-based discovery |
| `deploy.*` | graph routing, workload placement | Deploy graph routing and placement |
| `toadstool.provenance` | cross-spring flow matrix | Cross-spring evolution introspection API |

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
|   +-- server/                    JSON-RPC server, GPU job queue, Ollama, cross-gate router
|   +-- (api/ fossilized S96 — ByobApi extracted to container, remainder to ecoPrimals/fossil/)
|   +-- cli/                       UniBin CLI (single binary, BYOB server subcommand)
|   +-- integration/               Inter-primal protocols (beardog, nestgate, songbird)
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
|   +-- management/                Analytics, monitoring, resources (real ResourceManager with sysinfo)
+-- (fossil at ecoPrimals/fossil/toadStool/)
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
6. **Mocks isolated to testing** -- all `#[cfg(test)]` gated; production code is complete implementations
7. **Honest documentation** -- no aspirational claims as facts; ML stubs return `ModelNotLoaded`/`ModelBackendRequired`
8. **Vendor-agnostic** -- WGPU/Vulkan for GPU discovery, any vendor works
9. **Sovereign compute** -- no vendor lock-in, pure Rust core
10. **100% unsafe documentation** -- every `unsafe` block has `// SAFETY:` comments (~60+ blocks, all justified)
11. **Shared error tracking** -- `AtomicU64` counter across all server transports

### Quality Metrics

| Metric | Value |
|--------|-------|
| Clippy pedantic warnings | 0 (workspace-wide `clippy::pedantic` clean; `#[expect]` evolution S131+) |
| Doc warnings | 0 |
| Build warnings | 0 |
| Workspace tests | 19,840+ (server + core + distributed + common + config + CLI + testing + display + auto_config) |
| Full workspace test time | ~8m (8 threads, GPU crates have NVK resilience wrappers) |
| `unsafe` blocks | ~70+ (GPU APIs + FFI/MMIO), all `// SAFETY:` documented |
| Production panics/unwraps | 0 blind `unwrap()`; infallible `expect()` only |
| Production `Box<dyn Error>` | 0 in core crates -- all typed errors (thiserror) |
| Production stubs / mocks | 0 -- all evolved to real implementations or proper errors |
| Production `todo!()`/`unimplemented!()`/`dbg!()` | 0 |
| Production FIXME / HACK | 0 |
| Dead code removed | ~400+ lines (REST handlers, middleware, dead modules); ~25 justified `#[allow(dead_code)]` remain |
| Hardcoded localhost/ports/URLs in prod | 0 -- config constants + capability-based discovery |
| External deps eliminated | `chrono`, `log`, `instant`, `anyhow` (core), `pollster`, `serde_yaml`, `libc` |
| Default test timeout | 5s (unit: 2s, integration: 30s, chaos: 20s) |
| Hardware transports | 3 | Display (DRM), Capture (V4L2), Serial (feature-gated) |

---

## Evolution

**We are still evolving.** barraCuda (separate primal) owns all math and shaders. ToadStool focuses on hardware discovery, capability probing, and workload orchestration. All 5 spring handoffs absorbed.

### Active / Next
- **Test coverage** -- pushing toward 90% target; 19,840+ tests; focus on hardware-dependent code
- **DF64 / ComputeDispatch** -- transferred to barraCuda team (S93); toadStool serves hardware capabilities
- **Sovereign compiler Phase 4+** -- register pressure estimation, loop software pipelining (barraCuda)

### Recently Completed
- **S134 (Mar 8, 2026)**: BearDog crypto delegation enforced (secure_enclave). `dev-crypto` feature gate for SoftwareHsmProvider. lifecycle_ops refactored. Unused deps removed (notify, aes-gcm from secure_enclave). Volatile MMIO safe abstraction. Property-based tests (proptest).
- **S133 (Mar 8, 2026)**: Ada Lovelace reclassification, f64_zeros_risk, fused_ops_healthy(), 14 ecology.* methods, NUCLEUS discovery, deploy graph routing, 20 semantic methods (71→91). Spring versions: hotSpring v0.6.23, groundSpring V99, neuralSpring V90/S132, wetSpring V99, airSpring v0.7.5.
- **S131+ (Mar 7, 2026)**: Spring sync (all 5 springs pinned to latest). `#[allow]` → `#[expect]` lint evolution (3 stale suppressions removed). SPRING_ABSORPTION_TRACKER comprehensive update. coralReef E2E AMD dispatch milestone noted.
- **S130+ (Mar 7, 2026)**: Deep debt execution. Clippy pedantic zero workspace-wide. Unsafe audit (70+ blocks justified). Dep/hardcoding/clone/file-size audits. 240 new coverage tests. CI pedantic gate.
- **S130 (Mar 7, 2026)**: Cross-spring shader rewiring. `shader.compile.*` evolved to real coralReef proxy. Cross-spring provenance tracking. 31 new tests.
- **S129 (Mar 7, 2026)**: C dep evolution. 5 god files refactored. 200+ coverage tests. Zero-copy hot paths.
- **S128 (Mar 6, 2026)**: f64 shared-memory bug absorbed. `PrecisionRoutingAdvice`. `shader.compile.*` IPC. Architecture stubs evolved.
- **S90–S96**: REST API deleted. barraCuda budded. Sovereign pipeline. NpuDispatch. ecoBin verified.

See [CHANGELOG.md](CHANGELOG.md) for full session-by-session detail.

---

## Active Debt (toadStool)

| ID | Description | Status |
|----|-------------|--------|
| D-COV | Test coverage → 90% | Active -- 19,840+ tests; focus on hardware-dependent code |
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
| management/resources | Placeholder evolved to real ResourceManager with sysinfo (re-added to workspace S95) |

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
| [STATUS.md](STATUS.md) | Detailed technical status, session-by-session |
| [DEBT.md](DEBT.md) | Active debt register, workarounds, evolution paths |
| [NEXT_STEPS.md](NEXT_STEPS.md) | Roadmap and upcoming work |
| [EVOLUTION_TRACKER.md](EVOLUTION_TRACKER.md) | Cross-spring absorption tracker |
| [QUICK_REFERENCE.md](QUICK_REFERENCE.md) | Commands, JSON-RPC methods, API reference |
| [DOCUMENTATION.md](DOCUMENTATION.md) | Navigation hub (guides, specs, audits) |
| [CHANGELOG.md](CHANGELOG.md) | Full session-by-session evolution history |
| [SOVEREIGN_COMPUTE.md](SOVEREIGN_COMPUTE.md) | Sovereign compute phases and Mesa NAK roadmap |

---

**Last Updated**: March 8, 2026 -- S134. 19,840+ workspace tests. ~86% line coverage (121K production lines). 85+ JSON-RPC methods. Clippy pedantic zero. BearDog crypto delegation enforced. `dev-crypto` feature gate. All 5 springs synced. Rust 1.82+ (MSRV).
