# toadStool S289 — Telemetry Wire Contract + Adversarial Trust Tests + Emission + Bollard Feature Gate

**Date**: June 4, 2026
**Gate**: strandGate (biomeGate hardware OFFLINE)
**Status**: All cascade targets complete. Software-only.

## Changes

### Telemetry Wire Contract v1.1

- `dispatch.telemetry.schema` JSON-RPC response evolved to versioned wire contract
- Added `TELEMETRY_SCHEMA_VERSION` constant ("1.1") for consumer validation
- Schema now includes: `contract`, `version`, `previous_versions`, `backward_compatible`
- Encoding rules documented in-schema: `string_fields` (FNV-1a → [0,1)), `boolean_fields` (binary), `nullable_fields` (zero default), `numeric_fields` (raw cast)
- Consumer list: `barraCuda:ml.mlp_train`, `biomeOS:L5.perceptron`
- Normalization guidance: `min_max_per_dimension`

### Adversarial Trust Test Coverage

+8 new tests in `dispatch::trust::tests` (14 total):
- `verify_trust_forged_btsp_no_gate_id` — BtspVerified with null gate_id
- `verify_trust_forged_mutual_auth_no_gate_id` — MutuallyAuthenticated with null gate_id
- `verify_trust_gate_id_mismatch_with_requested` — caller vs target gate mismatch
- `verify_trust_anonymous_with_gate_id` — Anonymous trust with rogue gate_id
- `verify_trust_malformed_params_non_string_gate_id` — numeric gate_id in params
- `verify_trust_empty_params_object` — empty params object
- `verify_trust_extra_unknown_params_ignored` — junk fields in params
- `all_trust_levels_serialize_to_snake_case` — serialization roundtrip

### Telemetry Emission from Dispatch Paths

- `emit_telemetry_record()` — structured `tracing::info!` on `dispatch.telemetry` target
- `DispatchTelemetryEmit` + `emit_dispatch_completion_telemetry()` — shared builder
- `compute.dispatch.submit` — telemetry on local cylinder success, coral IPC success/failure
- `shader.dispatch` — telemetry on local cylinder, wgpu, coral success/failure
- `trust_level_from_caller()` — serde-based trust level serialization
- Removed `#[allow(dead_code)]` from `DispatchTelemetryRecord` struct/impl

### Bollard Feature Gate

- `runtime/container`: `default = ["docker"]` → `default = []`
- Server and CLI crate dependencies explicitly enable `docker` feature
- Builds clean with and without `docker` feature

## Metrics

- **9,204** lib tests passed, 0 failed
- Full workspace clippy `-D warnings` clean
- All workspace tests pass (exit 0)
- 10 files changed, 394 insertions, 13 deletions

## Remaining Debt (for future sessions)

1. **`distributed::coordination` module** — deprecated for external use, still compiled internally
2. **`networking` feature off by default** — `fallback_response()` in ecosystem communication
3. **`LEGACY_*` env fallbacks** — ~30 deprecated constants still read in identity chains
4. **`serialport`** always-on in `runtime/edge` — no practical pure-Rust replacement for USB-UART
5. **Sovereign dispatch paths** don't thread `CallerContext` (trust/gate_id) through submit/shader/pipeline/fan_out
6. **`DispatchTelemetryRecord`** emitted but not yet persisted to disk (tracing-only)
7. **Test file proliferation** — overlapping `*_v2.rs`, `*_coverage_tests.rs` suites could consolidate
