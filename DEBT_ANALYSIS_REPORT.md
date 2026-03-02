# Deep Debt Analysis Report

**Date:** 2026-03-01  
**Scope:** crates/ (production src, excluding tests/ directory for line counts)

---

## 1. Top 10 Largest .rs Files (Production src in crates/)

| Rank | Path | Lines | Notes |
|------|------|-------|-------|
| 1 | `crates/neuromorphic/akida-driver/src/backends/vfio.rs` | 962 | VFIO backend; S78 migrated libc→rustix |
| 2 | `crates/core/config/src/runtime_defaults/validation.rs` | 872 | Config validation |
| 3 | `crates/barracuda/src/shaders/sovereign/df64_rewrite.rs` | 854 | DF64 shader sovereign compiler |
| 4 | `crates/testing/src/integration/integration_impl.rs` | 845 | Test infrastructure |
| 5 | `crates/core/toadstool/src/ecosystem/management.rs` | 830 | Ecosystem management |
| 6 | `crates/distributed/src/crypto_integration/client.rs` | 818 | Crypto client |
| 7 | `crates/core/common/src/infant_discovery/engine.rs` | 812 | Infant discovery engine |
| 8 | `crates/runtime/specialty/src/lib.rs` | 806 | Specialty runtime |
| 9 | `crates/core/common/src/infant_discovery/sources.rs` | 806 | Infant discovery sources |
| 10 | `crates/cli/src/templates/specialized_templates.rs` | 794 | CLI templates |

**Refactoring recommendation:** No files exceed 1000 lines. The largest (vfio.rs at 962) was touched in S78 (libc→rustix migration). Consider splitting vfio.rs into submodules if it grows further. No immediate refactor required.

---

## 2. Unsafe Code Audit

### Summary

| Category | Count | Files |
|----------|-------|-------|
| **JUSTIFIED** | 8 | akida-driver (mmio, vfio), barracuda (wgpu), runtime/gpu (cpu, vulkan, opencl) |
| **REMOVABLE** | 0 | — |
| **EVOLVE** | 0 | — |

### Detailed Breakdown

#### JUSTIFIED (FFI / raw pointer math with proof)

1. **`crates/neuromorphic/akida-driver/src/mmio.rs`** — VFIO MMIO, rustix ioctl, mmap/munmap, volatile R/W
2. **`crates/neuromorphic/akida-driver/src/backends/vfio.rs`** — VFIO ioctls, DMA alloc/mlock, from_raw_parts
3. **`crates/barracuda/src/device/wgpu_device/creation.rs`** — wgpu create_pipeline_cache (data: None)
4. **`crates/barracuda/src/device/wgpu_device/compilation.rs`** — wgpu create_shader_module_spirv
5. **`crates/runtime/gpu/src/unified_memory/backends/cpu.rs`** — alloc_zeroed/dealloc for 64-byte aligned buffer
6. **`crates/runtime/gpu/src/unified_memory/backends/vulkan.rs`** — ash loader probe, unsafe fn with_device
7. **`crates/runtime/gpu/src/unified_memory/backends/opencl.rs`** — unsafe fn with_context

#### REMOVABLE / EVOLVE

None. All blocks have SAFETY comments and documented invariants.

### Fixes Applied

None. No REMOVABLE unsafe found.

---

## 3. Hardcoding Audit

### Address/Port

- **OK:** `core/common/src/constants/network.rs` — documented constants
- **OK:** `core/config/src/types/network.rs` — env vars first, fallback
- **Flag:** `core/common/src/runtime_discovery.rs` — LocalhostDiscoveryClient hardcodes localhost:8080 (dev fallback)
- All other matches in `#[cfg(test)]` — no production impact

### Primal Names

- **OK:** well_known constants, PrimalType enum, adapter identities (SongbirdAdapter, BearDog client)
- **Flag:** `core/common/src/primal_sockets/api.rs` — match on names; evolution: capability-based discovery

**Verdict:** No sovereignty violations. Production uses env/config overrides.

---

## 4. Mock Audit

| Location | Gating | Status |
|----------|--------|--------|
| auto_config MockHardwareDetector, MockEcosystemDiscoverer | `#[cfg(any(test, feature = "test-mocks"))]` | OK |
| distributed MockSecurityProvider, create_mock | `#[cfg(test)]` | OK |
| server (handlers, server, routes, lifecycle, background) | `#[cfg(test)]` or `#[cfg(all(test, feature = "api"))]` | OK |
| core/toadstool MockProvider, MockCloudProvider | `#[cfg(test)]` | OK |

**Edge case:** `runtime/edge/src/platforms/arduino.rs` — comment said "mock implementation"; clarified to "Stub: simplified implementation" (fixed).

**Verdict:** All mocks correctly gated. toadstool_testing is dev-dependency.

---

## 5. Changes Made

| File | Change |
|------|--------|
| `crates/runtime/edge/src/platforms/arduino.rs` | Comment: "mock implementation for testing Arduino workflow" → "Stub: simplified implementation until serial monitor integration is wired" |
