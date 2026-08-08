# toadStool Cross-Architecture Support

**Status**: 16/16 native targets + 10/48 crates on WASM (Tier 3 active)
**Sprint**: S370 — Tier 3 WASM compute subset achieved
**Philosophy**: If it can run a bin, we can run primals on it.

## Node Atomic Fleet — Actual Hardware

| Hardware | Target | Status |
|----------|--------|--------|
| Development (x86_64 Linux) | `x86_64-unknown-linux-gnu` | Primary |
| Mac M4 Mini | `aarch64-apple-darwin` | Full workspace |
| Pixel 8 (GrapheneOS) | `aarch64-linux-android` | Full workspace |
| iPhone XS | `aarch64-apple-ios` | Full workspace |
| Milk-V Jupiter 2 (RISC-V vector) | `riscv64gc-unknown-linux-gnu` | Full workspace |
| Steam Deck (SteamOS) | `x86_64-unknown-linux-gnu` | Full workspace |
| Raspberry Pi | `aarch64-unknown-linux-gnu` | Full workspace |
| WebGPU/Browser | `wasm32-unknown-unknown` | 10 crates (compute subset) |
| Cloud/WASI edge | `wasm32-wasip1` | 10 crates (compute subset) |

## Supported Architectures

### Tier 1 — Primary (CI-validated, depot builds)

| Target | OS | Arch | Hardware |
|--------|----|------|----------|
| `x86_64-unknown-linux-gnu` | Linux | x86_64 | Dev, Steam Deck, servers |
| `x86_64-unknown-linux-musl` | Linux | x86_64 | Static deploy binaries |
| `aarch64-unknown-linux-gnu` | Linux | ARM64 | Raspberry Pi, Graviton |
| `aarch64-unknown-linux-musl` | Linux | ARM64 | Static ARM deploy |
| `x86_64-pc-windows-gnu` | Windows | x86_64 | Desktop compute nodes |
| `x86_64-apple-darwin` | macOS | x86_64 | Intel Mac |
| `aarch64-apple-darwin` | macOS | ARM64 | Mac M4 Mini |
| `aarch64-apple-ios` | iOS | ARM64 | iPhone XS |

### Tier 2 — Extended (type-checks, future depot)

| Target | OS | Arch | Hardware |
|--------|----|------|----------|
| `x86_64-pc-windows-msvc` | Windows | x86_64 | MSVC toolchain |
| `aarch64-pc-windows-gnullvm` | Windows | ARM64 | Snapdragon compute |
| `aarch64-linux-android` | Android | ARM64 | Pixel 8 (GrapheneOS) |
| `armv7-unknown-linux-gnueabihf` | Linux | ARM32 | IoT edge nodes |
| `riscv64gc-unknown-linux-gnu` | Linux | RISC-V 64 | Milk-V Jupiter 2 |
| `powerpc64le-unknown-linux-gnu` | Linux | POWER9/10 | IBM HPC |
| `s390x-unknown-linux-gnu` | Linux | IBM Z | Mainframe compute |
| `loongarch64-unknown-linux-gnu` | Linux | LoongArch | Loongson sovereign |

### Tier 3 — Compute Subset (WASM — 10 crates pass)

| Target | Runtime | Crates |
|--------|---------|--------|
| `wasm32-unknown-unknown` | Browser/WebGPU | 10/48 (compute core) |
| `wasm32-wasip1` | WASI edge/cloud | 10/48 (compute core) |

**Crates passing on WASM** (compute subset):
- `toadstool-hw-safe` — safe hardware abstraction layer
- `toadstool-core` — core hardware infrastructure types
- `toadstool-sysmon` — system monitoring types
- `toadstool-management-resources` — resource management types
- `toadstool-runtime-secure-enclave` — enclave computation types
- `hw-learn` — vendor-neutral GPU hardware learning
- `nvpmu` — NVIDIA power management logic
- `akida-chip` — NPU chip abstraction
- `akida-models` — NPU model loading/inference
- `akida-setup` — NPU setup utilities

**Key: `toadstool-common` `runtime` feature**

The `runtime` feature (default-enabled) gates tokio and all async networking.
Crates that only need types/traits/constants from `common` use `default-features = false`
to avoid pulling in tokio/mio/socket2 which cannot compile on WASM.

**What won't compile on WASM** (requires OS networking/processes):
- Server, client, CLI — fundamentally need TCP/Unix sockets
- Runtime orchestration — needs process control
- All integration/testing crates — need full OS

## Architecture Decisions

### Hardware-dependent code layering

```
Layer 0 (Pure Rust)     — unconditional, all platforms (including WASM)
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
# Full sweep (all 16 native targets)
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
    aarch64-apple-ios aarch64-linux-android \
    aarch64-pc-windows-gnullvm armv7-unknown-linux-gnueabihf \
    riscv64gc-unknown-linux-gnu powerpc64le-unknown-linux-gnu \
    s390x-unknown-linux-gnu loongarch64-unknown-linux-gnu \
    wasm32-unknown-unknown wasm32-wasip1
```
