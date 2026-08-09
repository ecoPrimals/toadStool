# ToadStool S372 Self-Audit + Types Extraction Plan

**Date**: Aug 9, 2026 | **Sprint**: S372 | **Wave**: 157a Vertebrate Evolution
**Status**: Self-audit COMPLETE. Extraction plan documented.

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

## Types Extraction Plan (Next Sprint)

### Problem
The main `toadstool` crate (56K lines, 65 files using tokio) is depended on by 4 management/security crates that only need ~3,035 lines of pure types. This blocks those crates from WASM compilation.

### Extraction Target: ~3,035 lines → `toadstool-core`

| Module | Lines | Types needed by downstream |
|--------|-------|---------------------------|
| `workload/` | 2,030 | `WorkloadSpec`, `ExecutableSource`, `WasmModuleSource`, `WorkloadType` |
| `resources/types/` | 501 | `RuntimeMetrics`, `CpuMetrics`, `MemoryMetrics`, `StorageMetrics`, `NetworkMetrics`, `TimingMetrics`, `ResourceRequirements` |
| `security/types+context+policy` | 297 | `Capability`, `IsolationLevel`, `SecurityContext` |
| `execution/` (types only) | 260 | `RuntimeType`, `ExecutionRequest`, `ExecutionStatus`, `ExecutionResponse` |

### Downstream Crates Unblocked
- `toadstool-management-performance`
- `toadstool-management-monitoring`
- `toadstool-management-analytics`
- `toadstool-security-policies`

### Strategy
Expand `toadstool-core` (already WASM-capable, 1,899 lines) to include platform types. Main `toadstool` crate re-exports from `toadstool-core`. Downstream crates switch from `toadstool` → `toadstool-core` dependency.

### Dependencies Added to `toadstool-core`
```
serde_json, bytes, uuid, humantime-serde
```
All pure Rust, WASM-safe. Zero tokio.

### Expected Result
28/48 crates WASM-capable (from current 24/48).

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
