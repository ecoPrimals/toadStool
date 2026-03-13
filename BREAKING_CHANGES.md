# Breaking Changes

Per-session log of API changes that may affect downstream springs.
Springs: check this file after updating your toadStool pin.

## Sessions S145–S152 (Mar 10–13, 2026)

No breaking changes. All additions are backward-compatible:
- New `compute.dispatch.*` and `compute.hardware.auto_init_all` JSON-RPC methods
- New `os_keyring` module in toadstool-common
- New `RemoteDispatcher` in cross_gate
- New `MockV4l2Device` / `MockVfioDevice` in testing

## S95–S96 (March 6, 2026)

### Breaking Changes

- **`management/resources` crate re-added to workspace** — evolved from placeholder to real `ResourceManager` with `toadstool-sysmon` (pure Rust). Springs that pinned a workspace without this member should update.
- **`SubstrateType` enum expanded** — 4→8 variants (`IntegratedGpu`, `Npu`, `Tpu`, `Fpga`, `Dsp`, `Quantum` added). Non-exhaustive match arms on `SubstrateType` need updating.
- **`GpuAdapterInfo` fields added** — `fingerprint: HardwareFingerprint` and `safe_allocation_limit: u64` fields added.

### New APIs (non-breaking)

- `HardwareFingerprint` struct — estimated TFLOPS, sovereign capability flag
- `SubstrateCapabilityKind` enum (12 variants) — fine-grained capability classification
- `GpuAdapterInfo::is_sovereign_capable()` / `is_allocation_safe()` / `is_nvk()` helper methods
- `SubstrateType::is_batch_oriented()` / `is_latency_oriented()` classification helpers

### Internal (non-breaking)

- 5 god files smart-split into domain modules (public APIs preserved via re-exports)
- `crates/api/` ByobApi extracted to container crate (api crate was not in workspace)
- `TOADSTOOL_DISCOVERY_BIND_ADDR` env var for configurable discovery bind address

## S94b (March 3, 2026)

### Breaking Changes

- **`management/resources` crate removed from workspace** — placeholder crate excluded; no production code existed.
- **`integration-tests` barracuda dependency now optional** — enable with `cargo test -p toadstool-integration-tests --features barracuda`. Zero imports were found; this was a dead dependency.

### Deprecation Migration Complete (non-breaking)

- All 7 production callers of `get_socket_path_for_service()` migrated to `get_socket_path_for_capability()` (D-SOV resolved).
- Hardcoded CLI ports `8080` / `9090` replaced with `ConfigUtils::get_toadstool_port()` and `ports::toadstool::METRICS`.

### New APIs (non-breaking)

- `NpuDispatch` trait — vendor-agnostic neuromorphic compute interface (`toadstool-core`)
- `AkidaNpuDispatch` — Akida adapter for `NpuDispatch`
- `NpuParameterController` trait — NPU-driven autonomous parameter tuning (`toadstool-core`)
- `GpuAdapterInfo` / `GpuDeviceType` — detailed GPU adapter info for barraCuda driver profiling (`toadstool-runtime-universal`)
- Multi-adapter GPU selection via `TOADSTOOL_GPU_ADAPTER` environment variable
- `NestGateClient::store_artifact()` / `retrieve_artifact()` now use real JSON-RPC with fallback

## S90-92 (March 3, 2026)

### Deprecations (non-breaking, migration recommended)

- `get_socket_path_for_service()` deprecated since 0.92.0 — use `get_socket_path_for_capability()`
- `get_primal_default_port()` deprecated since 0.92.0 — use capability-based discovery via infant_discovery
- `capability_typical_provider()` deprecated since 0.92.0 — use capability-based discovery via infant_discovery

### Breaking Changes

- **REST API removed** (S90): All `/api/v2/*` routes deleted. JSON-RPC 2.0 via `/jsonrpc` is the only API path.
- **Middleware removed** (S92): `toadstool_api::middleware` module deleted. No replacement needed — middleware was REST-era code with no production callers.
- **BearDog strings neutralized** (S92): Error messages in `crypto_lock/access_control/manager.rs` no longer reference "BearDog" by name. Now uses generic "crypto permission" / "security provider" language.

### New APIs (non-breaking)

- `get_socket_path_for_capability(capability: &str)` — sovereignty-compliant socket discovery
- `EcosystemDiscoverer::find_pattern_by_capability(capability: &str)` — capability-based service pattern lookup
- `verify_sha256(data, expected)` extracted as standalone fn in `wasm_ops`

## S88 (March 2, 2026)

### New APIs (non-breaking)

- `spectral::anderson_4d` and `spectral::wegner_block_4d` now re-exported
  from `spectral/mod.rs` (previously required `spectral::anderson::` path)
- `SeasonalGpuParams::new()` constructor added (no longer requires
  `bytemuck::zeroed()` + field-by-field assignment)
- `MultiHeadEsn::from_exported_weights()` constructor added
- `spectral::tridiag_eigenvectors()` added (Sturm + inverse iteration)
- `optimize::LbfgsGpu` batched GPU L-BFGS optimizer added
- New tolerances: `HYDRO_ET0`, `HYDRO_SOIL_MOISTURE`, `HYDRO_WATER_BALANCE`,
  `HYDRO_CROP_COEFFICIENT`, `PHYSICS_ANDERSON_EIGENVALUE`,
  `PHYSICS_LATTICE_ACTION`, `PHYSICS_LYAPUNOV`, `BIO_DIVERSITY_SHANNON`,
  `BIO_DIVERSITY_SIMPSON`, `BIO_PHYLOGENETIC`

### No breaking changes in S88.

## S87 (March 2, 2026)

### No breaking changes in S87.

- `TODO(afit)` comments reclassified to `NOTE(async-dyn)` (documentation only)
- FHE shader arithmetic fixes (internal, no API change)
- `gpu_helpers.rs` refactored into submodules (internal, re-exports preserved)

## S86 (March 2, 2026)

- `BatchedMultinomialGpu::sample` signature changed (groundSpring V67, wetSpring V92F)
- `DriftMonitor::is_drifting()` replaces `consecutive_drift` field (neuralSpring)
