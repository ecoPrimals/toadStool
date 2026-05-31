# ToadStool S282 — Deep Debt Evolution V: Complete Unsafe Hardening + Env Centralization + Panic Elimination

**Date**: May 28, 2026
**Session**: S282
**Scope**: Comprehensive deep debt pass — unsafe SAFETY documentation, production panic elimination, env::var centralization, libc mmap migration, clippy hardening, idiomatic Rust evolution

---

## Actions Completed

### 1. libc::mmap → rustix::mm Migration
- **rm_trigger.rs**: Migrated `libc::mmap`/`libc::munmap` BAR0 MMIO operations to `rustix::mm::mmap`/`rustix::mm::munmap`
- Removed `libc::O_RDONLY` custom flags (unnecessary with rustix)
- Removed unused `std::os::unix::fs::OpenOptionsExt` import
- **Zero `libc::` references remain in workspace**

### 2. Unsafe SAFETY Documentation — 28 Gaps Closed (12 files)

**`/// # Safety` on unsafe functions (11 ioctl trait impls + 1 stub):**
- `rm_trigger.rs`: RmRawIoctl, RmIoctl `output_from_ptr`
- `vfio/ioctl.rs`: VfioIoctlReturn, VfioIoctlPtr `output_from_ptr`
- `drm.rs`: DrmIoctlCmd `output_from_ptr`
- `nvpmu/src/vfio.rs`: VfioPtrIoctl `output_from_ptr`
- `hw-safe/src/vfio_setup.rs`: VfioReturnIoctl, VfioPtrIoctl `output_from_ptr`
- `hw-safe/src/vfio_dma.rs`: DmaMapIoctl, DmaUnmapIoctl `output_from_ptr`
- `hw-learn/src/applicator/nouveau_drm.rs`: DrmIoctl `output_from_ptr`
- `cache_ops.rs`: non-x86_64 `cache_line_flush` stub

**`// SAFETY:` on unsafe blocks (16 blocks):**
- `pmc.rs`: volatile reads for new_en, pending; munmap
- `mapped_bar.rs`: mmap in from_sysfs_rw; MmioRegion::new
- `isolation.rs`: 7 nested MMIO blocks in fork closures
- 3 boot bins: Bar0::map call sites

### 3. Production Panic Paths → Result Propagation (4 fixes)

| Location | Before | After |
|----------|--------|-------|
| `catalyst_watchdog.rs` | `.expect("failed to spawn")` | `start_watchdog_thread() → std::io::Result<()>` |
| `akida-driver/mmio.rs` | `read32`/`write32` `.expect()` | `try_read32`/`try_write32` returning `Result<_, MmioError>` |
| `cpu_resource.rs` | Rayon pool `.expect("CRITICAL")` | Graceful fallback chain: new → new_fallback → process-wide degraded |
| `unified_memory/lifecycle.rs` | `assert!` + `.expect()` | `BufferError` enum with `validate_creation_params` |

### 4. Env Centralization — 97% Complete

**+56 new socket_env constants** across categories:
- Monitoring/observability (7), TLS/certs (3), client config (4)
- Discovery/auto-config (8), IPC/biomeOS (5), profiler (6)
- Substrate detection (5), auth (1), ember (1), GPU testing (1)
- Cross-platform (3), mainframe (2), external SDK (10)

**110 raw env::var sites migrated** across 46 files spanning:
- core/common (11), core/config (5), core/ember (1), core/toadstool (6)
- auto_config (2), cli (2), client (2), distributed (7)
- integration (3), neuromorphic (1), runtime (4), security (1), testing (1)

### 5. Clippy Hardening

**8 cylinder lib errors fixed:**
- Raw pointer cast constness in pmc.rs → `.cast::<u8>()`
- Collapsible else-if in driver_ops.rs
- `from_str` shadowing `FromStr` trait → proper `impl std::str::FromStr` for `PatchStrategy`
- Needless borrow in sovereign_handoff/types.rs

**13 server warnings fixed:**
- Dead code annotations for watchdog heartbeat, exclude_bdf
- Redundant closures → function pointers (PoisonError::into_inner)
- `.clone()` on Copy type → deref

### 6. Idiomatic Rust Evolution
- `PatchStrategy::from_str` → `impl std::str::FromStr` with `.parse()` at call site
- Removed `RM_ALLOC_CMD`/`RM_CTRL_CMD` references → `RM_ALLOC_OP`/`RM_CTRL_OP`
- Removed stale `errno` from rm_alloc diagnostic output

### 7. Root Docs + Debris Cleanup
- README.md, DOCUMENTATION.md, NEXT_STEPS.md, .env.example → S282
- 7 orphan files deleted (~1,400 lines dead code)
- tools/rm_trigger binary removed (Rust replacement canonical since S278)
- 4 active handoffs archived to archive/
- cargo clean

---

## Metrics

| Metric | Value |
|--------|-------|
| Files changed | 104 |
| Lines added | 954 |
| Lines removed | 354 |
| Orphan files deleted | 7 (~1,400 lines) |
| Clippy warnings | 0 (workspace-wide) |
| Lib tests | 178 pass, 0 fail |
| libc references | 0 |
| Unsafe without SAFETY | 0 |
| Production panics in lib | 0 |
| Env centralization | ~97% |

---

## Downstream: primalSpring Audit Surface

For primalSpring Wave 60+ audit:
- **Zero libc** — all hardware I/O via rustix (mmap, ioctl, fs, mm)
- **Zero unsafe without SAFETY docs** — all 46 blocks documented
- **Zero production panics** — 4 high-risk paths evolved to Result
- **~97% env centralized** — <10 raw env::var remaining (niche third-party)
- **Full clippy -D warnings** clean across workspace
- **PatchStrategy** now uses idiomatic `FromStr` trait

Remaining deep debt for future sprints:
- 21 files >800L (14 production, 7 test) — mostly cylinder VFIO single-concern sequences
- Test suite consolidation (sprint-dated test files could be merged)
- `#[allow(dead_code)]` attrs in cylinder Phase C files (post-S278)
