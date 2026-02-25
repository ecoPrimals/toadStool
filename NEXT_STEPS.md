# ToadStool/BarraCuda -- Next Steps

**Updated**: February 25, 2026 -- Session 63
**Status**: Production-grade | 687 WGSL f64 shaders (12 DF64) | 2,440 barracuda tests | All quality gates green

---

## Completed This Session

### Sessions 62-63: Deep Debt Evolution ✅

- **Codebase-wide dead code resolution**: 25+ `#[allow(dead_code)]` WGSL constants evolved to documented `pub` API. Fossil `wgsl_shader()` methods → `pub const`. Electrostatics shader constants re-exported.
- **Smart refactoring**: `morse_f64.rs` 953→804 lines (MorseBuffers + reduce_bond_forces). `coulomb_f64/mod.rs` 610→369 lines (CoulombBuffers + staging helpers). Zero GPU pipeline duplication.
- **Dead code → live code**: `solve_gpu_parallel` wired for n≥2048 (cyclic reduction). `partial_maximin` O(n) wired into maximin_lhs (was O(n²)). `erfc_deriv` promoted to public electrostatics API.
- **Platform evolution**: OS compat stubs return `SystemError::NotSupported` on wrong platform. Discovery fallbacks table-driven with env guard.
- **Hygiene**: `instant` crate removed, `WebGPUAdapter::mock_data` → zero-size `()`, GriffinLim annotations cleaned, `fhe_key_switch` dead constant removed.

### Session 61: Sovereign Compiler Phase 4 ✅

- `SovereignCompiler` — naga-IR optimizer: parse WGSL → FMA fusion → DCE → SPIR-V emit
- FMA fusion pass: detects `Mul(a,b) + c` patterns, replaces with `fma(a, b, c)` (~1.3x)
- SPIR-V passthrough: `SPIRV_SHADER_PASSTHROUGH` in all device creation paths
- `compile_shader_f64()` evolved to three-stage pipeline with WGSL text fallback

### Session 60: DF64 FMA + Transcendentals ✅

- `df64_core.wgsl`: FMA-optimized `two_prod` (17 ops → 2), `df64_mul` cross-terms use FMA
- New `df64_transcendentals.wgsl`: sqrt, exp, log, sin, cos, pow, tanh at FP32 core speed
- 4 force shaders evolved to all-DF64 (Born-Mayer, Morse, Yukawa, Lennard-Jones)
- Patcher hardened against ldexp/exp_df64 substring collisions

---

## Active Workarounds

### W-001: f64 Transcendental Polyfills

SPIR-V has no mechanism to link vendor math libraries (libdevice/ocml). `compile_shader_f64()` polyfills 28 transcendental functions via pure WGSL. Applies to all drivers (NVK, RADV, NVIDIA proprietary). Architecturally solved — not a workaround but the sovereign solution.

- **ACO (AMD)**: Contribute `fexp2(f64)` implementation to Mesa RADV/ACO for RDNA2/3
- **NAK (NVIDIA)**: Contribute `exp(f64)` lowering after Titan V hardware validation
- **Validate**: `bench_f64_builtins` on Titan V + RTX 4070 to complete capability matrix

### W-003: NAK Compiler — Titan V Hardware Validation

Phases 0–4 complete. Sovereign compiler (naga-IR + SPIR-V passthrough) wired into `compile_shader_f64()`.
Phase 4 FMA fusion addresses Deficiency 4 at the IR level for all backends.

**Pending**: Run `bench_wgsize_nvk` on Titan V to measure combined ILP + FMA speedup
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

### Sovereign Phase 4+ — naga-IR Optimizer Evolution

Phase 4 core is DONE (FMA fusion, DCE, SPIR-V passthrough). Remaining iterations:

- [ ] Register pressure estimation (live-range counting on naga expression arena)
- [ ] Loop software pipelining at naga IR level (preload iteration i+1 during i's ops)
- [ ] Architecture-specific peephole optimization per `GpuArch`
- [ ] naga → NAK IR direct bridge — bypass `spirv_to_nir` (C) for full end-to-end Rust (research)

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
