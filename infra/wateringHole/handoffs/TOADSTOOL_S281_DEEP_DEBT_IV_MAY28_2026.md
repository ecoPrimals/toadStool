# ToadStool S281 Handoff — Deep Debt Evolution IV

**Date**: May 28, 2026
**Sprint**: S281
**Focus**: libc elimination, unsafe hardening, workspace consolidation, env centralization wave 2

## Summary

Comprehensive deep debt audit and execution across all 6 dimensions:
dependencies, large files, unsafe, hardcoding, mocks, env centralization.

## Actions Taken

### 1. Dependency Evolution — libc → rustix (P0)

- **Eliminated `libc` from `toadstool-cylinder`** — last direct C binding on the core hardware path
- `rm_trigger.rs`: evolved `libc::ioctl` calls to `rustix::ioctl::Ioctl` trait pattern
- New `RmIoctl<const OP: Opcode, T>` adapter with documented SAFETY contracts
- Function signatures evolved from `RawFd` to `impl AsFd`
- Modernized to Rust 2024: `&raw const`/`&raw mut` pointers, struct init blocks

### 2. Workspace Dependency Consolidation

Unified 10 inline `rustix` version pins to `{ workspace = true }`:
- `cli`, `hw-learn`, `hw-safe`, `nvpmu`, `sysmon`, `monitoring`
- `akida-driver`, `display`, `secure_enclave`, `sandbox`
- All resolve to workspace `1.1.4` (was `"1"`, `"1.1"`, `"1.1"` etc.)

### 3. Unsafe Hardening

- **Fixed P0 panic**: `bar_cartography.rs:499` `.expect()` → `if let Some(bp)`
- **Added SAFETY comments** to all `unsafe` blocks in 3 diagnostic bins:
  - `sovereign_pmu_boot.rs`: mmap, read_volatile, write_volatile, munmap
  - `sovereign_acr_boot.rs`: same pattern
  - `capture_pmu_falcon.rs`: same pattern with read-only variant
- Added `/// # Safety` doc contracts on `Bar0::map()` methods

### 4. Environment Variable Centralization (Wave 2)

**+33 new constants** in `socket_env.rs`:
- Environment/runtime: TOADSTOOL_ENVIRONMENT, ENVIRONMENT, ENV, HOST, DISPLAY, WAYLAND_DISPLAY
- Discovery: TOADSTOOL_DISCOVERY_CONFIG, FALLBACK_PORT/ENABLED, SERVICE_DIR, REGISTRY_ENDPOINT, BIOMEOS_RUNTIME_DIR
- Service URLs: COORDINATION/CRYPTO/STORAGE/AI_SERVICE_URL, COORDINATOR, STORAGE, SERVICES
- K8s/container: KUBERNETES_SERVICE_HOST, POD_NAMESPACE, COMPOSE_PROJECT_NAME, CONSUL_HTTP_ADDR, ETCD_ENDPOINTS
- Deprecated: BEARDOG_FAMILY_SEED

**47 raw sites migrated** across 15 files in config, common, toadstool, auto_config, cli.

### 5. Audit Findings (no action needed)

- **Large files**: 9 production files >800L — all cylinder VFIO or acceptable (ABI tables, HW init sequences)
- **Mocks**: All properly gated behind `#[cfg(test)]`/`feature = "test-mocks"`; `Noop*` stubs return typed errors
- **TODO/FIXME**: Zero comment markers in production code

## Metrics

| Metric | Before | After |
|--------|--------|-------|
| `libc` in workspace | 1 (cylinder) | **0** |
| `rustix` inline pins | 10 | **0** (all workspace) |
| Production panic paths | 1 (bar_cartography) | **0** |
| SAFETY-undocumented bins | 3 | **0** |
| Env reads via socket_env | ~258 (64%) | **~305 (76%)** |
| Raw env reads remaining | ~148 | **~100** |
| Lib tests passing | 9,156 | **9,156** |
| Clippy warnings | 0 | **0** |

## Remaining Debt (P2+)

- ~100 raw `std::env::var` sites in deployment infra, observability, substrate probes
- `axum` in container default path (should be feature-gated or replaced with UDS JSON-RPC)
- `reqwest` + `rustls` in edge conflicts with `aws-lc-sys` ban when `http-downloads` enabled
- Workspace dep hoisting: `mdns-sd`, `zeroize`, `bytemuck`, `blake3`, `rayon` repeated
- Package version alignment: ~20 crates still at `0.1.0` vs workspace `0.2.0`
- `primal_capabilities::get_endpoint` — name-based registry routing (P1 hardcoding)
