# ToadStool S278b — Primal Mountain Debt: Clippy Absorption + Archive Hygiene

**Date:** 2026-05-27
**Session:** S278b (post-S278 upstream absorption)
**Upstream:** primalSpring Primal Mountain Debt audit + S278 cylinder refactoring

---

## Context

primalSpring's Primal Mountain Debt audit flagged toadStool with one item:
"36 unmirrored local wateringHole handoffs need archive hygiene" (LOW).

Additionally, the S278 upstream pull (cylinder module extraction: 7 oversized
files → module directories, 4 C→Rust bin ports, rm_abi.rs, registers/)
introduced 50 clippy errors that needed absorption.

## Work Done

### 1. Absorbed 50 upstream clippy errors (from S278)

| Error | Count | Fix |
|-------|-------|-----|
| `missing_docs` on rm_abi.rs constants/structs | 249→0 | `#![allow(missing_docs)]` — ABI definitions file |
| `pub _pad` unused-but-public fields | 9 | `pub _pad` → `_pad` (private padding) |
| `needless_borrow` (&profile where profile is &ref) | 10 | Remove outer `&` in open_anchor.rs, open_vfio.rs |
| `needless_borrow` (kernel_health.rs) | 5 | Remove outer `&` on krel borrows |
| Unused imports (guarded_sysfs, sovereign_stages, pfifo) | 8 | Remove unused imports |
| `unnecessary_cast` (i32 as i32) | 3 | Remove redundant casts |
| `collapsible_if` + `pattern_matching_equality` | 5 | Let-chains |
| `unwrap_or_default` | 1 | `match Ok/Err → .unwrap_or_default()` |
| `unfulfilled_lint_expectations` (registers.rs) | 2 | Remove stale `#[expect(dead_code)]` |
| `unused_variable` (tier_t) | 1 | Prefix with `_` |
| `dead_code` field (pri_faulted) | 1 | `#[expect(dead_code)]` |
| `deprecated` (StubGspBridge) | 1 | → NoopGspBridge |
| `unnecessary_map_or` | 1 | → `.is_some_and()` |
| Missing doc (pfifo create_on_runlist) | 1 | Added doc comment |

### 2. Archive hygiene (Primal Mountain Debt item)

The audit's "36 unmirrored" claim was **stale** — all handoffs were already
mirrored to central (completed in S275 Wave 53). However, 6 superseded
handoffs (S267-S272) were still in the active directory:

Moved to `archive/`:
- `PRIMALSPRING_WAVE38_RESPONSE_S269_MAY22_2026.md`
- `PRIMALSPRING_WAVE43_RESPONSE_S270_MAY23_2026.md`
- `PRIMALSPRING_WAVE44_RESPONSE_S271_MAY23_2026.md`
- `PRIMALSPRING_WAVE47_RESPONSE_S272_MAY24_2026.md`
- `TOADSTOOL_S267_SOVEREIGN_DRIVER_ROTATION_MAY20_2026.md`
- `TOADSTOOL_S268_KERNEL_HEALTH_PREFLIGHT_MAY21_2026.md`

Result: 7 active handoffs (S273+), 34 archived. All mirrored to central.

## Primal Mountain Debt — ToadStool Status

| Item | Status |
|------|--------|
| 36 unmirrored handoffs | **RESOLVED** — all mirrored (S275), stale handoffs archived (S278b) |
| R11 PID file alongside socket | **DEPRIORITIZED** — connect-probe provides liveness |

**Verdict: toadStool CLEAN/zero mountain debt.**

## Test Verification

- 9,156+ lib tests, 0 failures, 0 clippy warnings
- Full upstream S278 cylinder refactoring absorbed cleanly
