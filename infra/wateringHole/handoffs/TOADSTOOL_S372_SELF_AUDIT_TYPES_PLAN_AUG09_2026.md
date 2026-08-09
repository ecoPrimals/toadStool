# ToadStool S372 Self-Audit + Types Extraction Plan

**Date**: Aug 9, 2026 | **Sprint**: S372 | **Wave**: 157a Vertebrate Evolution
**Status**: Self-audit COMPLETE. Types extraction Phase 1+2 COMPLETE.

---

## Self-Audit Results

**126/126 methods — zero divergence.**

| Source | Methods | Alignment |
|--------|---------|-----------|
| `DIRECT_JSONRPC_METHODS` (server handlers) | 112 | ✓ all in registry |
| `ANNOUNCED_METHODS` (biomeOS announcement) | 46 | ✓ all in registry |
| `capability_registry.toml` | 126 | ✓ all implemented |

### Fix Applied
14 methods in `science.*` and `inference.*` namespaces were announced to biomeOS but missing from `capability_registry.toml`. Added:
- `[capabilities.science]`: 10 methods (compute/gpu/npu/substrate domain aliases)
- `[capabilities.inference]`: 4 methods (model lifecycle)

Registry bumped to v0.2.1.

---

## Types Extraction — EXECUTED

### Phase 1: `workload/` module (3,095 lines, 17 files)
Moved from `crates/core/toadstool/src/workload/` → `crates/toadstool-core/src/workload/`.
Main crate re-exports via `pub use toadstool_core::workload::*;`.

### Phase 2: resources + security + encryption + execution types
- `resources/types/` (501L) → `crates/toadstool-core/src/resources/`
- `security/{types,context,policy}` (297L) → `crates/toadstool-core/src/security/`
- `encryption/{security,types,config,error}` → `crates/toadstool-core/src/encryption/`
- `execution/` types (260L) → `crates/toadstool-core/src/execution/`

Main crate re-exports all via `pub use toadstool_core::...::*;`. Zero downstream breakage.

### Dead Deps Removed from Main Crate
- `zeroize` (moved to toadstool-core)
- `humantime-serde` (moved to toadstool-core)

### Dependencies Added to `toadstool-core`
```
serde_json, bytes, uuid, humantime-serde, zeroize
```
All pure Rust, WASM-safe. `uuid/v4` feature-gated behind `runtime` (getrandom-safe).

### Result
- `toadstool-core` now ~5K lines of pure types, fully WASM-capable
- `cargo check -p toadstool-core --target wasm32-unknown-unknown --no-default-features` passes
- Full workspace compiles cleanly (zero errors, zero TODO/FIXME)
- 5.2 GiB reclaimed from `cargo clean`

---

## Architecture After Extraction

```
toadstool-core (WASM-capable, ~5K lines)
├── hardware types (existing)
├── workload types (extracted)
├── execution types (extracted)
├── resources types (extracted)
└── security types (extracted)

toadstool (native-only, ~53K lines)
├── re-exports from toadstool-core
├── async runtime engines
├── IPC/networking
└── server infrastructure

management-*, security-policies
├── depends on toadstool-core (not toadstool)
└── WASM-capable
```
