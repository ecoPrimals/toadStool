# D-S18-002: cubecl Transitive dirs-sys Dependency (ecoBin Purity)

**Status**: Documented • **Priority**: Low • **ecoBin**: Violates pure-Rust requirement

---

## Summary

The `dirs-sys` crate (C FFI for XDG/home dir discovery) is pulled transitively via the Burn ML stack. It violates ecoBin purity, which requires zero C dependencies in the core binary path.

## Dependency Chain

```
dirs-sys v0.4.1
└── dirs v5.0.1
    └── cubecl-runtime v0.4.0
        └── cubecl v0.4.0
            ├── burn-jit
            ├── burn-tensor
            └── burn-wgpu
                └── burn-inference (crates/ml/burn-inference)
                    └── showcase/cross-platform
```

**Direct workspace dependents of cubecl**: None (transitive only).

**Path**: `burn-inference` → `burn-wgpu` 0.16 → `burn-cubecl` → `cubecl` 0.4.0 → `cubecl-runtime` 0.4.0 → `dirs` → `dirs-sys`.

## Crates Pulling cubecl

| Crate | Dependency | Notes |
|-------|------------|-------|
| `crates/ml/burn-inference` | burn-wgpu 0.16 | ML inference via Burn |
| `showcase/cross-platform` | burn-inference | Cross-platform compute showcase |

## cubecl Version Analysis

### cubecl 0.4.0 (current via Burn 0.16)

- `dirs` is a **direct dependency** of `cubecl-runtime` (not optional).
- No feature flag to disable it.

### cubecl 0.9.0 / 0.10.0-pre.1 (latest)

- **cubecl-runtime**: `dirs` is **optional** (`optional = true`).
- `std` feature enables: `cubecl-common/std`, `toml`, **`dirs`**, `thiserror/std`.
- **cubecl-common**: `dirs` is **optional**; `cache` feature enables it (for autotune cache storage).

**Conclusion**: Newer cubecl has `dirs` behind features, but Burn enables `std` by default. Burn 0.20 still pulls `dirs-sys` because burn-wgpu default features include `std` → `cubecl/std`.

## Resolution Options

| Option | Feasibility | Notes |
|--------|-------------|-------|
| **a) Update Burn** 0.16 → 0.20 | No fix | Burn 0.20 still pulls dirs-sys (cubecl 0.9.0 std enabled). |
| **b) Disable cubecl std via Burn** | Not exposed | Burn has no feature to disable cubecl std; would likely break runtime. |
| **c) Upstream PR to cubecl** | **Recommended** | Replace `dirs` with `etcetera` (pure Rust) for cache paths. |
| **d) Upstream PR to Burn** | Possible | Add `no-cache` or `nostd` feature to disable cubecl cache/std. |
| **e) Cargo patch + fork** | High maintenance | Fork cubecl, replace dirs, patch in workspace Cargo.toml. |
| **f) Feature-gate burn-inference** | Partial | Core binary could exclude ML, but showcases and tests would still pull it. |

## Recommended Resolution

**Upstream PR to cubecl**: Replace `dirs` with `etcetera` (pure Rust, XDG-compliant).

- ToadStool already uses `etcetera` (workspace dep).
- cubecl uses `dirs` for autotune cache directory (e.g. `dirs::cache_dir()`).
- `etcetera` provides equivalent via `BaseDirs::new()` → `cache_dir()`.

## Related Files

- `DEBT.md` — Debt register entry
- `crates/integration-tests/tests/pure_rust_validation_tests.rs` — `test_dirs_sys_eliminated` (ignored)
- `deny.toml` — Crate bans; dirs-sys not banned (would block burn-inference)

## Verification Commands

```bash
# Inspect dirs-sys in tree
cargo tree -i dirs-sys

# Inspect cubecl dependency
cargo tree -p cubecl
```

---

*Analysis: 2026-02-22. Last updated: 2026-02-23. Verified: dirs-sys still present via burn-inference 0.16 → burn-wgpu → cubecl 0.4.0 → dirs → dirs-sys. cubecl has not yet switched to etcetera; resolution options unchanged.*
