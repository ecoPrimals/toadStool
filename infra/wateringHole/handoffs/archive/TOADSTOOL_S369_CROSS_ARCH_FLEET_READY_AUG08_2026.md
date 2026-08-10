# TOADSTOOL — S369 Handoff: Cross-Architecture Fleet Ready

**Sprint**: S366–S369
**Date**: Aug 8, 2026
**Gate**: strandGate (eastGate)
**Upstream**: overwatch audit via golgiBody

---

## Summary

toadStool is the **first primal** to achieve full cross-architecture compilation
across all 16 native OS targets. The Node Atomic hardware fleet is ready:
Mac M4 Mini, Pixel 8 (GrapheneOS), iPhone XS, Milk-V Jupiter 2 (RISC-V),
Steam Deck, Raspberry Pi, and exotic compute (IBM POWER, IBM Z, LoongArch).

**16/16 native targets** pass `cargo check --workspace`.
WASM (Tier 3) documented as compute-only subset.

---

## Sprint Sequence

| Sprint | Work |
|--------|------|
| S366 | musl ioctl fix + libc→rustix migration + L2 false positive resolution |
| S367 | hw-safe restructured: Layer 0 unconditional, Layer 1 stubs, Layer 2 gated |
| S368 | hw-safe Layer 2 internal gating — G68 violations 4→0 |
| S369 | Full 16-target cross-arch sweep + fleet documentation |

---

## Architecture Targets (16/16 PASS)

### Tier 1 — Primary
- `x86_64-unknown-linux-gnu` (dev, Steam Deck, servers)
- `x86_64-unknown-linux-musl` (static deploy)
- `aarch64-unknown-linux-gnu` (Raspberry Pi, Graviton)
- `aarch64-unknown-linux-musl` (static ARM deploy)
- `x86_64-pc-windows-gnu` (Windows desktop)
- `x86_64-apple-darwin` (Intel Mac)
- `aarch64-apple-darwin` (Mac M4 Mini)
- `aarch64-apple-ios` (iPhone XS)

### Tier 2 — Extended
- `x86_64-pc-windows-msvc`
- `aarch64-pc-windows-gnullvm` (ARM64 Windows)
- `aarch64-linux-android` (Pixel 8 / GrapheneOS)
- `armv7-unknown-linux-gnueabihf` (IoT edge)
- `riscv64gc-unknown-linux-gnu` (Milk-V Jupiter 2)
- `powerpc64le-unknown-linux-gnu` (IBM POWER)
- `s390x-unknown-linux-gnu` (IBM Z mainframe)
- `loongarch64-unknown-linux-gnu` (Loongson sovereign)

### Tier 3 — Compute Subset (future)
- `wasm32-unknown-unknown` (WebGPU/browser) — blocked by tokio/mio
- `wasm32-wasip1` (WASI edge) — blocked by socket2/polling

---

## Key Technical Decisions

1. **`#[cfg(unix)]` → `#[cfg(target_os = "linux")]`**: VFIO/sysfs are Linux kernel
   interfaces. macOS is unix but has no VFIO. G68-correct gating.

2. **`rustix::runtime` arch fallbacks**: `fork()`/`exit_group()` unavailable on
   ppc64le/s390x/loongarch. Fallbacks: fork→`Err(Unsupported)`, exit_group→`std::process::exit`.

3. **ioctl `Opcode` portability**: Cast via `as Opcode` not `as u32` — powerpc uses u64.

4. **seccompiler arch restriction**: Only available on x86_64/aarch64. Cargo.toml
   target-gated + code cfg-gated.

5. **Consumer crate stubs**: `#[cfg(not(target_os = "linux"))]` replaces `#[cfg(not(unix))]`
   wherever the real impl is Linux-only.

---

## Patterns for Other Primals

barraCuda and coralReef can adopt this pattern:

```rust
// Hardware-specific modules: target_os gate (not unix!)
#[cfg(target_os = "linux")]
mod vfio_backend;

// Fallback stub for non-Linux
#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("This binary requires Linux hardware access");
    std::process::exit(1);
}
```

See `docs/CROSS_ARCH.md` for full rules and `scripts/cross-arch-check.sh` for verification.

---

## Quality Gates

- `cargo check --workspace` (Linux native): PASS
- `cargo check --workspace --target <all 16>`: 16/16 PASS
- `cargo fmt --check`: 0 diffs
- `cargo clippy -p toadstool-hw-safe -- -D warnings`: 0 warnings
- Pre-existing akida-driver doc warnings: not addressed (unrelated)

---

## Files Changed (S369)

27 files, 444 insertions, 127 deletions:
- `.cargo/config.toml` — cross-linker configs for exotic arches
- `crates/core/hw-safe/src/{platform_backends,drm_ioctl,lib}.rs` — arch fallbacks
- `crates/core/cylinder/src/bin/rm_trigger/{main,rm_ioctl}.rs` — Opcode portability
- `crates/neuromorphic/akida-driver/src/{lib,backend,backends/mod,capabilities,hybrid/mod}.rs` — cfg(unix)→cfg(linux)
- `crates/neuromorphic/{akida-models,neurobench-runner,akida-reservoir-research,cross-substrate-validation,akida-setup}` — stub fixes
- `crates/security/sandbox/{Cargo.toml,src/macos.rs,src/manager.rs,src/linux/mod.rs}` — macOS stub + seccompiler gating
- `crates/management/monitoring/src/platform.rs` — type annotations
- `docs/CROSS_ARCH.md` — architecture support matrix (NEW)
- `scripts/cross-arch-check.sh` — automated tier 1/full sweep (NEW)

---

## Next Steps

- **Tier 3 WASM**: Feature-gate tokio networking out of core crates
- **Other primals**: barraCuda + coralReef adopt cross-arch patterns
- **Depot builds**: CI matrix for all 16 targets
- **Hardware validation**: Deploy on actual fleet hardware
