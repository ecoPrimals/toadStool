# ToadStool/BarraCuda -- Next Steps

**Updated**: February 24, 2026 -- Session 67
**Status**: Production-grade | 700 WGSL shaders (21 DF64, 0 f32-only, 296 consolidated) | 2,546 barracuda tests | Universal precision pipeline | Precision gate OPEN | All quality gates green

---

## Completed This Session

### Session 66: Cross-Spring Absorption + Deep Debt + Multi-Precision Expansion ✅

**Wave 4 — Cross-Spring Evolution (All 5 Springs + wateringHole):**
- **stats::mae**, **shannon_from_frequencies()**, **hill()/monod()** public APIs (+13 tests)
- **WGSL_RK4_PARALLEL** re-exported, **PRNG f64 polyfill** (su3_random_momenta)
- **Sovereign compiler fix**: BatchedElementwiseF64 explicit BindGroupLayout (SPIR-V passthrough panic)
- **NeighborMode** lattice abstraction, **NPU hardcoding** → shared constants, **RK step-size** named constants

**Wave 5 — Multi-Precision Expansion (Unleashing All Silicon):**
- **`compile_shader_df64()`** — first-class DF64 compilation pipeline
- **6 universal DF64 math shaders** — elementwise add/mul/sub/fma, sum/mean reduce
- **5 f64 reduce gap-fills** — mean/std reduce, sum/mean/std dim
- **2 f64 science losses** — mse_loss_f64, mae_loss_f64
- **Full precision inventory** — 700 shaders: all f64 canonical, 21 DF64, 0 f32-only. 296 f32 WGSL files deleted
- **700 WGSL shaders, 2,546 tests, 0 clippy warnings**

**Wave 1 — Absorption + Initial Refactoring:**
- **4 new modules absorbed** from airSpring V009 + groundSpring V7: `stats::regression` (12 tests), `stats::hydrology` (13 tests), `stats::moving_window_f64` (7 tests), `stats::bootstrap::rawr_mean` (4 tests).
- **`spearman_correlation`** re-exported (was private).
- **Richards PDE evolved**: 8 named soil constants (Carsel & Parrish 1988), Picard buffer preallocation, magic numbers → named constants.
- **Smart refactoring**: `morse_f64.rs` 804→556, `resource_quota.rs` 795→547.
- **Dead code 13→10**: `griffin_lim` fields → STFT validation, `fhe_key_switch` pipeline wired, `nn/mod.rs` blanket allow removed.
- **+36 new tests**, all passing.

**Wave 2 — Deep Debt + Dependency Evolution:**
- **12 more files refactored**: `workload.rs` (812→452), `cholesky.rs` (815→557), `cubic_spline.rs` (788→590), `batched_bisection_gpu.rs` (758→562), `anderson.rs` (657→486), `timeseries.rs` (640→477), `genomics.rs` (682→537), `solvers.rs` (692→551), `filter.rs` (705→636), `fused_map_reduce_f64.rs` (624→532), `spin_orbit_f64.rs` (623→508), `gpu_hmc_trajectory.rs` (785→767).
- **`anyhow` crate eliminated**: 3 files migrated to typed `BarracudaError` via `thiserror`. Proper library error handling — no more `anyhow::bail!` in a library crate.
- **`async_trait` analyzed and justified**: Required for dyn-compatible async trait objects (`ComputeExecutor`, `TensorStorage`). Native async fn in traits not dyn-safe — this dependency provides real value.
- **Dead code 10→3**: `timeseries.device` → accessor, `vision.device` → accessor, `ring_buffer.staging_buffer` → accessor, `unidirectional.device` → accessor. Remaining 3 are feature-gated (TPU), PCIe diagnostic (Akida), and Phase 5+ placeholder.
- **Production `expect()` audit**: 2 evolved in `observables/mod.rs` (→ `if let` + direct indexing). All 29 remaining are Mutex/RwLock poison guards (correct Rust practice).
- **DF64 force shaders confirmed complete**: All 4 (Born-Mayer, Morse, Yukawa, Lennard-Jones) with full WGSL implementations wired via `Fp64Strategy::Hybrid`.

**Wave 3 — Hardcoding Elimination + Final Dependency Cuts:**
- **GPU executor scoring**: 15 named constants for routing thresholds (`mod scoring {}`) replacing ~30 inline literals.
- **Timeseries ESN defaults**: 6 named constants for ESN configuration and anomaly detection window sizing.
- **`shaders/precision/mod.rs`** 733→452 (38% reduction) — last large file with inline tests.
- **`log` crate eliminated**: 68 `log::*!` calls migrated to `tracing::*!` across 18 files. Single unified logging facade — `log = "0.4"` removed from Cargo.toml.

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

### P0: Universal Precision Shaders — Math Is Universal, Precision Is Silicon

**Principle**: Every shader runs at every precision. The WGSL is universal math. Precision is a compilation pipeline detail — just as we solved f64 builtins with polyfills, we solve multi-precision with compilation.

**Proven**: hotSpring ran 32⁴ QCD on 3090's 1.6% FP64 cores. DF64 gives 9.9× throughput vs native f64. Consumer GPU FP64 is real.

**Inventory (700 shaders, all f64 canonical)**:
- ~50 "conceptually universal" shaders (elementwise, reduce, loss, basic linalg) — math doesn't change, only type surface (~6-15 lines per shader)
- ~180 transcendental-dependent shaders — need polyfill pipeline (`compile_shader_f64()` / `compile_shader_df64()`)
- ~80 precision-critical (lattice QCD, MD) — already f64/df64

**Architecture**: Rust-side codegen emits f32/f64/Df64 variants from one source template. The `compile_shader_f64()` and `compile_shader_df64()` pipelines already prove the pattern. Evolution:
1. Template system for the ~50 universal math shaders (type surface is small)
2. Extend to transcendental-dependent shaders via polyfill injection
3. Runtime `Fp64Strategy` selects f32/df64/f64 based on hardware capability

**Infrastructure already built**:
- `compile_shader_f64()` — 3-stage pipeline (driver patch → ILP → sovereign compiler)
- `compile_shader_df64()` — auto-injects df64_core + df64_transcendentals
- `Fp64Strategy::Native/Hybrid` — runtime precision selection
- `math_f64.wgsl` — 28 transcendental polyfills
- `df64_core.wgsl` + `df64_transcendentals.wgsl` — FMA-optimized DF64 arithmetic + transcendentals

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
