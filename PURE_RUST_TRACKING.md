# Pure Rust Tracking — C Dependency Elimination

**Last Updated**: March 21, 2026 (S162)

## Goal

Eliminate all C dependencies from toadStool, creating a pure Rust binary that
makes raw Linux syscalls. Analogous to DNA: one unified language, one type system,
one compilation model.

## Phase 1: sysinfo Elimination — COMPLETE

| Item | Status | Date |
|------|--------|------|
| Create `toadstool-sysmon` crate | Done | Mar 9, 2026 |
| Migrate 22+ call sites across 18 files | Done | Mar 9, 2026 |
| Remove `sysinfo` from all 15 Cargo.toml | Done | Mar 9, 2026 |
| Verify `cargo tree` has no sysinfo | Done | Mar 9, 2026 |

**Result**: sysinfo (15 transitive crates, libc FFI) fully replaced by
`toadstool-sysmon` (pure Rust, /proc parsing + rustix statvfs).

## Phase 2: Ecosystem Tracking

### Remaining `libc` Paths (via `cargo tree --invert libc`)

| Crate | Version | libc Usage | Upstream Issue | ETA |
|-------|---------|------------|----------------|-----|
| **mio** | 1.1.1 | Event polling (epoll) | tokio-rs/mio#1735 | When mio switches to rustix |
| **tokio** | 1.50 | Via mio + signal-hook-registry | Tracks mio | When mio migrates |
| **evdev** | 0.13.2 | Via nix | No rustix alternative yet | Needs contributor |
| **drm** (via display) | 0.14.2 | drm-sys → libc | Partial rustix adoption | Tracking upstream |
| **wgpu-hal** | 22.0 | GPU platform libs | Will be replaced by coralDriver | Long-term |
| **nix** | 0.29 | libc wrapper | Being replaced by rustix ecosystem-wide | Ongoing |
| **caps** | 0.5.6 | Linux capabilities | Consider rustix alternative | P3 |
| **console** | 0.15 | Terminal detection | is-terminal uses libc | May switch to rustix |
| **getrandom** | 0.2-0.4 | Entropy source | Uses libc for /dev/urandom | May switch to rustix |
| **parking_lot** | 0.9 | Futex | libc for syscalls | May switch to rustix |
| **socket2** | 0.5-0.6 | Socket creation | libc for socket syscalls | Tracks mio |
| **signal-hook-registry** | 1.4 | Signal handling | libc for sigaction | Tracks tokio |
| **polling** | 2.8 | Legacy polling backend | Being replaced by mio | N/A |

### What We Control (Phase 1 — Done)

- `toadstool-sysmon`: Pure Rust /proc parsing (replaced sysinfo)
- `akida-driver`: Uses rustix for VFIO ioctls and mmap (libc eliminated)
- All direct libc calls in our code: **zero**

### What We Track (Phase 2 — Ongoing)

- **mio → rustix**: The biggest win. When mio adopts rustix, tokio's event loop
  becomes pure Rust on Linux automatically. Track tokio-rs/mio#1735.
- **drm → rustix**: drm crate partially uses rustix but drm-sys still pulls libc.
  Track upstream for full migration.
- **evdev → rustix**: No rustix backend exists yet. Consider contributing a minimal
  rustix-based evdev wrapper when display becomes a priority feature.

### What Rust Evolves (Phase 3 — Language Level)

When Rust's std adopts `linux-raw-sys` internally:
- musl becomes unnecessary for Linux targets
- `cargo build --target aarch64-unknown-linux` just works, no packages
- Android builds work without NDK (same syscall ABI)

Our action: Nothing — we automatically benefit once Phases 1+2 are done.

## Phase 4: Endgame — Sovereign Binary

```
cargo build --target aarch64-unknown-linux-gnu    # ARM64 server
cargo build --target armv7-unknown-linux-gnueabihf # Raspberry Pi
cargo build --target aarch64-linux-android         # Android
cargo build --target riscv64gc-unknown-linux-gnu   # RISC-V
```

No C compiler. No musl-tools. No NDK. Just Rust.

## Feature-Gated C (Intentional, Opt-in)

| Crate | Feature Gate | Reason |
|-------|-------------|--------|
| pyo3 | `python-embedded` | Python C API is inherent |
| esp-idf-sys | `esp32` (excluded crate) | ESP32 SDK is C |
| core-foundation-sys | `macos-sandbox` | macOS only |
| wgpu | default (GPU) | Long-term replaced by coralDriver |

## Verification Commands

```bash
# Verify sysinfo is gone
cargo tree --workspace | rg sysinfo
# Should return nothing

# Check remaining libc paths
cargo tree --workspace --invert libc | rg 'libc v|^[├└]'
# Should only show ecosystem deps (mio, tokio, wgpu, etc.)

# Cross-compile check (no musl-tools needed for our code)
cargo check --target aarch64-unknown-linux-gnu
```
