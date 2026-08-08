# toadStool Cross-Architecture Support

**Status**: 15/15 targets pass `cargo check --workspace`
**Sprint**: S369 — First primal to full cross-arch compilation

## Supported Architectures

### Tier 1 — Primary (CI-validated, depot builds)

| Target | OS | Arch | Notes |
|--------|----|----- |-------|
| `x86_64-unknown-linux-gnu` | Linux | x86_64 | Primary development |
| `x86_64-unknown-linux-musl` | Linux | x86_64 | Static deploy binaries |
| `aarch64-unknown-linux-gnu` | Linux | ARM64 | Graviton, RPi 5, Jetson |
| `aarch64-unknown-linux-musl` | Linux | ARM64 | Static ARM deploy |
| `x86_64-pc-windows-gnu` | Windows | x86_64 | Desktop compute nodes |
| `x86_64-apple-darwin` | macOS | x86_64 | Intel Mac dev |
| `aarch64-apple-darwin` | macOS | ARM64 | Apple Silicon dev |

### Tier 2 — Extended (type-checks, future depot)

| Target | OS | Arch | Notes |
|--------|----|----- |-------|
| `x86_64-pc-windows-msvc` | Windows | x86_64 | MSVC toolchain |
| `aarch64-pc-windows-gnullvm` | Windows | ARM64 | Snapdragon compute |
| `aarch64-linux-android` | Android | ARM64 | Pixel / mobile edge |
| `armv7-unknown-linux-gnueabihf` | Linux | ARM32 | IoT edge nodes |
| `riscv64gc-unknown-linux-gnu` | Linux | RISC-V 64 | SiFive, StarFive |
| `powerpc64le-unknown-linux-gnu` | Linux | POWER9/10 | IBM HPC |
| `s390x-unknown-linux-gnu` | Linux | IBM Z | Mainframe compute |
| `loongarch64-unknown-linux-gnu` | Linux | LoongArch | Loongson sovereign |

## Architecture Decisions

### Hardware-dependent code layering

```
Layer 0 (Pure Rust)     — unconditional, all platforms
Layer 1 (Memory Mgmt)   — types unconditional, constructors return Err(Unsupported) on non-Linux
Layer 2 (Kernel ABI)    — types unconditional, functions #[cfg(target_os = "linux")]
systemd_fds             — #[cfg(target_os = "linux")] at module level
```

### Platform gating rules

- **Never** `#[cfg(unix)]` for Linux-specific code (macOS IS unix but has no VFIO/sysfs)
- **Always** `#[cfg(target_os = "linux")]` for kernel interfaces
- **Arch-specific** gates for `rustix::runtime` (fork/exit_group) — not available on ppc64le/s390x/loongarch
- **seccompiler** restricted to x86_64/aarch64 (no s390x/ppc64le/loongarch support)
- **ioctl `Opcode`** is `u32` on most arches but `u64` on powerpc — always cast via `as Opcode`

### Consumer crate gating

Hardware driver crates (`akida-driver`, `akida-models`, `neurobench-runner`) gate their
Linux-dependent code with `#[cfg(target_os = "linux")]` and provide `#[cfg(not(target_os = "linux"))]`
fallback stubs that return appropriate errors.

## Running the cross-arch check

```bash
# Full sweep (all 15 targets)
./scripts/cross-arch-check.sh

# Quick check (Tier 1 only)
./scripts/cross-arch-check.sh quick
```

## Installing targets

```bash
rustup target add \
    x86_64-unknown-linux-gnu x86_64-unknown-linux-musl \
    aarch64-unknown-linux-gnu aarch64-unknown-linux-musl \
    x86_64-pc-windows-gnu x86_64-pc-windows-msvc \
    x86_64-apple-darwin aarch64-apple-darwin \
    aarch64-linux-android aarch64-pc-windows-gnullvm \
    armv7-unknown-linux-gnueabihf riscv64gc-unknown-linux-gnu \
    powerpc64le-unknown-linux-gnu s390x-unknown-linux-gnu \
    loongarch64-unknown-linux-gnu
```
