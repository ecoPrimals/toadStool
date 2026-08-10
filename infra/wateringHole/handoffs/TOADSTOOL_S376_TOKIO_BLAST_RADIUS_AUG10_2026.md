# ToadStool S376 — Tokio Blast Radius Reduction

**Date**: Aug 10, 2026
**Sprint**: S376
**Gate**: strandGate (eastGate overwatch)

## Summary

Systematically reduced tokio's presence from a workspace-wide unconditional
dependency to a deployment-layer-only concern. Production code no longer uses
`tokio::fs` or `tokio::process`. `tokio::sync::RwLock` reduced from ~99 files
to ~20 (irreducible async contexts). 7 more crates feature-gated for WASM
(31→38/48). Workspace tokio features trimmed from 9 to 7.

## Changes

### Phase 1: `tokio::fs` → `std::fs` (37 production files)
- Config loading, hardware detection, policy file reads — none were
  high-concurrency I/O paths
- Crates affected: auto-config, cli, core/toadstool, core/common, server,
  security/sandbox, security/policies, runtime/display, runtime/wasm,
  runtime/edge, runtime/container, distributed, integration/security
- 1 legitimate async use retained: `display_ops.rs` async file tailing via
  `tokio::fs::File::from_std()`

### Phase 2: `tokio::process` → `std::process` (15 production files)
- GPU detection (`nvidia-smi`, `lspci`), installer operations, cross-compilation
  toolchains — all fire-and-forget subprocess calls
- Crates affected: auto-config, core/toadstool, cli, runtime/native
- Zero remaining `tokio::process` usage

### Phase 3: `tokio::sync::RwLock` → `std::sync::RwLock` (~65 files)
- Guards not held across `.await` → safe to migrate
- Poison-tolerant: `.unwrap_or_else(|e| e.into_inner())`
- ~20 files remain on tokio (guards held across `.await` in deployment-layer code)
- Crates affected: runtime/gpu (9 files), distributed (18 files), core/common,
  core/toadstool, server, cli, security, testing, runtime/edge, and others

### Phase 4: Feature-gate 7 crates (31→38/48 WASM)
- `toadstool-auto-config` — hardware detection types ungated, installer/ecosystem gated
- `toadstool-client` — already done in S375
- `toadstool-integration-protocols` — protocol types ungated, transport/client gated
- `toadstool-management-monitoring` — metric types ungated, collection/reporting gated
- `toadstool-distributed` — types/common ungated, networking/coordination gated
- `toadstool-runtime-wasm` — already done in S375
- `toadstool-runtime-gpu` — types/frameworks ungated, coordinator/scheduler/engine gated

### Phase 5: Workspace tokio features trimmed
- Removed `fs` and `process` from workspace-level tokio features
- Before: `rt-multi-thread, macros, sync, time, net, io-util, fs, signal, process` (9)
- After: `rt-multi-thread, macros, sync, time, net, io-util, signal` (7)

## Metrics

| Metric | Before | After |
|--------|--------|-------|
| Crates with tokio optional | 17 | 24 |
| WASM-capable crates | 31/48 | 38/48 |
| `tokio::fs` usage | 37+ files | 0 production files |
| `tokio::process` usage | 15 files | 0 files |
| `tokio::sync::RwLock` usage | ~99 files | ~20 files |
| Workspace tokio features | 9 | 7 |
| Native-only crates | 17 | 10 |

## Remaining Native-Only (10 crates — irreducible)
- `toadstool-server` — TCP/Unix socket listeners, daemon lifecycle
- `toadstool-cli` — daemon management, process lifecycle
- `toadstool-runtime-container` — Docker/OCI engine management
- `toadstool-runtime-display` — DRM/framebuffer, X11/Wayland
- `toadstool-runtime-edge` — mDNS discovery, device deployment
- `toadstool-runtime-native` — OS process execution, security contexts
- `toadstool-testing` — test infrastructure, chaos testing
- `toadstool-examples` — example binaries
- `toadstool-integration-tests` — integration test harness
- `toadstool-security-sandbox` — seccomp/cgroup containment

## Verification
- `cargo check --workspace` — 0 errors, 0 warnings
- `cargo test --workspace --lib` — 9,008+ tests, 0 failures
- All documentation updated (README, CROSS_ARCH, CHANGELOG, CONTEXT, DEBT,
  DOCUMENTATION, NEXT_STEPS, specs/README, sporeprint)
