# ToadStool S249 — Deep Debt: Duration Constants + Deprecated Cleanup

**Date**: May 12, 2026
**Session**: S249
**Phase**: Deep debt closure (parallel to Phase C)

---

## Summary

Full-spectrum audit and cleanup session. Extracted ~55 hardcoded `Duration`
literals into named constants across 14 production files. Removed 3 vestigial
`#[allow(deprecated)]` attributes from CLI template modules. Comprehensive
audit confirmed all prior quality gates remain green.

## Duration Constants Extracted

| File | Constants | Domain |
|------|-----------|--------|
| `network_config/configurator/core/defaults.rs` | 35 | Proxy/mTLS/discovery/pooling/retry/DNS/audit/canary/health/circuit-breaker |
| `monitoring/alerting.rs` | 4 | Alert threshold/cooldown durations |
| `daemon/config.rs` | 4 | Workload timeout, monitor/heartbeat/health intervals |
| `ecosystem/discovery.rs` | 1 | Discovery scan timeout |
| `ecosystem/capabilities/registry.rs` | 1 | Provider TTL |
| `ecosystem/adapters/universal.rs` | 1 | Request timeout |
| `executor/lifecycle_ops/start.rs` | 1 | Default workload timeout |
| `executor/display_ops.rs` | 1 | Log tail poll interval |
| `zero_config/service_discovery.rs` | 1 | Discovery timeout |
| `universal/operations/federation.rs` | 1 | Peer heartbeat interval |
| `universal/operations/migration.rs` | 1 | Migration estimate |
| `monitoring/collectors.rs` | 1 | CPU sample window |
| `monitoring/dashboard.rs` | 1 | CPU sample window |
| `core/ember/vendor_lifecycle/amd.rs` | 1 | hwmon poll interval |
| `core/nvpmu/power_manager.rs` | 2 | PMC settle deadline + poll interval |

## Deprecated Cleanup

Three CLI specialized template modules had `#[allow(deprecated)]` that was
no longer triggered (deprecated usage already migrated in prior sessions):
- `templates/specialized_templates/infrastructure_templates.rs`
- `templates/specialized_templates/custom_templates.rs`
- `templates/specialized_templates/ml_science_templates.rs`

Removed all three suppression attributes.

## Full Audit Results

| Dimension | Status |
|-----------|--------|
| Files >800L (production) | None |
| Mocks in production | All gated `#[cfg(any(test, feature = "test-mocks"))]` |
| `todo!()`/`unimplemented!()` in prod | None |
| `panic!()` in prod | None |
| `println!`/`eprintln!` in library | None |
| `cfg(feature = "cuda")` | Fully removed |
| Unsafe blocks | All legitimate hw FFI, all SAFETY-documented |
| External deps | All justified (wgpu/libloading/rustix for hw primal) |

## Quality Gates

- **Clippy**: Zero warnings (`-D warnings`), full workspace
- **Tests**: 8,704 lib-only passing
- **Cylinder tests**: 415 (unchanged — no cylinder changes this session)

## Root Documentation Updates

- `README.md`: Footer updated to S249, 8,704 tests, Phase C progress
- `DOCUMENTATION.md`: Condensed current state to S249, historical detail moved to CHANGELOG
- `CONTEXT.md`: Updated Phase C and deep debt status
- `CHANGELOG.md`, `DEBT.md`, `NEXT_STEPS.md`: S249 entries added

## Next Steps (S250+)

1. **VFIO channel modules** — devinit, glowplug, HBM2 training, diagnostics
2. **sovereign init/stages** — pipeline orchestration
3. **Phase D prep** — Wire `compute.dispatch.submit` through absorbed driver layer
4. **Coverage push** — MockVfioDevice/MockPciSysfs for hardware-gated test paths
5. **pcie.rs** — Create local `GpuTarget` type
