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
   Foundation: `GpuDriverProfile::from_device()` in `device/capabilities.rs` already
   detects driver/compiler via runtime adapter info — capability probing is the next step.
2. **Upstream NAK fix**: Contribute `exp(f64)` lowering to Mesa NAK compiler.
   Track: https://gitlab.freedesktop.org/mesa/mesa — see W-003 for full NAK roadmap.
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

### W-003: NAK Compiler 149x Performance Gap (Sovereign FP64 Compute)

**Status**: ACTIVE — Contribution roadmap defined, first solution absorbed
**Impact**: NVK/NAK Jacobi eigensolve ~9x slower than NVIDIA proprietary after warp-packing
**Files**:
- `crates/barracuda/src/shaders/linalg/batched_eigh_single_dispatch_f64.wgsl` — warp-packed (done)
- `crates/barracuda/src/device/capabilities.rs` — `GpuDriverProfile`, `EigensolveStrategy` (done)
- `crates/barracuda/src/bin/bench_wgsize_nvk.rs` — diagnostic binary (done)

**Problem**: hotSpring analysis (Feb 18, 2026) found a 149x compiler efficiency gap
between NAK (Mesa open-source NVIDIA compiler, Rust) and proprietary PTXAS for
loop-heavy f64 Jacobi kernels. Root cause is five specific NAK deficiencies:

| # | Deficiency | Gap factor | NAK status |
|---|-----------|------------|------------|
| 1 | No SM70 instruction scheduling | ~3-4x | Only SM32 (Kepler) has real scheduling |
| 2 | No dual-issue exploitation | ~2x | Not implemented for any arch |
| 3 | Limited loop unrolling | ~1.5-2x | MR 26626 (Dec 2023), may miss nested loops |
| 4 | Missing f64 FMA selection | ~1.3-1.5x | Not confirmed, needs IR dump |
| 5 | Generic shared-mem scheduling | ~1.5-2x | No bank-conflict awareness |

**First Solution Already Absorbed** (R-019):
- Warp-packed eigensolve (`@workgroup_size(32,1,1)`) — 2.2x NVK speedup, neutral on proprietary
- `GpuDriverProfile::optimal_eigensolve_strategy()` — data-driven strategy selection
- `bench_wgsize_nvk.rs` — permanent diagnostic binary

**Evolution Path** (NAK contribution — all Rust, AGPL-aligned):
1. **Phase 1**: SM70 latency tables in `nak/src/calc_instr_deps.rs` — use envytools ISA data
2. **Phase 2**: f64 FMA pattern matching in `nak/src/from_nir.rs` — fold `mul+add` → `DFMA`
3. **Phase 3**: Loop unrolling for bounded nested loops — targets our Jacobi pattern
4. **Phase 4**: Dual-issue exploitation for Volta (SM70) — paired execution units

**Why This Matters**: NAK is written in Rust, same language as BarraCUDA. Every improvement
benefits all NVK users — this is the open-source multiplier. AMD RDNA3 with RADV/ACO is
a second target once NVK baseline is established.

**Tracking**: https://gitlab.freedesktop.org/mesa/mesa/-/tree/main/src/nouveau/compiler

---

## Tracked Debt (Not Workarounds)

### D-001: ~218 remaining ops test modules still create per-test GPU devices

**Impact**: GPU resource exhaustion under concurrent testing
**Progress**: `device/test_pool.rs` foundation exists; 9 modules migrated (upsample, unfold, tile, tensor_split, take, squeeze, split, tanh, swish_wgsl)
**Evolution**: Continue migrating remaining ops modules to `test_pool::get_test_device_if_gpu_available()`

### D-002: Hardcoded timeouts in distributed/capability crates

**Impact**: Non-configurable behavior
**Evolution**: Move to capability-based or config-driven timeouts

### D-003: RESOLVED — All files now under 1000 lines

**Status**: RESOLVED Feb 18, 2026
All non-showcase Rust files are under 1000 lines.
Deflation, shift-invert, blocked, banded eigh variants are future additions (new files when implemented).

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
| R-017 | cg_gpu (1519L), pppm_gpu (1337L), precision (1270L), primal_sockets (1154L), service_discovery (1135L), cuda_impl (1093L), ipc_helpers (1091L), unibin (1059L), composition_constraints (1051L), resource_optimizer (1036L), biomeos/auth (1033L) all split | Feb 18, 2026 |
| R-018 | D-003 resolved: ALL non-showcase files now under 1000 lines | Feb 18, 2026 |
| R-019 | Warp-packed eigensolve (`@workgroup_size(32,1,1)`, 2.2x NVK speedup), `GpuDriverProfile`, `EigensolveStrategy`, `bench_wgsize_nvk.rs` diagnostic binary — hotSpring Phase 1 handoff absorbed | Feb 18, 2026 |

---

*Debt is tracked, not ignored. Each workaround has an evolution path.*
*The goal is zero workarounds — vendor-agnostic, capability-based code.*
