# Active Technical Debt Register

**Date**: February 18, 2026
**Philosophy**: Workarounds are short-term solutions that increase debt.
We aim to solve deep debt over iterations, evolving toward vendor-agnostic,
capability-based solutions.

---

## Active Workarounds

### W-001: Open-Source GPU Driver f64 Transcendental Workaround

**Status**: ACTIVE — First solution, scheduled for evolution
**Impact**: ~2x performance penalty for exp/log on affected drivers
**Files**:
- `crates/barracuda/src/device/wgpu_device.rs` — `needs_f64_exp_log_workaround()`
- `crates/barracuda/src/shaders/precision.rs` — `for_driver_auto()`, `inject_missing_math_f64()`

**Problem**: Open-source GPU compiler backends crash on f64 transcendentals:
- **NVK/NAK** (NVIDIA nouveau): `exp(f64)` crashes the NAK compiler
- **RADV/ACO** (AMD open-source): `fexp2` unimplemented for bit size 64

**Current Solution**: Text replacement `exp()` → `exp_f64()`, `log()` → `log_f64()`
with software implementations from `math_f64.wgsl`. Detection is driver-name based.

**Why This Is Debt**:
1. Text replacement is fragile (could match comments, variable names)
2. Driver detection is heuristic (name matching, not capability probing)
3. Software fallbacks are ~2x slower than native hardware
4. Applies blanket workaround rather than per-op capability check

**Evolution Path** (ordered by priority):
1. **Capability probing**: At startup, dispatch a tiny `exp(f64)` test shader.
   If it succeeds, skip the workaround. This makes detection vendor-agnostic.
2. **Upstream NAK fix**: Contribute `exp(f64)` lowering to Mesa NAK compiler.
   Track: https://gitlab.freedesktop.org/mesa/mesa
3. **Upstream ACO fix**: Contribute `fexp2(f64)` implementation to Mesa ACO.
   Track: https://gitlab.freedesktop.org/mesa/mesa
4. **Remove workaround**: When both compilers support f64 transcendentals
   natively, delete the replacement logic entirely.

**Validation**: Cross-GPU testing on 3090 (NVIDIA), 6950XT (AMD),
Titan V (NVK), RTX 4070 (proprietary).

---

### W-002: PPPM GPU Physics Validation

**Status**: ACTIVE — Pre-existing bug, never validated on GPU
**Impact**: 3 PPPM GPU tests fail with wrong physics values
**Files**: `crates/barracuda/src/ops/md/electrostatics/pppm_gpu.rs`

**Problem**: The PPPM (Particle-Particle Particle-Mesh) electrostatics solver
produces incorrect force directions and energy signs. Tests were previously
masked by f64 capability errors (wrong device pool).

**Why This Is Debt**: Complex multi-stage algorithm (charge spreading → FFT →
Green's function → force interpolation → short-range correction) with no
validated reference implementation. Each stage may have subtle bugs.

**Evolution Path**:
1. **CPU reference**: Implement CPU PPPM and validate against known benchmarks
2. **Stage-by-stage validation**: Test each pipeline stage independently
3. **Cross-check**: Compare with established MD codes (LAMMPS PPPM values)

---

## Tracked Debt (Not Workarounds)

### D-001: ~218 remaining ops test modules still create per-test GPU devices

**Impact**: GPU resource exhaustion under concurrent testing
**Progress**: `device/test_pool.rs` foundation exists; 9 modules migrated (upsample, unfold, tile, tensor_split, take, squeeze, split, tanh, swish_wgsl)
**Evolution**: Continue migrating remaining ops modules to `test_pool::get_test_device_if_gpu_available()`

### D-002: Hardcoded timeouts in distributed/capability crates

**Impact**: Non-configurable behavior
**Evolution**: Move to capability-based or config-driven timeouts

### D-003: `batched_eigh_gpu.rs` split into 6-module dir (was 2085→1772→split)

**Status**: Module-split complete (standard.rs, single_dispatch.rs, pipelines.rs, sweep.rs, params.rs, mod.rs)
**Remaining**: deflation, shift-invert, blocked, banded variants still to implement when needed

### D-004: cudarc version outdated in docs

**Impact**: DEEP_DEBT_STATUS.md references 0.11 → 0.19 as future; already done
**Evolution**: Audit and update stale documentation

---

## Resolved Debt (Recent)

| ID | Description | Resolution Date |
|----|-------------|----------------|
| R-001 | All-or-nothing shader injection (`_safe` methods) | Feb 18, 2026 |
| R-002 | Hardcoded `div_ceil(256)` in 19 files | Feb 18, 2026 |
| R-003 | String-based hardware detection in substrate.rs | Feb 18, 2026 |
| R-004 | `futures::executor::block_on` in async test context | Feb 18, 2026 |
| R-005 | wgpu 0.19 pinned dependency | Feb 17, 2026 |
| R-006 | NVK-only exp workaround (now covers NVK + RADV) | Feb 18, 2026 |
| R-007 | Duplicate `ExternalTarget` enums in crypto_lock vs security_provider | Feb 18, 2026 |
| R-008 | `reqwest` C-FFI dep in toadstool-client — migrated to Unix JSON-RPC | Feb 18, 2026 |
| R-009 | `crates/client` excluded from workspace — re-included | Feb 18, 2026 |
| R-010 | 9 files over 1000 LOC — smart-refactored: cg_gpu, multi_gpu, production_hardening, graph_types, handlers, graph_types, handlers | Feb 18, 2026 |
| R-011 | WebSocket (tungstenite/ring C-FFI) removed from entire codebase — pure Rust | Feb 18, 2026 |
| R-012 | 10 more files over 1000 LOC refactored: batched_eigh, wgpu_device, tensor_context, workload_migration, deployment_layer, songbird/types, analyzer, three_springs tests, hotspring tests, capabilities/tests | Feb 18, 2026 |
| R-013 | D-002 hardcoded timeouts replaced with toadstool_common constants | Feb 18, 2026 |
| R-014 | D-004 stale docs updated (cudarc 0.11→0.19, WebSocket refs removed) | Feb 18, 2026 |
| R-015 | sparsity.rs (1242L), fd_gradient_f64.rs (1175L), manual_jsonrpc.rs (1100L) split | Feb 18, 2026 |
| R-016 | D-001 partial: test_pool foundation + 9 ops modules migrated to shared GPU device | Feb 18, 2026 |

---

*Debt is tracked, not ignored. Each workaround has an evolution path.*
*The goal is zero workarounds — vendor-agnostic, capability-based code.*
