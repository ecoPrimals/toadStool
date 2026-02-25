# ToadStool/BarraCuda -- Next Steps

**Updated**: February 25, 2026 -- Session 60
**Status**: Production-grade | 680+ WGSL f64 shaders (12 DF64) | 2,435 barracuda tests | All quality gates green

---

## Completed This Session

### DF64 FMA + Transcendentals ✅

- `df64_core.wgsl`: FMA-optimized `two_prod` (17 ops → 2), `df64_mul` cross-terms use FMA
- New `df64_transcendentals.wgsl`: sqrt, exp, log, sin, cos, pow, tanh at FP32 core speed
- 4 force shaders evolved to all-DF64 (Born-Mayer, Morse, Yukawa, Lennard-Jones)
- Patcher hardened against ldexp/exp_df64 substring collisions
- P0 polyfill coverage verified (28 functions); AMD RADV tested (233+18 tests, 0 failures)

### Deep Debt Fixes ✅

- Crank-Nicolson variable shadowing bug fixed
- Cholesky SPD validation added
- Cross-attention `q_seq_len`/`kv_seq_len` evolution (6 Rust + 6 WGSL files)
- Multi-GPU adapter selection (deterministic pinning via `BARRACUDA_GPU_ADAPTER`)

---

## Active Workarounds

### W-001: f64 Transcendental Polyfills

SPIR-V has no mechanism to link vendor math libraries (libdevice/ocml). `compile_shader_f64()` polyfills 28 transcendental functions via pure WGSL. Applies to all drivers (NVK, RADV, NVIDIA proprietary). Architecturally solved — not a workaround but the sovereign solution.

- **ACO (AMD)**: Contribute `fexp2(f64)` implementation to Mesa RADV/ACO for RDNA2/3
- **NAK (NVIDIA)**: Contribute `exp(f64)` lowering after Titan V hardware validation
- **Validate**: `bench_f64_builtins` on Titan V + RTX 4070 to complete capability matrix

### W-003: NAK Compiler — Titan V Hardware Validation

Phases 0–3 complete. Optimizer wired into `compile_shader_f64()`.

**Pending**: Run `bench_wgsize_nvk` on Titan V to measure ILP pre-scheduling speedup
and confirm >= 3x before submitting the Mesa MR.

### W-004: NAK Mesa Patches (5 Deficiencies)

| Priority | Deficiency | Expected Gain | Mesa Location |
|----------|-----------|---------------|--------------|
| 1 | Loop unrolling | ~4x | `nak/opt_instr.rs` / `lower_vec.rs` |
| 2 | Register allocation | ~2x | `nak/ra.rs` |
| 3 | Instruction scheduling | ~1.5x | `nak/sched.rs` |
| 4 | FMA fusion | ~1.3x | `nak/lower_fma.rs` |
| 5 | Branch predicates | ~1.1x | `nak/opt_pred.rs` |

See `contrib/mesa-nak/NAK_DEFICIENCIES.md` for full decomposition.

---

## Upcoming

### P1: DF64 Transcendentals — Extended Coverage

Extend DF64 transcendentals to cover remaining functions:
- [ ] `asin_df64`, `acos_df64`, `atan_df64`, `atan2_df64`
- [ ] `sinh_df64`, `cosh_df64`
- [ ] `gamma_df64`, `erf_df64` (Lanczos/Abramowitz at DF64 precision)

### P2: Architecture-Specific Polynomial Selection (Q3 2026)

Different evaluation strategies per silicon family:
- SM70 (Volta): 8-cycle ILP fill — longer Horner chains
- SM80+ (Ampere/Ada): 4-cycle ILP — Estrin evaluation may beat Horner
- RDNA2/3 (AMD): VALU utilization patterns differ from NVIDIA
- Requires profiling data per silicon before implementation

### Sovereign Phase 4 — Full naga-IR Optimizer (Q3 2026)

Drive naga as a library for full SSA-form analysis and register pressure estimation.

```
WGSL text
  → naga::parse() → naga::Module (typed IR)
  → BarraCuda IR passes (reorder, unroll, software pipeline)
  → modified naga::Module
  → naga::back::spv::write() → SPIR-V bytes  (no WGSL round-trip)
  → wgpu device
```

### Infrastructure

- [ ] **ComputeDispatch migration** -- Builder pattern created; migrate 2-3 existing ops
- [ ] **Conv2D/Pool full parametric support** -- WGSL exists, single-channel wired; stride/padding/channels pending (D-S46-001)
- [ ] **NVK/Titan V readiness** -- Ensure f64 workarounds complete for NVK/Volta + NAK-specific paths
- [ ] **NPU model pipeline** -- train/compile/deploy from Rust (VFIO backend exists)
- [ ] **Test coverage target 90%** -- `cargo llvm-cov` gap analysis needed
- [ ] **PCoA BatchedEighGpu** -- naga "invalid function call" in eigensolve shaders

### Cross-Repo Debt

- [ ] **D-S20-003**: neuralSpring `evolved/` migration (~2075 lines) — awaiting neuralSpring team
- [ ] **D-S18-002**: cubecl transitive `dirs-sys` — needs upstream PR

---

See [CHANGELOG.md](CHANGELOG.md) for full completed session history.
