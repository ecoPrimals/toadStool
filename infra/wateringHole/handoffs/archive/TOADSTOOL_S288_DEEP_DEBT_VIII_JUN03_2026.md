# toadStool S288 — Deep Debt VIII: Panic Elimination + Naming + Feature Gates + Safety Docs

**Date**: June 3, 2026
**Gate**: strandGate (biomeGate hardware OFFLINE)
**Status**: Deep debt pass complete. Software-only.

## Comprehensive Audit Results

| Category | Before S288 | After S288 |
|----------|-------------|------------|
| Production files >800L | 0 (already done S284) | 0 |
| P0 production panic paths | 4 (akida MMIO) | 0 |
| P1 production panic paths | 3 (cpu_resource, rm_trigger) | 0 |
| Primal-name type aliases | 3 (BearDog*) | 0 |
| Always-on FFI deps | 1 (modbus) | 0 |
| Missing // SAFETY: on unsafe | 7 (Ioctl boilerplate) | 0 |

## Changes

### P0: Akida MMIO (NPU panic paths)
- Deprecated `read32`/`write32`/`read64`/`write64` panicking wrappers
- Migrated VFIO backend callers to `try_read32`/`try_write32` with `?`
- `write_iova_regs` returns `Result<()>`, `is_ready` returns `false` on MMIO failure

### P1: CPU resource + rm_trigger
- Degraded Rayon pool: cascading fallback (current_thread → 1-thread → 0-thread)
- rm_trigger: `ne_bytes<N>()` helper, `run_card_info` returns `Result`

### Naming debt
- Removed `BearDogIntegration`, `BearDogPermission`, `BearDogIntegrationConfig`
- Callers migrated to `SecurityServiceIntegration`, `SecurityPermission`, `SecurityServiceIntegrationConfig`

### Feature gates
- `modbus` in `runtime/specialty`: now optional behind `modbus-transport`
- Stub module returns clear error when feature disabled

### Safety documentation
- Added `// SAFETY:` to 7 `output_from_ptr` boilerplate impls

## Remaining Debt (for future sessions)

1. **`distributed::coordination` module** — deprecated for external use, still compiled internally
2. **`networking` feature off by default** — `fallback_response()` in ecosystem communication
3. **`LEGACY_*` env fallbacks** — ~30 deprecated constants still read in identity chains
4. **`serialport`** always-on in `runtime/edge` — no practical pure-Rust replacement for USB-UART
5. **`bollard`** default-on in `runtime/container` — could be feature-gated
6. **Sovereign dispatch paths** don't thread `CallerContext` (trust/gate_id)
7. **`DispatchTelemetryRecord`** not yet emitted from submit paths

## Metrics

- Full workspace clippy `-D warnings` clean
- All workspace tests pass (exit 0)
- Zero production panic paths (P0 + P1)
