# ToadStool/BarraCuda -- Next Steps

**Updated**: February 26, 2026 -- Session 68++
**Status**: Production-grade | AGPL-3 compliant | 0 clippy warnings (--all-targets) | Standalone-resilient | 700 WGSL shaders | 2,546+ barracuda tests | 43% coverage (target: 90%)
**Evolving**: Springs transition from fp64 shaders → true math. Coverage gap analysis. chrono full elimination.

---

## Completed This Session

### Session 68+: Standalone Resilience — Deep Debt ✅

- **GPU device-lost recovery**: `install_error_handler` no longer panics on device-lost — flags and returns. `submit_and_poll_inner` catches wgpu internal panics via `catch_unwind`, converts device-lost to `lost` flag. `read_buffer`/`map_staging_buffer` early-return `Err` when device is lost.
- **All submit paths hardened**: `compute_graph.rs` and `pppm_gpu/mod.rs` direct `queue.submit` calls wrapped in `catch_unwind`.
- **Test parallelism**: `.cargo/config.toml` sets `RUST_TEST_THREADS=4` (override: `RUST_TEST_THREADS=N cargo test`).
- **Stale debris archived**: 5 scripts + 4 docs → `ecoPrimals/fossil/`, `run-coverage.sh` fixed, `PRECISION_BOTTLENECK.md` archived as resolved gate.
- **Result**: 128 false test failures → 0. Pull to any machine, `cargo test` works.

### Session 68: Dual-Layer Universal Precision + Precision Bottleneck ✅

- **Dual-layer architecture**: Layer 1 — `op_preamble` (abstract ops for F16/F32/F64/DF64 via `compile_op_shader()`). Layer 2 — naga-guided `df64_rewrite` (bridge functions for infix f64→DF64).
- **Precision bottleneck RESOLVED**: 296 f32 WGSL files deleted. Zero f32-only shaders. All f64 canonical.
- **F16 downcast hardened**: sentinel protection + f16 literal clamping (±65504.0).
- **DF64 ghost mappings cleaned**: 8 non-existent transcendental mappings removed.
- **NaN-safe bridge functions**: IEEE 754 compliant `_df64_gte_f64`/`_df64_lte_f64`.
- **122 shader tests**: unit + e2e + chaos (15) + fault (13).
- **Comprehensive audit**: span bounds, undefined span fallbacks, op_pack/op_unpack consistency.

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

### P0: Universal Precision Shaders — OPERATIONAL ✅

**Principle**: Every shader runs at every precision. Math is universal, precision is silicon.

**Status**: Dual-layer architecture complete and tested (122 tests). All 700 shaders are f64 canonical. Precision bottleneck resolved.

**Infrastructure**:
- `compile_shader_universal(source, precision)` — routes one source to f16/f32/f64/df64
- `compile_op_shader(source, precision)` — abstract `op_add`/`op_mul` work at all precisions
- `Precision::op_preamble()` — F16/F32/F64/DF64 preambles with full op coverage
- `downcast_f64_to_f32/f16/df64()` — text-based with sentinel protection + literal clamping
- `sovereign/df64_rewrite.rs` — naga-guided f64 infix → DF64 bridge functions
- `compile_shader_f64/df64()` — polyfill injection pipelines (28 transcendentals)
- `Fp64Strategy::Native/Hybrid` — runtime precision selection

**Remaining evolution**:
- Migrate existing shaders from direct type usage to `op_add`/`op_mul` where beneficial
- Extend DF64 transcendental coverage (see P1)

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
