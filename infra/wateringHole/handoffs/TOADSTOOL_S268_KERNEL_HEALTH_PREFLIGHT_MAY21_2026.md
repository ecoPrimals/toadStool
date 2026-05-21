# S268: Kernel Build Environment Health Check

**Date:** May 21, 2026
**From:** toadStool (via hotSpring Exp 216)
**To:** primalSpring (audit)
**Session:** S268
**Status:** Implemented, integrated, all modules clean post-audit

## Summary

New `cylinder::vfio::kernel_health` module detects corrupted kernel build
environments that silently produce broken `.ko` modules. Root cause: a corrupted
`autoconf.h` shifted `struct module` layout by 24 bytes, causing the kernel loader
to see nonzero values in relocation targets and reject the module with misleading
errors.

This is a preflight safety net — fires before DKMS builds and warm handoffs.

## New Surface Area

### Rust API (`toadstool-cylinder`)

```rust
kernel_health::full_kernel_health_check() -> KernelHealthReport
kernel_health::repair_autoconf() -> Result<RepairResult>
kmod::ensure_build_environment_healthy() -> Result<()>
```

### JSON-RPC

```json
{"method": "sovereign.kernel_health", "params": {"repair": false}}
```

Returns: `{ freshness, probe_offsets, reference_offsets, layout_matches, diagnosis }`

### CLI

```bash
toadstool kernel-health              # text report
toadstool kernel-health --format json
toadstool kernel-health --repair     # attempt autoconf restoration
```

### Warm Handoff Integration

`sovereign_handoff.rs` step 0d blocks handoff if `layout_matches == false` for
`Patched` or `DkmsPatched` module sources.

## Detection Layers

| Layer | What | Fallback |
|-------|------|----------|
| 1. Freshness | `autoconf.h` mtime vs kernel image | Best-effort; fresh doesn't mean correct |
| 2. Struct Probe | Compile tiny module, read offsets | Read offsets from existing DKMS module |
| 3. Reference | Parse `.gnu.linkonce.this_module` RELA from installed module | Skip if no reference module found |

## Gaps for Upstream Review

- RPM/pacman repair paths (currently Debian-only `repair_autoconf`)
- NixOS/Guix immutable system short-circuit
- Container DKMS trust override (`--trust-headers`)
- `HealthProbe` trait integration for passive monitoring

## Changed Files

| File | Change |
|------|--------|
| `cylinder/src/vfio/kernel_health.rs` | New: 3-layer health check, repair, 12 tests |
| `cylinder/src/vfio/mod.rs` | `pub mod kernel_health` |
| `cylinder/src/vfio/kmod.rs` | `BuildEnvironmentCorrupted`, `ensure_build_environment_healthy()` |
| `cylinder/src/vfio/sovereign_handoff.rs` | Step 0d preflight gate |
| `server/src/pure_jsonrpc/handler/sovereign.rs` | `sovereign.kernel_health` handler |
| `server/src/pure_jsonrpc/handler/mod.rs` | Route registration |
| `cli/src/commands/kernel_health.rs` | New: CLI subcommand impl |
| `cli/src/commands/definitions.rs` | `KernelHealth` variant |
| `cli/src/commands/dispatch/mod.rs` | Dispatch wiring |
| `cli/src/commands/mod.rs` | Module registration |
| `cli/Cargo.toml` | `toadstool-cylinder` dep |
