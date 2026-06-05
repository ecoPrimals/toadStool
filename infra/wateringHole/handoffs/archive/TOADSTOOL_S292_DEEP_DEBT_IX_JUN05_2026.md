# toadStool S292 — Deep Debt IX: Feature Gates + Module Splits + Naming + SAFETY + Deprecated Cleanup

**Date**: June 5, 2026
**Gate**: strandGate (biomeGate hardware OFFLINE)
**Status**: Deep debt pass complete. Software-only.

## Changes

### Feature gate: serialport in runtime/edge

- `serialport` made `optional = true` behind `serial-transport` feature
- Not in default features — edge crate compiles without serial FFI
- Stub modules return clear errors when feature disabled
- Pattern matches `modbus-transport` in specialty and `docker` in container

### Module split: dispatch/device.rs (781L → 4 modules)

- `device/mod.rs` (403L) — shared types, device pool, factory, local dispatch
- `device/vfio.rs` (207L) — VFIO open/roundtrip handlers
- `device/gr_init.rs` (96L) — GR context initialization
- `device/lifecycle.rs` (90L) — DMA prepare/cleanup, handle lifecycle
- All public interfaces unchanged

### Hardcoded names → constants

- Added `PRIMAL_BINARY_NAME` constant for executable name references
- `installer/paths.rs` — Linux/macOS/Windows paths use `PRIMAL_NAME`/`PRIMAL_DISPLAY_NAME`
- `installer/core.rs` — install script uses `PRIMAL_BINARY_NAME`
- `launcher.rs` — default binary path uses `PRIMAL_BINARY_NAME`
- `loading.rs` — test env uses `PRIMAL_NAME`

### Deprecated exports removed

- `server/lib.rs` — removed deprecated `StandaloneExecutor`, `ToadStoolTarpcServer`, `WorkloadExecutor`, `WorkloadExecutorDispatch`, `TestExecutor` re-exports
- `distributed/lib.rs` — removed deprecated coordination type re-exports (module already feature-gated S290)

### SAFETY documentation

- `v4l2/ioctl.rs` — all 3 unsafe ioctl blocks now fully documented
- `plugin_system/abi.rs` — module-level FFI contract + per-symbol safety docs

## Metrics

- **8,895** lib tests passed (default), 0 failed
- Full workspace clippy `-D warnings` clean
- 24 files changed, 381 insertions, 972 deletions (net -591 lines)

## Remaining Debt

1. **StubRuntimeEngine default** — P0 correctness; needs careful API design
2. **CallerContext full threading** — P0 security; ionic token → identity/envelope
3. **LEGACY env fallbacks** — 23 reads in 5 files; staged removal pending
4. **Files 750L+** — `mmu_oracle/capture.rs` (795L), `bar_cartography.rs` (782L) in cylinder
5. **Feature tightening** — `tarpc` still always-on in integration/protocols + display
