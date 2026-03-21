# Unsafe Code Audit Report

**Date:** March 21, 2026 (updated S162)  
**Scope:** All `unsafe` blocks, `unsafe impl`, and `unsafe fn` in `crates/` (excluding `target/`, `archive/`)

---

## Executive Summary

| Category | Count | Action |
|----------|-------|--------|
| **NECESSARY** (hardware/FFI) | ~70+ | No change — all SAFETY-documented |
| **TEST-ONLY** (env::set_var/remove_var) | **0** | All migrated to `temp_env` (S159) |
| **EVOLVABLE** | 0 | All evolved to safe Rust |

---

## 1. NECESSARY – Hardware Access (MMIO, VFIO, DMA, ioctl)

Cannot be made safe; required for low-level hardware interaction.

### neuromorphic/akida-driver
- **mmio.rs**: `MappedRegion` Send/Sync, ioctl, `VolatileSlice::from_raw_parts`, munmap
- **backends/volatile_access.rs**: `VolatileSlice::from_raw_parts`, volatile read/write
- **backends/vfio/dma.rs**: alloc/dealloc, mlock, `BorrowedFd::borrow_raw`, `from_raw_parts`
- **backends/vfio/ioctl.rs**: rustix ioctl calls
- **backends/mmap.rs**: mmap, munmap, `VolatileSlice::from_raw_parts`

### core/nvpmu
- **dma.rs**: alloc, mlock, VFIO ioctl, `from_raw_parts`
- **vfio.rs**: ioctl, `OwnedFd::from_raw_fd`, `read_volatile`/`write_volatile`
- **bar0.rs**: mmap, volatile read/write, munmap

### core/hw-learn
- **nouveau_drm.rs**: DRM ioctl

---

## 2. NECESSARY – FFI (V4L2, DRM, GPU backends)

Cannot be made safe; FFI to kernel/C libraries.

### runtime/display (V4L2)
- **v4l2/device.rs**: `MmapBuffer` Send/Sync, `from_raw_parts`, ioctl, mmap

### runtime/gpu
- **unified_memory/backend.rs**: Vulkan/OpenCL/WebGPU/Cpu allocation Send/Sync
- **unified_memory/buffer.rs**: `UnifiedBuffer` Send/Sync, `from_raw_parts_mut`
- **unified_memory/backends/opencl.rs**: `unsafe fn with_context`
- **unified_memory/backends/vulkan.rs**: `unsafe fn with_device`
- **unified_memory/backends/cpu.rs**: alloc/dealloc, `AlignedBuffer::from_raw`, Send/Sync
- **memory/pinned.rs**: alloc, `from_raw_parts`, dealloc, Send/Sync
- **backends/cuda_impl/kernels.rs**: CUDA FFI
- **backends/opencl_impl/backend.rs**: OpenCL FFI

### runtime/secure_enclave
- **isolated_memory.rs**: alloc, mlock, munlock, dealloc, `from_raw_parts`, Send/Sync

---

## 3. TEST-ONLY – env::set_var / env::remove_var (Rust 2024)

Edition 2024 makes `env::set_var` and `env::remove_var` `unsafe fn` due to potential data races. In tests, this is acceptable when:
- Tests run sequentially (single-threaded)
- Or when wrapped with `temp_env` (safe API)

**Remaining test-only unsafe** (acceptable; async tests or complex setup):
- `server/unibin/execution.rs` – `#[tokio::test]` async tests (temp_env incompatible)
- `server/capabilities/tests/mod.rs` – `with_temp_discovery` async helper
- `server/capabilities/tests/peer_discovery.rs` – async discovery tests
- `server/tests/*`, `core/config/tests/*`, `integration-tests/*`, `cli/tests/*` – various env tests
- `distributed/src/substrate_detection/experimental.rs` – test teardown
- `distributed/src/hosting/recursive.rs` – test env
- `auto_config/*` – test env
- `client/core.rs` – test env
- `core/common/src/constants/network.rs` – test env
- `core/common/src/infant_discovery/sources.rs` – test env

---

## 4. EVOLVABLE → Changes Made

Replaced manual `unsafe { env::set_var/remove_var }` with `temp_env::with_var` / `with_vars` / `with_vars_unset` in **synchronous** tests.

### 4.1 crates/server/src/unibin/format.rs

**Before:** Manual save/restore with `unsafe { env::set_var/remove_var }` in 4 tests.

**After:** All 4 tests use `temp_env`:
- `get_socket_path_tmp_fallback_when_xdg_not_exists` → `temp_env::with_vars`
- `get_socket_path_from_toadstool_socket_env` → `temp_env::with_var`
- `get_socket_path_from_primal_socket_with_family_suffix` → `temp_env::with_vars`
- `get_socket_path_temp_dir_fallback_no_xdg` → `temp_env::with_vars_unset`

**Result:** No unsafe in format.rs tests.

### 4.2 crates/server/src/capabilities/tests/discovery_dir.rs

**Before:** `ENV_MUTEX` + `unsafe { env::set_var/remove_var }` in 4 tests.

**After:** All 4 tests use `temp_env`:
- `test_discovery_directory_structure` → `temp_env::with_var`
- `test_discovery_directory_fallback_when_xdg_unset` → `temp_env::with_var_unset`
- `test_default_socket_path_format` → `temp_env::with_var`
- `test_default_socket_path_fallback` → `temp_env::with_var_unset`

**Result:** No unsafe, no `ENV_MUTEX` in discovery_dir.rs.

### 4.3 crates/core/common/src/discovery_defaults.rs

**Before:** `ENV_MUTEX` + `unsafe { env::set_var/remove_var }` in 10 tests.

**After:** All 10 tests use `temp_env`:
- `test_discovery_config_default_production_mode` → `temp_env::with_var`
- `test_localhost_fallbacks_default_production_mode` → `temp_env::with_var`
- `test_discovery_error_strategy_default_production_mode` → `temp_env::with_var`
- `test_get_fallback_url_env_override` → `temp_env::with_var`
- `test_get_fallback_url_toadstool_env_override` → `temp_env::with_var`
- `test_get_fallback_url_postgres_env_override` → `temp_env::with_var`
- `test_get_fallback_url_mongodb_env_override` → `temp_env::with_var`
- `test_discovery_config_default_staging_env` → `temp_env::with_var`
- `test_discovery_config_default_empty_env` → `temp_env::with_var_unset`
- `test_get_fallback_url_service_type_case_insensitive` → `temp_env::with_var`

**Result:** No unsafe, no `ENV_MUTEX` in discovery_defaults tests.

---

## 5. unsafe impl / unsafe fn Summary

| Location | Purpose |
|----------|---------|
| `akida-driver` | Send/Sync for MappedRegion, DmaBuffer, MmapRegion; Ioctl trait; `from_raw_parts` |
| `runtime/gpu` | Send/Sync for Vulkan/OpenCL/WebGPU/Cpu allocations, UnifiedBuffer, AlignedBuffer, PinnedMemory |
| `runtime/display` | Send/Sync for MmapBuffer |
| `runtime/secure_enclave` | Send/Sync for IsolatedMemoryRegion |
| `core/nvpmu` | Send/Sync for DmaBuffer, VfioBar0Access, Bar0Access; Ioctl trait |
| `core/hw-learn` | Ioctl for DrmIoctl |

All are necessary for FFI/hardware types.

---

## 6. Recommendations

1. **Keep** all hardware/FFI unsafe; it is required for MMIO, VFIO, DMA, ioctl, GPU backends.
2. **Accept** remaining test-only `unsafe { env::set_var/remove_var }` in async tests and complex helpers.
3. **Prefer** `temp_env` for new sync tests that need env vars.
4. **Consider** evolving more sync tests to `temp_env` where feasible (e.g., `core/config/tests/*`, `core/common/src/constants/network.rs`).

---

## 7. No transmute Found

No `transmute` usage in the codebase; no bytemuck/zerocopy migration needed.
