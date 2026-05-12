# ToadStool S243–S244 Handoff — Vestigial Cleanup + Deep Debt Sweep

**Date**: May 12, 2026
**Sessions**: S243, S244
**From**: toadStool team
**Status**: 8,289 lib-only tests, zero clippy warnings, zero production debt

---

## S243 — Vestigial Cleanup: Legacy swap_device Removal, Capabilities Enhancement, Phase C Recon

### Changes

- **Legacy `swap_device()` removed**: `GlowPlugClient::swap_device()` (synchronous
  sysfs writes) had zero callers from JSON-RPC. `reacquire()` evolved to use async
  `swap_device_orchestrated()` (full 7-step lifecycle). Dead code removed:
  `EmberSwapResult` struct, `find_driver_unbind_path` helper
- **`compute.dispatch.capabilities` enhanced**: DRM GPU objects now include
  `render_node` (e.g. `/dev/dri/renderD128`) and `device_id` — prepares for
  Phase C where coralReef cuts over from direct `enumerate_render_nodes()` to IPC
- **`cuda` keyword removed**: Stale `"cuda"` removed from `toadstool-runtime-gpu`
  Cargo.toml keywords. Zero `cfg(feature = "cuda")` gates remain
- **`SwapExecutor` visibility confirmed**: `pub trait` with `pub use` from crate root
- **Phase C recon complete**: coral-driver tree mapped (100+ files: vfio/, amd/, nv/,
  drm.rs, hardware.rs, error.rs). No dependency on coral-reef compiler crate.
  gsp/ and intel/ retained by coralReef. nv/qmd/ contested (encoding→toadStool,
  values→coralReef)

## S244 — Deep Debt: println→tracing, Duration Constants, Test Coverage, Clippy Fixes

### Changes

- **Benchmark println!→tracing**: `comprehensive_benchmark.rs` production code
  migrated from `println!` to structured `tracing::info!`. Helper functions
  `format_time`/`truncate` relocated to test-only scope
- **Duration constant extraction** (11 files, 15+ constants):
  - `server/tarpc_server/executor.rs` — `CPU_USAGE_SAMPLE_WINDOW`
  - `distributed/cloud/federation/discovery.rs` — `PROBE_TIMEOUT_TEST`, `PROBE_TIMEOUT_PROD`
  - `distributed/coordination/discovery/core.rs` — `CPU_USAGE_SAMPLE_WINDOW`
  - `distributed/universal/adapter.rs` — `DEFAULT_REQUEST_TIMEOUT_SECS`
  - `integration/protocols/config.rs` — 6 constants
  - `integration/protocols/client/health.rs` — `HEALTH_PROBE_TIMEOUT_SECS`
  - `integration/protocols/transport.rs` — `BINARY_HANDSHAKE_TIMEOUT` (cfg-gated)
  - `integration/protocols/bear_dog/client.rs` — `AUDIT_FLUSH_INTERVAL_SECS`
  - `integration/storage/config.rs` — 3 constants
  - `integration/security/seed.rs` — `DEFAULT_SEED_FRESHNESS`
  - `integration/primals/manager.rs` — 3 constants
- **GlowPlugClient test coverage**: `reacquire_returns_bdf`,
  `swap_device_orchestrated_returns_boot_result`, `orchestrator_accessible`,
  `read_current_driver_nonexistent_device`
- **Clippy fixes**: `bool_to_int_with_if` → `usize::from()`,
  `unchecked_time_subtraction` → `.checked_sub().unwrap()`

---

## Quality Metrics

| Metric | Value |
|--------|-------|
| Lib-only tests | 8,289 |
| Workspace tests | 22,843+ |
| Clippy warnings | 0 |
| Production println/eprintln | 0 |
| Production TODO/FIXME | 0 |
| Production unreachable!() | 0 |
| Unsafe blocks | 46 (all SAFETY-documented) |
| Production files >800L | 0 |

---

## Next Steps

- **Phase C absorption**: Create `toadstool-cylinder` crate. Absorb coral-driver
  hardware modules (vfio/, amd/, nv/ hardware, drm.rs, hardware.rs, error.rs)
- **Coverage push**: Target 88% by end of Phase C (from ~83.6%)
- **Remaining Duration literals**: CLI crate has ~15 inline Duration values that
  can be centralized in a future sweep
