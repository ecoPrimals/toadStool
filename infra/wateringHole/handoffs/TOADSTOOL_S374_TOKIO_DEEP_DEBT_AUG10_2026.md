# ToadStool S374 — Tokio Deep Debt Evolution: `runtime` Feature Gate + Needless Async Removal

**Date**: Aug 9–10, 2026
**Sprint**: S374
**Gate**: strandGate (eastGate overwatch)
**Status**: COMPLETE — all quality gates green

---

## Summary

Major architectural evolution addressing the deepest technical debt in toadStool: the unconditional `tokio` dependency in `crates/core/toadstool/` (236 files, 51K lines). This was the single hardest blocker preventing 24 workspace crates from compiling on WASM.

Three-phase approach:
1. **RwLock migration** — `tokio::sync::RwLock` → `std::sync::RwLock` in types-only files
2. **`runtime` feature gate** — Optional tokio behind `#[cfg(feature = "runtime")]`
3. **Downstream unblocking** — `default-features = false` on consumer crates

Additionally: comprehensive needless-async removal from cargo-cult patterns predating `std::future` stabilization.

---

## Phase 1: RwLock Migration

Migrated `tokio::sync::RwLock` to `std::sync::RwLock` across 34+ types-only files where locks are held briefly for config reads, flag checks, or cache lookups — no `.await` while locked.

**Pattern**: `.read().await` → `.read().unwrap_or_else(|e| e.into_inner())` (poison-tolerant)

**Guard-across-await fixes**: In 3 files where `std::sync::RwLock` guards were inadvertently held across `.await` points (causing `!Send` errors in `tokio::spawn`), refactored to clone-before-await:
- `universal/registry.rs` — `route_request()` now scopes guard, clones provider
- `universal/scheduler/execution/native.rs` — scoped engine lookup
- `universal/scheduler/execution/wasm.rs` — scoped engine lookup

**Mutex migration**: `tokio::sync::Mutex` → `std::sync::Mutex` in 2 biomeos inmemory backends (agent, storage) — brief lock for cache ops.

## Phase 2: `runtime` Feature Gate

Added to `crates/core/toadstool/Cargo.toml`:
```toml
[features]
default = ["runtime", "mdns"]
runtime = ["dep:tokio", "dep:toadstool-sysmon", "toadstool-common/runtime",
           "toadstool-common/mdns", "toadstool-common/btsp",
           "toadstool-config/runtime", "toadstool-core/runtime", "uuid/v4"]
```

Gated modules behind `#[cfg(feature = "runtime")]`:
- `ipc/`, `ipc_helpers/`, `launcher` — Unix/TCP networking, process spawning
- `discovery/`, `ecosystem/` — mDNS, service discovery, ecosystem coordination
- `runtime_discovery/` — background runtime discovery
- `deployment_layer/detector` — fs/process probing
- `layer_adaptation/detection` — runtime detection
- `byob/resource_metrics`, `byob/byob_impl` — background monitoring
- `performance_hardening/async_ops`, `performance_hardening/caching` — background tasks
- `production_hardening/circuit_breaker`, `production_hardening/resource_leak` — spawn+interval
- `security_hardening/intrusion` — background IDS
- `resources/monitoring` — `SystemResourceMonitor`
- `universal/scheduler/execution/native` — process execution

**Stubs**: `#[cfg(not(feature = "runtime"))]` stubs return defaults or typed errors. `generate_uuid()` helper returns `Uuid::nil()` on WASM, `Uuid::new_v4()` on native.

## Phase 3: Downstream Unblocking + Needless Async Removal

**Downstream crates** updated to `default-features = false`:
- `toadstool-integration-primals` — added own `runtime` feature, `register_with_orchestrator` gated
- `toadstool-management-analytics` — already had `default-features = false`
- `toadstool-security-policies` — already had `default-features = false`
- `toadstool-runtime-specialty` — already had `default-features = false`

**Needless async removal** — Functions marked `async` that never `.await`, dating from before `std::future` stabilization:
- `SecurityProvider` trait — all 6 methods: `async fn` → `fn`
- `CryptoProviderRegistry` — 6 methods: `register`, `unregister`, `find_provider`, `find_all_providers`, `get_provider`, `list_providers`
- `RuntimeOrchestrator` / `EngineRegistry` — `register_engine`, `select_runtime`, `select_intelligent_backend`
- `UniversalPrimalRegistry` — `register_primal`, `find_by_capability`, `find_by_context`
- `UniversalScheduler` — `register_runtime_engine`, `available_runtimes`, `get_active_job_count`
- `UniversalComputePlatform` — `register_runtime_engine`, `get_available_runtimes`, `find_primals_by_capability`
- `EncryptionContext::discover_provider`

**RwLock audit result**: 7 remaining `tokio::sync::RwLock` usages all legitimately hold guards across `.await` (background task loops). All gated behind `#[cfg(feature = "runtime")]`.

---

## Results

| Criterion | Result |
|-----------|--------|
| `cargo check -p toadstool --no-default-features --target wasm32-unknown-unknown` | **PASS** |
| `cargo check --workspace` | **PASS** |
| `cargo test -p toadstool` | **PASS** — 15 passed, 0 failed |
| WASM-capable crates (`--no-default-features`) | **26/48** (up from 13 with default features) |
| Test regressions | **Zero** |

### WASM-capable crates (26/48 with `--no-default-features`)

`toadstool`, `toadstool-common`, `toadstool-config`, `toadstool-core`, `toadstool-sysmon`, `toadstool-hw-safe`, `hw-learn`, `toadstool-ember`, `toadstool-glowplug`, `toadstool-cylinder`, `nvpmu`, `toadstool-integration-primals`, `toadstool-runtime-orchestration`, `toadstool-runtime-universal`, `toadstool-integration-security`, `toadstool-management-resources`, `toadstool-runtime-adaptive`, `toadstool-runtime-secure-enclave`, `akida-chip`, `akida-driver`, `akida-models`, `akida-reservoir-research`, `akida-setup`, `cross-substrate-validation`, `neurobench-runner`, `toadstool-security-monitoring`

---

## Also Delivered: Node Atomic AAR + Silicon Registry (S374a)

- **Silicon discovery** — `silicon_discovery.rs` in `toadstool-core`: queries coralReef `shader.compile.capabilities` at startup to populate `SiliconRegistry` with `ShaderCompilerStatus`
- **Confirmed** `SiliconCapabilities` in `toadstool-core` already serves the `silicon_capability_registry` role — no absorption needed, already native
- **Dispatch descriptor wiring** — `ShaderInfo` in `cylinder` already consumed by dispatch

---

## For Upstream Overwatch

### Gaps Identified

1. **22 crates still fail WASM** — all directly depend on `tokio` (runtime crates: gpu, container, wasm, edge, native, display, etc.) or `socket2`/`mio`. These are inherently native-only deployment crates.
2. **Feature unification** — On native builds, Cargo's feature unification ensures `runtime` is enabled transitively. Zero functionality loss for server/CLI.
3. **`uuid` on WASM** — `v4` feature requires RNG, disabled for types-only WASM builds. `generate_uuid()` returns `Uuid::nil()` on WASM (acceptable for types-only compilation).
4. **Pre-existing `cylinder`/`vfio` errors** in `toadstool-examples` — unrelated to this work, from prior refactoring.

### Clean State

- Zero test regressions
- Full workspace builds clean (excluding pre-existing examples issue)
- WASM compilation verified for toadstool core + 25 additional crates
- All feature gates use established `toadstool-common`/`toadstool-config` pattern
