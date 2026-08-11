# ToadStool S380 — G72 Dependency Pandemic Tier 2: wgpu 28 + axum Excision + Deep Debt

**Date**: Aug 11, 2026
**Sprint**: S380
**Wave**: 157i POST-PANDEMIC CASCADE

## Completed

### Tier 2 Quick Wins (early S380)
- **`uuid` workspace promotion** — 15 crates migrated from inline `version = "1.7"` to `{ workspace = true }`. Zero version fragmentation.
- **`tracing-subscriber` feature-gated** — `toadstool` crate now has `logging = ["dep:tracing-subscriber"]` (non-default). CLI unaffected (uses own subscriber).
- **`tokio-serde` workspace aligned** — server inline → `{ workspace = true, features = ["json"] }`.

### wgpu 22 → 28 (MSRV 1.85 → 1.92)
- **Declared MSRV bumped** — `rust-version = "1.92.0"` in workspace `Cargo.toml`.
- **wgpu workspace dep updated** — `22` → `28`, features: `wgsl`, `vulkan-portability`.
- **GPU crate** (`runtime/gpu`) — `spirv` feature added for SPIR-V shader loading. `Instance::new` borrowed. `enumerate_adapters` awaited. `request_adapter` returns `Result`. `request_device` single-argument. `ShaderSource::SpirV` replaces removed unsafe API.
- **Universal crate** (`runtime/universal`) — `Limits` subgroup fields moved to `AdapterInfo`. Device descriptor updated. All 7 test `AdapterInfo` literals updated with new fields (`device_pci_bus_id`, `subgroup_min_size`, `subgroup_max_size`, `transient_saves_memory`).
- **Adaptive crate** (`runtime/adaptive`) — `Instance::new` panic caught with `catch_unwind` for no-backend-available environments. Lock-poisoning `.expect()` → `.unwrap_or_else(|e| e.into_inner())`.
- **Server crate** — `Maintain::Wait` → `PollType::wait_indefinitely()`. `SPIRV_SHADER_PASSTHROUGH` → `EXPERIMENTAL_PASSTHROUGH_SHADERS`. `PipelineLayoutDescriptor::push_constant_ranges` → `immediate_size`. `entry_point` now `Option<&str>`. `#![recursion_limit = "256"]` for trait evaluation.
- **Glowplug** (`runtime/gpu`) — 1 test `AdapterInfo` literal updated.
- **Crates touched**: `runtime/gpu`, `runtime/universal`, `runtime/adaptive`, `server` (6 modules), `core/glowplug`.
- **toadStool leads the ecosystem** — first primal on wgpu 28.

### axum Excision (HTTP → UDS JSON-RPC)
- **`runtime/container` BYOB server** — rewritten from axum/HTTP route handlers to transport-neutral `ByobApi<E>` JSON-RPC dispatcher. 7 RPC methods: `byob.deploy`, `byob.list_deployments`, `byob.get_deployment`, `byob.stop_deployment`, `byob.get_resource_usage`, `byob.health`, `byob.info`.
- **Transport layer** — `ByobJsonRpcServer` uses Unix domain sockets (primary) + TCP fallback. Newline-delimited JSON-RPC 2.0. No HTTP dependency.
- **Dependencies excised** — `axum` removed from `runtime/container/Cargo.toml`. `tower` dev-dependency removed. `cargo tree -i axum` returns "did not match any packages".
- **Tests rewritten** — `byob_routes_coverage_tests.rs` converted from HTTP integration to direct JSON-RPC dispatcher tests.
- **HTTP is songBird's domain** — documented in comments across workspace.

### Darwin/graftGate Fix (upstream merge)
- **`#[cfg(unix)]` → `#[cfg(target_os = "linux")]`** — silicon_registry/silicon_discovery/ipc_watch narrowed from Unix-wide to Linux-only. These modules probe sysfs/procfs which are Linux-specific. Fixes graftGate (aarch64-apple-darwin) compilation. 6 sites fixed across `server/src/background/mod.rs`, `server/src/pure_jsonrpc/handler/mod.rs`, `server/src/pure_jsonrpc/handler/router.rs`.

### Feature-Gating Fix (mdns)
- **`discover_via_mdns()` call sites gated** — `service_discovery/service.rs` match arms for `DiscoveryMethod::Mdns` now have `#[cfg(feature = "mdns")]` / `#[cfg(not(feature = "mdns"))]` arms. Import was already gated but call sites weren't.
- **Discovery engine warnings fixed** — unused `HTTP_PROTOCOL` import (mdns-only) and unnecessary `mut` (mdns-only push) resolved.

### Deep Debt: Lock-Poisoning Alignment
- **`security/monitoring`** — 5 `.expect("lock poisoned")` → `.unwrap_or_else(|e| e.into_inner())`.
- **`runtime/adaptive`** — 5 `.expect("lock poisoned")` → `.unwrap_or_else(|e| e.into_inner())`.
- Aligns with server-established pattern: recover from poisoned lock instead of panicking.

## Remaining (Tier 2 / Deep Debt)

| Item | Status | Notes |
|------|--------|-------|
| **Gossip injection (0/17)** | Coordinate with swarmVine team | Spec exists (`GOSSIP_EVENTS.md`), zero production injection code. swarmVine socket discovery needed. |
| **`String` param modernization** | P2 | BDF/device-key APIs (`ember`, `cylinder`, `gpu`) → `impl AsRef<str>` |
| **parking_lot evaluation** | P3 | Not currently a workspace dep; server+GPU coordinator would benefit |
| **Deprecated module cleanup** | P3 | `distributed` legacy features, `protocols` legacy clients have removal timeline |

## Verification

- `cargo check --workspace` — 0 errors, 0 warnings
- `cargo test --workspace --lib` — all pass (1 pre-existing NPU hardware test excluded)
- `cargo tree -i axum` — "did not match any packages" (fully excised)
- Rust edition 2024 throughout
- Zero `tokio::fs`, zero `extern crate`, zero non-comment axum references
