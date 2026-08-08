# TOADSTOOL — S365 Handoff: G68 Complete (Platform Containment)

**Sprint**: S365
**Date**: Aug 7-8, 2026
**Gate**: strandGate (eastGate)
**Upstream**: overwatch audit via golgiBody

---

## Summary

G68 Platform Substrate Abstraction is **COMPLETE**. All `rustix::` syscall
imports now live exclusively in `toadstool-hw-safe` — the single platform
containment crate. Every other crate in the workspace routes Linux-specific
operations through hw-safe's safe wrappers.

**Before S365**: 25+ files across 8 crates imported rustix directly.
**After S365**: 0 code imports outside hw-safe. Only doc-comments remain.

---

## New hw-safe APIs (S365)

| API | Replaces |
|-----|----------|
| `LinuxPrivilegeProbeBackend` | `rustix::thread::capabilities` |
| `LinuxFilesystemIsolation` | `rustix::mount::*` |
| `LinuxDeviceIo::{read,write,pread,pwrite,poll_read}` | `rustix::io::*` |
| `vfio_bar_map` / `vfio_bar_unmap` | `rustix::mm::mmap` (SHARED RW) |
| `mmap_device` / `munmap_device` | General `rustix::mm::mmap` |
| `lock_memory` / `unlock_memory` | `rustix::mm::mlock` |
| `pipe_cloexec` | `rustix::pipe::pipe_with` |
| `fork` / `exit_group` / `kill_process` / `waitpid_nohang` | `rustix::runtime::*` + `rustix::process::*` |
| `recv_with_fds` / `sendmsg_with_fds` | `rustix::net::recvmsg` + SCM_RIGHTS |
| `unix_dgram_socket` | `rustix::net::socket` |
| `mknod_char` | `rustix::fs::mknodat` |
| `open_path` | `rustix::fs::open` |
| `fs_stats` | `rustix::fs::statvfs` |
| `clock_monotonic_ns` | `rustix::time::clock_gettime` |
| `seek_end` | `rustix::fs::seek` |
| `getpid` / `send_signal` | `rustix::process::*` |
| `ioctl_infra::{Ioctl,Getter,Setter,Updater,opcode,...}` | `rustix::ioctl::*` |
| `finit_module` / `delete_module` | `rustix::system::*` |

---

## Crates Migrated

| Crate | Files | Notes |
|-------|-------|-------|
| `toadstool-security-sandbox` | 3 | proc, privilege, mod — rustix fully removed |
| `akida-driver` | 4 | io, mmio, backends/mmap, vfio/dma — rustix fully removed |
| `toadstool-cylinder` | 18+ | irq, dma, ioctl, isolation, ember, drm, types, device, clutch, pmc, guarded_sysfs, rm_trigger, sovereign, amd |
| `nvpmu` | 1 | vfio.rs — rustix fully removed |
| `toadstool-runtime-display` | 1 | v4l2/ioctl.rs — rustix fully removed |
| `toadstool-sysmon` | 1 | disk.rs — rustix fully removed |
| `toadstool-monitoring` | 1 | reporting.rs — rustix fully removed |
| `toadstool-server` | 2 | systemd_fdstore, kernel_sentinel — rustix fully removed |
| `toadstool-cli` (tests) | 2 | e2e tests — rustix fully removed |

---

## Quality Gates

- `cargo check --workspace`: PASS
- `cargo fmt --check`: PASS
- `cargo clippy --workspace -D warnings`: PASS (pre-existing akida-driver lints only)
- Zero `use rustix::` outside hw-safe
- Zero production TODO/FIXME/HACK

---

## Architecture Impact

The porting contract for new architectures is now enforceable:
- **darwinGate** (G12): implement hw-safe backends using `IOKit`/`Mach`
- **riscGate** (G42): implement hw-safe backends using Linux on RISC-V
- **No consumer crate needs modification** — they use trait objects and hw-safe wrappers

---

## For Upstream Overwatch

- G68 violations: **0** (was 25+)
- Platform containment: **complete**
- Ready for depot rebuild
