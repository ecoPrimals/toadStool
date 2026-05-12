# ToadStool S238 — Deep Debt: Magic Numbers, println→tracing, deny.toml, JH-2 Audit

**Date**: May 11, 2026
**Session**: S238
**From**: toadStool
**To**: primalSpring, ecosystem

---

## Summary

Comprehensive deep debt sweep targeting duplicated magic numbers, production
`println!` usage, stale `deny.toml` documentation, and JH-2 envelope
enforcement audit.

## Changes

### Magic Number Consolidation (20+ literals extracted)

**New module: `crates/distributed/src/common/defaults.rs`**

Centralised 11 named constants replacing bare literals that were duplicated
across 4+ files: `DISCOVERY_TIMEOUT_MS` (5000), `HEALTH_CHECK_INTERVAL_SECS`
(30), `HEALTH_CHECK_INTERVAL_MS` (5000), `STARTUP_TIMEOUT_MS` (30000),
`FAILOVER_THRESHOLD` (3), `MAX_RETRIES` (3), `CIRCUIT_BREAKER_THRESHOLD` (5),
`MAX_HOSTING_DEPTH` (3), `SHARING_RATIO` (0.8), `PRIORITY_BOOST` (1.2).

**Files updated**:
- `security/mod.rs` — `discovery_timeout_ms` → shared constant
- `crypto_integration/mod.rs` — `discovery_timeout_ms` → shared constant
- `coordination_integration/mod.rs` — `discovery_timeout_ms` + `health_check_interval_secs`
- `core/coordinator.rs` — replaced 11-line inline config with `CoordinationConfig::default()`
- `universal/scheduler.rs` — 7 literals in 4 Default impls
- `runtime/container/src/types.rs` — port ranges, resource limits, image cache (9 literals)
- `runtime/edge/src/lib.rs` — 3 config defaults
- `types/resources/host_config.rs` — port range, startup/health timeouts, resource limits (7 literals)

### println → tracing

`akida-models/src/zoo.rs` `print_status()` migrated from `println!` to
`tracing::info!` with structured fields (`cache`, `available`, `total`, `model`, `status`).

### deny.toml Corrections

Fixed 3 stale comments:
1. Ring license clarify — was "absent from Cargo.lock" → now "may appear as conditional transitive"
2. Ring ban comment — was "not in resolved workspace graph" → now "conditional via quinn-proto/rustls-webpki"
3. OpenSSL ban — removed misleading "ring" from alternative suggestion
4. skip-tree comment — updated date and description

### JH-2 Audit Result: FULLY RESOLVED

Confirmed all three envelope dimensions are enforced:
- `mem_mb` — binary size checked against envelope limit
- `cpu_cores` — workgroup total checked against core × 1024 thread cap
- `max_timeout_ms` — requested timeout checked against envelope limit

All dispatch paths wired:
- `compute.dispatch.submit` → `enforce_envelope` (submit.rs:321)
- `shader.dispatch` → `enforce_envelope` (shader_dispatch.rs:102)
- `compute.dispatch.pipeline.submit` → forwards `CallerContext` to `_with_context` variants (pipeline.rs:316-319)

**JH-2 is FULLY RESOLVED. No remaining enforcement gaps.**

## Quality Gates

- `cargo clippy --workspace -- -D warnings`: 0 warnings
- `cargo test --workspace --lib`: all pass (0 failures)
- No new unsafe code
- No new dependencies
