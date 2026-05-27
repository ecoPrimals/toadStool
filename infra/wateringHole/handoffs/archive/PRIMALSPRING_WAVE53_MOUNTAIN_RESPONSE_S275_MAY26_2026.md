# toadStool — Wave 53 "Primals on the Mountain" Response

**Date**: May 26, 2026
**Session**: S275
**From**: toadStool team
**To**: primalSpring (downstream audit)
**Audit ref**: Wave 53 — Primals on the Mountain (May 26, 2026)

---

## toadStool Items — Status

### 1. Coverage push 83.6% → 90% — INCREMENTAL, on track

Coverage infrastructure is solid:
- `cargo llvm-cov` via `scripts/run-coverage.sh` (primary)
- `cargo tarpaulin` via `tarpaulin.toml` (secondary)
- 133 dedicated `*coverage*` test files across workspace

Gap analysis: remaining ~6.4% concentrated in hardware-dependent paths
(display/V4L2 ~3,800L, neuromorphic/VFIO ~2,000L, runtime backends ~1,000L).
These require real hardware for meaningful coverage. Soft-testable modules
are well covered (~90%+ in IPC/JSON-RPC/core logic).

No blocking debt. Continuing incremental push.

### 2. Expand sovereign VFIO dispatch — ACTIVE

Upstream commits landed this session:
- `reagent.rs` (841L) — reagent capture/distill pipeline
- `pri_ring_anchor.rs` (187L) — PRI ring anchor for Volta+
- `sovereign_handoff.rs` expansion (+700L) — IMEM firmware capture, catalyst teardown
- `profile-catalyst-teardown.sh` script
- `sovereign.rs` handler — reagent JSON-RPC methods

All absorbed with 0 clippy warnings (13 upstream clippy issues fixed during
absorption: collapsible_if, derivable_impls, equatable_if_let,
unnecessary_if_let, map_unwrap_or, single_match_else, too_many_lines,
dead_code).

### 3. Songbird `ipc.register` self-registration — ALIGNED

**Already implemented** as outbound client registration at startup via
`register_with_discovery()` in `connection.rs`. Fires at both unibin and
CLI daemon startup.

**Fixed this session**: Capability list was stale — registered
`["compute.dispatch", "compute.capabilities"]` but should match Node Atomic
set. Now registers via `DISCOVERY_CAPABILITIES` constant:

```
["compute", "workload", "orchestration", "gpu", "wasm",
 "container", "hardware_transport", "shader_dispatch", "hardware_learning"]
```

Aligned with `primal.announce` handler capabilities. Tests updated.
Stale DEBT.md note about `capability.register` corrected (method is
`ipc.register`, not `capability.register`).

### 4. Cold-start latency >8s — ALREADY RESOLVED (S275)

Fixed in S275 (Wave 49 ecosystem tightening):
- Deferred wgpu GPU enumeration (1–5s savings)
- Pre-bound JSON-RPC socket (health probes connect during init)
- Socket listening within ~1s of startup

---

## Upstream Clippy Absorption

13 clippy warnings from upstream VFIO/reagent commits absorbed:

| Crate | Issues | Fix |
|-------|--------|-----|
| cylinder (boot_follower) | 1 collapsible_if | Collapsed nested if-let |
| cylinder (sovereign_handoff) | 2: equatable_if_let, too_many_lines | `.is_ok()`, `#[allow]` with reason |
| cylinder (reagent) | 6: derivable_impls, unnecessary_if_let, 3x collapsible_if, map_as_ref | `#[derive(Default)]`, flatten(), collapsed |
| glowplug (swap) | 1 single_match_else | `if let ... else` |
| runtime-gpu (firmware) | 1 dead_code | `#[expect]` with reason |
| server (sovereign) | 1 map_unwrap_or | `map_or_else` |

## Metrics

| Metric | Value |
|--------|-------|
| Lib tests | 9,158 |
| Workspace tests | 23,000+ |
| JSON-RPC methods | 88+ (reagent methods added) |
| Clippy warnings | 0 |

---

All toadStool Wave 53 items addressed. Zero blocking debt.
