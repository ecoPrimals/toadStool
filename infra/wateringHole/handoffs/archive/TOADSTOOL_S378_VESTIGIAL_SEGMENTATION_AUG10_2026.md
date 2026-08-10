# ToadStool S378 — Tokio Vestigial Segmentation

**Date**: Aug 10, 2026 | **Sprint**: S378 | **Gate**: strandGate

## Summary

The "irreducible" ~118-file tokio production surface was largely vestigial — primordial code reimplementing what Tower Atomic primals (songBird, bearDog, cellMembrane) and biomeOS now own. Feature-gated ~35k LOC of dead modules behind 9 non-default features. Migrated remaining safe `tokio::time`/`tokio::sync` to `std` equivalents. Excluded orphaned `runtime/edge` crate.

**Result: Default-build tokio surface 118 → 65 production files (45% reduction).**

## What Changed

### Feature-gated vestigial modules (preserved as fossil record)

| Feature | Module | LOC | Reimplements |
|---------|--------|-----|--------------|
| `legacy-cloud` | `distributed/cloud/` | ~7.8k | biomeOS graph executor |
| `legacy-security` | `distributed/security/` + `security_provider/` + `crypto_lock/` | ~12k | bearDog via `crypto_integration` |
| `legacy-scheduler` | `distributed/universal/scheduler` + adapter + platform | ~1k | biomeOS + core scheduler |
| `legacy-protocol-client` | `protocols/client/` + root `transport` | ~2.5k | biomeOS capability routing |
| `legacy-security-client` | `protocols/security_client/` | ~2k | bearDog via `crypto_integration` |
| `hardening` | `performance_hardening/async_ops,caching` + `circuit_breaker` + `intrusion` | ~3k | Zero production callers |
| `background-monitors` | `server/background/{resource,health,statistics,cleanup,capability}` + `ServerState` | ~2k | Never started in production |
| `cli-monitoring` | `cli/monitoring/` | ~1.8k | No CLI command wiring |
| `network-scan` | `auto_config/ecosystem_network.rs` | ~0.5k | songBird domain (TCP scanning) |

### Workspace exclusion

- `runtime/edge/` — Orphaned crate, zero workspace dependents → moved to `[workspace].exclude`

### Safe migrations to `std`

- **`tokio::time::Duration` → `std::time::Duration`**: 8 CLI files
- **`tokio::time::Instant` → `std::time::Instant`**: 2 files
- **`tokio::sync::RwLock` → `std::sync::RwLock`**: 10 files (runtime-specialty, auto_config, GPU coordinator/engine, WASM cache, distributed client)
- **`tokio::sync::Mutex` → `std::sync::Mutex`**: 3 files (server transport, GPU coordinator, distributed scheduling)
- **WASM `cache_wasmi.rs`**: Entire module now synchronous (wasmi is sync)

## Metrics

| Metric | Before (S377) | After (S378) |
|--------|---------------|--------------|
| Default-build tokio production files | 118 | **65** |
| Vestigial LOC in default build | ~35k | **0** (gated) |
| Non-default features (vestigial) | 1 (`legacy-coordination`) | **9** |
| `runtime/edge` in workspace | Yes | **Excluded** |
| GPU tokio::sync | 4 files | **2** (frameworks/devices retained — guards across await) |
| WASM tokio | 1 file | **0** |

## Remaining Irreducible Tokio (~65 files)

Genuinely needed for the async deployment layer:
- `server/` (15) — JSON-RPC server, BTSP, cross-gate UDS, ipc_watch, silicon_discovery, pcie_keepalive
- `core/toadstool` (8) — IPC client/server, platform sockets, ipc_helpers
- `core/common` (9) — BTSP framing/handshake/relay, unix_jsonrpc_client
- `runtime/display` (6) — Display IPC server/client
- `runtime/specialty` (5) — Async adapter trait (engine, embedded, mainframe)
- `runtime/gpu` (3) — Framework discovery (guards across await)
- `cli/` (7) — Daemon server, signals, display_ops tail, executor lifecycle
- `distributed/` (2) — coordination_integration, crypto_integration
- `client/` (2) — JSON-RPC client
- Other (6) — container BYOB, native engine, ember keepalive, testing helpers

## Dead Features Excised

| Feature | Crate | Reason |
|---------|-------|--------|
| `plugin-loading` | `toadstool` | C FFI dlopen via `libloading` — ecoBin v3.0 incompatible. `ffi_loader.rs` deleted. |
| `wgpu` | `toadstool` | Superseded by `runtime/gpu` + server `gpu-discovery`. Core crate no longer pulls wgpu. |
| `wasm-runtime` | `toadstool` | Dead probe stub — CLI never passed it. Real WASM engine is `runtime/wasm`. |
| `akida` | `toadstool-core` | Empty stub with no dep. `AkidaNpuDispatch` adapter not yet written. |
| `vulkano` dep | `runtime/gpu` | Dead weight — `vulkan` feature actually uses wgpu-vulkan. `FrameworkHandle::Vulkan` excised. |

## Last-Mile Gaps (documented, not this sprint)

### NPU/Akida pipeline
- `AkidaNpuDispatch` adapter (impl `NpuDispatch` wrapping `NpuBackend`) — should live in `akida-driver`
- Server NPU handler (dedicated, not aliasing GPU dispatch)
- CLI `npu` feature wiring (uncomment `akida-driver` dep, add to `Commands` enum)

### WASM workload pipeline
- `conversion.rs` rejects WASM workloads ("not yet supported")
- Server daemon `runtime_engines` empty at startup (no engine registration)
- Biome lifecycle hardcodes `RuntimeType::Native` for WASM sources (bug)
- `compute.engine.register` JSON-RPC method referenced but missing
