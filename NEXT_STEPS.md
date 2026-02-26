# ToadStool/BarraCuda -- Next Steps

**Updated**: February 25, 2026 -- Session 65
**Status**: Production-grade | 694 WGSL f64 shaders (14 DF64) | 2,490 barracuda tests | All quality gates green

---

## Completed This Session

### Session 65: Smart Refactoring ✅

- **5 large files refactored** (32-44% reductions): `compute_graph.rs` 819→522 (generic `compile_shader`/`dispatch_pass`), `esn_v2/model.rs` 861→482, `tensor/mod.rs` 808→529 (merged duplicate test), `special/gamma.rs` 685→463, `numerical/rk45.rs` 579→352. Tests extracted to `*_tests.rs` files.
- **Production panic eliminated**: `coulomb_f64` `map_async` callback `expect()` → `let _ = tx.send()`.
- **Hardcoding eliminated**: `kernel_router.rs` — 7 magic routing thresholds → named constants.
- **Dead code**: 14→13 (`compute_graph.device_name` → public accessor). All 13 remaining are Phase 5+ reserved.

### Session 64: Cross-Spring Absorption ✅

- **8 lattice QCD shaders** absorbed from hotSpring: `su3_math_f64`, `prng_pcg_f64`, `su3_f64`, `su3_gauge_force_df64`, `su3_kinetic_energy_df64`, `axpy_f64`, `complex_dot_re_f64`, `xpay_f64`.
- **`stats::metrics`** module: RMSE, MBE, NSE, R², Index of Agreement, hit_rate, mean, percentile (18 tests).
- **`stats::diversity`** module: Shannon, Simpson, Chao1, Pielou, Bray-Curtis, rarefaction (16 tests).
- **`chrono` eliminated**: `chrono::Local::now()` → `std::time::SystemTime`.
- **3 dead_code resolved**: `BroydenMixer::device()/vec_dim()`, `KrigingF64::device()`, `KernelRouter::has_tpu()`.

### Sessions 61-63: Sovereign Compiler + Deep Debt ✅

- `SovereignCompiler` — naga-IR optimizer: WGSL → FMA fusion → DCE → SPIR-V emit. Three-stage `compile_shader_f64()` pipeline.
- 25+ `#[allow(dead_code)]` evolved to documented `pub` API. `solve_gpu_parallel` wired for n≥2048. `partial_maximin` O(n) wired into maximin_lhs.
- Smart refactoring: `morse_f64.rs` 953→804, `coulomb_f64/mod.rs` 610→369. `instant` crate removed. Platform stubs evolved.

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
