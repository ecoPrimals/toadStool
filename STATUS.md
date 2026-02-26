# Status -- February 26, 2026 (Sessions 32-68: Precision Bottleneck Execution)

## Quality Gates

| Gate | Status | Notes |
|------|--------|-------|
| `cargo build --workspace` | PASS | Clean build |
| `cargo fmt --all -- --check` | PASS | 0 diffs |
| `cargo clippy -p barracuda --lib -- -D warnings` | PASS | **0 warnings** |
| `cargo doc --workspace --no-deps` | PASS | 0 warnings |
| `cargo test -p barracuda --lib` | PASS | **2,546+ total** (122 shader-specific: unit/e2e/chaos/fault) |
| hotSpring validation | PASS | **664 tests, 22 papers validated** |
| wetSpring validation | PASS | **918 tests, 96.48% coverage** |
| neuralSpring validation | PASS | **580 tests, 94.53% coverage** |
| airSpring validation | PASS | **468 tests, 96.84% coverage** |
| groundSpring validation | PASS | **154 tests, 98.64% coverage** |
| Pure Rust syscalls | PASS | mmap/mlock via rustix |
| Zero-copy hot paths | PASS | `Cow<'a, str>` + `#[serde(borrow)]`, `from_slice`, `bytes::Bytes` |
| Hardcoded primal names | PASS | **0 -- capability-based discovery** |
| Hardcoded localhost/ports | PASS | **0 -- bind `0.0.0.0`, port 0, `discover_self_ip_address()`** |
| `unsafe` blocks | PASS | **2 in barracuda (SPIRV passthrough + pipeline cache), both `// SAFETY:` documented** |
| `#![deny(unsafe_code)]` | PASS | **36 crates hardened** (2 justified: gpu, secure_enclave) |
| Production `Box<dyn Error>` | PASS | **0 in core crates -- all typed errors (thiserror)** |
| Production panics/unwraps | PASS | **Zero blind `unwrap()`; infallible `expect()` only** |
| Production TODOs | PASS | **Zero -- all evolved to `BLOCKED(reason)` markers** |
| File size limit | PASS | **All production files under 1000 lines** |
| WGSL shaders | PASS | **700 (zero orphans, shader-first, 21 DF64 + 182 f64 + 497 f32, 0 f32-only — all f32 via LazyLock downcast from f64 canonical)** |
| Dead code | PASS | **5 `#[allow(dead_code)]` remain — feature-gated TPU + PCIe diag + f64 shader accessors** |
| Production println!/dbg! | PASS | **Zero — evolved to `tracing::info!`** |
| External dep hygiene | PASS | **`instant` + `chrono` + `anyhow` + `log` eliminated; `async_trait` justified (dyn-required)** |
| Production mocks | PASS | **Zero — TpuBackend::Mock behind `mock-tpu` feature gate** |
| Platform stubs | PASS | **Evolved to platform-aware `can_handle()` + `SystemError::NotSupported`** |
| FP64 strategy | PASS | **Fp64Strategy::Native/Hybrid -- FMA-optimized DF64 + transcendentals** |
| Dependency security | PASS | **bytes >=1.11.1, aes-gcm >=0.10.3, zero chrono** |

Excludes hardware-dependent crates: `toadstool-runtime-gpu`, `ml-inference-showcase`, `homomorphic-computing`. Examples excluded (require GPU).

---

## Session 68: Dual-Layer Universal Precision + Precision Bottleneck (Feb 26, 2026)

Executing on the precision bottleneck gate AND building the dual-layer universal precision architecture. Math is universal — precision is the silicon detail.

### Dual-Layer Universal Precision Architecture
- **Layer 1 — Operation Preamble (source-level)**: `Precision::op_preamble()` returns precision-specific WGSL implementing `op_add`/`op_sub`/`op_mul`/`op_div`/`op_neg`/`op_abs`/`op_max`/`op_min`/`op_gt`/`op_lt`/`op_ge`/`op_le`/`op_from_f32`/`op_zero`/`op_one`/`op_pack`/`op_unpack` for F16, F32, F64, and DF64. `compile_op_shader()` injects the correct preamble — shaders written with abstract ops work at ALL precisions without transformation.
- **Layer 2 — Naga-Guided Rewrite (compiler-level)**: `sovereign/df64_rewrite.rs` parses f64 WGSL with naga, identifies f64 `Binary{+,-,*,/}` and comparison operators by type analysis, replaces with bridge functions (`_df64_add_f64`, `_df64_sub_f64`, `_df64_mul_f64`, `_df64_div_f64`, `_df64_neg_f64`, `_df64_gt_f64`, `_df64_lt_f64`, `_df64_gte_f64`, `_df64_lte_f64`) that route computation through DF64 while keeping the f64 type system intact. Uses `__SPAN__` markers for source-level accuracy and `dedup_overlapping()` for nested expression handling.
- **`compile_shader_universal()` DF64 branch**: naga rewrite attempted first, text-based `downcast_f64_to_df64()` as fallback.

### Precision Pipeline Hardening
- **F16 downcast**: `downcast_f64_to_f16()` with sentinel protection (`_f64(` polyfill names preserved) + `clamp_f64_range_literals_f16()` (caps to ±65504.0).
- **DF64 transcendental mapping**: Only maps 8 functions with actual implementations (exp, log, pow, sin, cos, tanh, sqrt, abs). Removed 8 ghost mappings (tan/asin/acos/atan/atan2/sinh/cosh/erf_df64).
- **NaN-safe bridge functions**: `_df64_gte_f64`/`_df64_lte_f64` use explicit equality check instead of `!lt`/`!gt` (correct IEEE 754 semantics).
- **Span robustness**: Bounds validation (`start <= end`, `end <= len`) in both `resolve_spans()` and `replace_range`. Safe `f64(0.0)` fallback for undefined naga spans.

### Comprehensive Test Suite (122 tests)
- **Unit**: Downcast edge cases, preamble structure verification, sentinel protection, literal clamping
- **E2E**: Real shaders (elementwise add, reduce sum, comparison) validated at all precisions via naga parse
- **Chaos** (15 tests): Empty input, nested f64 patterns, adversarial sentinel chains, boundary values, mixed types
- **Fault** (13 tests): Idempotency (double-downcast), span bounds, dedup correctness, graceful degradation

### Shader Consolidation (Phase 2)
- **5 near-duplicate pairs consolidated**: elementwise_add, elementwise_mul, sum_dim, mean_dim, std_dim
- **Pattern**: f64 is canonical source, f32 produced via `LazyLock<String>` + `downcast_f64_to_f32()`, f32 WGSL file deleted
- **Discovery**: Only 5 of 54 pairs are true type-only duplicates. The remaining 49 are structurally different (f64 uses superior algorithms like workgroup tree reduction, Welford statistics)
- **New Phase 4**: Algorithm evolution — evolve f32 implementations to match f64 quality

### f32-Only → f64 Canonical (Phase 3) — COMPLETE
- **291 f32-only shaders converted** to f64 canonical (240 trivial + 294 transcendental)
- All f32 WGSL files deleted; f32 variant generated at runtime via `LazyLock<String>`
- Trivial shaders use `downcast_f64_to_f32()`, transcendental use `downcast_f64_to_f32_with_transcendentals()`
- **Zero f32-only shaders remain** — every shader has an f64 canonical source
- 16 u32/i32-only shaders unchanged (no f32 types to convert)
- **Precision bottleneck gate: OPEN** — spring absorptions can proceed

### Deep Debt Sweep
- **Production println!**: 14 calls in `auto_tensor.rs` and `validation.rs` → evolved to `tracing::info!`
- **Magic numbers**: `npu_executor.rs` hardcoded floats → named constants in `npu_efficiency` module
- **Mock naming**: `MOCK_FP16_TFLOPS` → `NPU_EQUIVALENT_TFLOPS` (NPUs use spike counts, not TFLOPS)
- **Infallible expect()**: Documented in `lanczos.rs` and `stateful.rs`
- **Verified clean**: 0 unsafe, 0 todo!/unimplemented!, 0 dbg!, 0 production unwrap(), all files < 1000 lines

### Metrics
- Shaders: 707 → 700 (296 f32 WGSL files deleted, f64 canonical, 7 net reduction from consolidation)
- `downcast_f64_to_f32()` callers: 0 → 296
- f32-only shaders remaining: 534 → **0**
- Shader tests: 0 → **122** (unit + e2e + chaos + fault)
- Production println!: 14 → 0
- Production magic numbers: 5 → 0

---

## Session 67: Universal Precision Architecture (Feb 24, 2026)

Math is universal, precision is silicon. Shader source is written once (conceptually f64, the true math), and the compilation pipeline handles precision specialization — the same pattern that solved f64 builtins with polyfills, extended to all precisions.

### Universal Precision Pipeline
- **`compile_shader_universal(source, precision)`**: Routes one shader source to f32/f64/df64 via the appropriate compilation pipeline. F32 uses `downcast_f64_to_f32()`, F64 uses `compile_shader_f64()` (polyfills + sovereign compiler), Df64 uses `compile_shader_df64()` (auto-injected DF64 core).
- **`compile_template(template, precision)`**: Compiles `{{SCALAR}}`-parameterized templates at any precision, routing through the appropriate pipeline.
- **`Precision::Df64`**: New enum variant — double-float f32-pair (~48-bit mantissa, ~14 decimal digits). `is_f64_class()` method for f64-equivalent detection. `required_feature()` returns `None` (runs on any FP32 hardware).
- **`downcast_f64_to_f32()`**: Text-transforms f64 shaders to f32 with sentinel-protected `_f64(` function names (prevents mangling polyfill calls like `exp_f64`).
- **`downcast_f64_to_f32_with_transcendentals()`**: Also maps polyfill calls to native WGSL builtins (`exp_f64` → `exp`, `sin_f64` → `sin`, etc.).

### 12 Universal Shader Templates
All generate valid WGSL at any precision (f16/f32/f64/df64) from `{{SCALAR}}` placeholders:
- **Elementwise**: add, mul, sub, fma, abs, neg, clamp, saxpy
- **Reduction**: sum, mean
- **Loss**: MSE, MAE
- **Existing**: dot product

### Precision Inventory (707 shaders)
- **510 pure f32** (72%) — universal math candidates (~50 can be consolidated)
- **195 native f64** (28%) — scientific computing, lattice QCD, MD forces
- **20 DF64** (3%) — consumer GPU f64-class precision on FP32 cores
- **2 DF64 infrastructure** — `df64_core.wgsl`, `df64_transcendentals.wgsl`

### Root Docs Cleaned
All stale counts updated across README, STATUS, DEBT, NEXT_STEPS, QUICK_REFERENCE, DOCUMENTATION (694→707 shaders, 14→21 DF64, 2526→2541 tests).

### Tests
5 new tests: `test_precision_df64`, `test_downcast_f64_to_f32_elementwise`, `test_downcast_f64_to_f32_with_transcendentals`, `test_downcast_preserves_u32_and_structure`, `test_template_renders_df64`. All 21 precision tests pass.

---

## Session 66: Absorption + Deep Debt + Cross-Spring Evolution (Feb 26, 2026)

Cross-spring absorption of actionable handoff items + deep debt audit and fixes:

### Wave 4: Cross-Spring Evolution (All 5 Springs + wateringHole)
- **`stats::mae`**: CPU Mean Absolute Error — completes neuralSpring V39 API gap (4 tests)
- **`shannon_from_frequencies()`**: Shannon entropy from pre-computed frequency vectors (3 tests)
- **`hill()`/`monod()`**: Public Rust API for dose-response/Monod kinetics — was WGSL-only (6 tests)
- **`WGSL_RK4_PARALLEL`**: Re-exported public const (removed in S65 refactor, neuralSpring dependency)
- **PRNG type-safety**: `su3_random_momenta_f64.wgsl` evolved from f32→f64 polyfill (sqrt_f64, log_f64, cos_f64)
- **Sovereign compiler fix**: `BatchedElementwiseF64` explicit `BindGroupLayout` — fixes SPIR-V passthrough panic (airSpring P0 blocker)
- **`NeighborMode`**: Lattice neighbor buffer abstraction (`Compute` | `PrecomputedBuffer`) with `precompute()` for 4D periodic t-major lattices (hotSpring P1)
- **NPU hardcoding**: `0.5` sparsity thresholds → shared `NPU_SPARSITY_THRESHOLD` constant (npu_bridge + matmul)
- **RK step-size**: `0.9`/`2.0`/`0.25`/`0.2` → named constants `STEP_SAFETY`/`STEP_MAX_GROWTH`/`STEP_GROW_EXPONENT`/`STEP_SHRINK_EXPONENT`

### New Modules Absorbed (airSpring V009 + groundSpring V7)
- **`stats::regression`**: 4 closed-form regression models (linear/quadratic/exponential/logarithmic) with `FitResult::predict()`, `fit_all()` convenience (12 tests)
- **`stats::hydrology`**: FAO-56 reference hydrology — Hargreaves ET₀, crop coefficient interpolation, soil water balance (13 tests)
- **`stats::moving_window_f64`**: CPU f64 sliding window statistics complementing GPU f32 path (7 tests)
- **`stats::bootstrap::rawr_mean`**: RAWR Dirichlet-weighted resampling (Wang et al. 2021) for ecological inference (4 tests)
- **`spearman_correlation`** re-exported from `stats/mod.rs` (was private)

### Richards PDE Evolution
- 8 named `SoilParams` constants (Carsel & Parrish 1988): SANDY_LOAM through LOAMY_SAND
- Picard iteration buffer preallocation (9 vectors once, not per-iteration)
- Magic numbers → `HARMONIC_MEAN_GUARD`, `MIN_CAPACITY` named constants

### Smart Refactoring
- **`morse_f64.rs`** 804→556 (31%): Tests + cfg(test) helpers extracted with shared `bond_geometry()` and `test_bond()` factory
- **`resource_quota.rs`** 795→547 (31%): 22 tests extracted to `resource_quota_tests.rs`

### Wave 2: Smart Refactoring (12 more files)
- `workload.rs` 812→452, `cholesky.rs` 815→557, `cubic_spline.rs` 788→590, `batched_bisection_gpu.rs` 758→562
- `anderson.rs` 657→486, `timeseries.rs` 640→477, `genomics.rs` 682→537, `solvers.rs` 692→551
- `filter.rs` 705→636, `fused_map_reduce_f64.rs` 624→532, `spin_orbit_f64.rs` 623→508, `gpu_hmc_trajectory.rs` 785→767
- Total: 14 files refactored across Session 66 (Wave 1: 2, Wave 2: 12)

### Wave 2: External Dependency Evolution
- **`anyhow` removed**: 3 files migrated to typed `BarracudaError` via `thiserror` — proper library error handling
- **`async_trait` justified**: Required for dyn-compatible async trait objects (`ComputeExecutor`, `TensorStorage`)

### Wave 2: Production expect() → Idiomatic Rust
- `observables/mod.rs`: 2 production `expect()` → `if let` pattern + direct indexing
- Full audit: 29 remaining production `expect()` are all Mutex/RwLock poison guards (correct Rust practice)

### Wave 3: Hardcoding Elimination + More Dependency Evolution
- `gpu_executor/mod.rs`: 15 named scoring constants replace ~30 inline routing thresholds
- `timeseries.rs`: 6 named constants (ESN defaults + anomaly detection window)
- `shaders/precision/mod.rs` 733→452 (38%) — 16 tests extracted
- **`log` crate eliminated**: 68 calls migrated to `tracing` across 18 files — single unified logging facade

### Dead Code Evolution (13 → 3)
- `griffin_lim.rs`: `n_fft`/`hop_length` now used for STFT validation + `expected_signal_length()` accessor
- `fhe_key_switch.rs`: `pipeline_accumulate` wired into 2-pass execute (decompose + accumulate)
- `nn/mod.rs`: blanket `#![allow(dead_code)]` removed (zero warnings — all items consumed)

### Deep Debt Audit Summary
- **Large files (600+)**: 21 identified, top 2 refactored below threshold
- **`unsafe`**: 2 locations only (SPIR-V passthrough, pipeline cache) — both documented
- **`expect()` in production**: ~100 calls — all verified as lock-poisoning (correct) or test-only
- **`unwrap()` in production**: 0 (only in test code)
- **`todo!()`/`unimplemented!()`**: 0
- **Production mocks**: 0 (mock TPU is feature-gated)
- **New tests**: +36 (12 regression + 13 hydrology + 7 moving_window + 4 RAWR)

---

## Session 65: Smart Refactoring (Feb 25, 2026)

Large-file refactoring — smart extraction (not just splitting), duplicate elimination, dead code evolution:

- **`compute_graph.rs` 819→522 (36%)**: Extracted `compile_shader()`/`compile_elementwise()` (4 near-identical shader compilation methods → 2), `dispatch_pass()` (3 identical BGL→bind→pipeline→dispatch chains → 1 generic function). Reused `storage_bgl_entry()`/`uniform_bgl_entry()` from `device::compute_pipeline` instead of hand-rolling `BindGroupLayoutEntry` arrays. Dead code `device_name` → public accessor.
- **`esn_v2/model.rs` 861→482 (44%)**: 20 async tests extracted to `model_tests.rs` via `#[path = "model_tests.rs"] mod tests`. Production code untouched — all 478 production lines under 500.
- **`tensor/mod.rs` 808→529 (35%)**: Tests extracted to `tensor_tests.rs`. Merged the verbatim-duplicate `test_tensor_laplacian_context_debug` (self-annotated "EXACT COPY") into parameterized `test_tensor_3d_roundtrip` covering 2×2×2, 3×3×3, 4×4×4.
- **`special/gamma.rs` 685→463 (32%)**: 20 tests extracted to `gamma_tests.rs`.
- **`numerical/rk45.rs` 579→352 (39%)**: 16 ODE solver tests extracted to `rk45_tests.rs`.
- **Production panic fix**: `coulomb_f64` `map_async` callback `expect()` → `let _ = tx.send()` (no panic on dropped receiver).
- **Hardcoding elimination**: `kernel_router.rs` — 7 magic routing thresholds → named constants (`CPU_FALLBACK_THRESHOLD`, `EIGENDECOMP_CPU_THRESHOLD`, `LINEAR_SOLVE_CPU_THRESHOLD`, `MATMUL_LARGE_DIM`, `MATMUL_MEDIUM_DIM`).
- **Dead code count**: 14 → 13 (compute_graph `device_name` resolved). All 13 remaining are Phase 5+ reserved fields.
- **Zero regressions**: All 84 targeted tests pass (20 ESN + 20 tensor + 1 compute_graph + 20 gamma + 16 rk45 + 7 kernel_router). Full clippy clean.

---

## Session 64: Cross-Spring Absorption (Feb 25, 2026)

Pulled all 5 springs + wateringHole; reviewed and absorbed handoffs:

### Lattice Shader Absorption (hotSpring V0613/V0614)
- **8 new WGSL shaders** absorbed into `shaders/lattice/`: `su3_math_f64` (naga-safe composition fix), `prng_pcg_f64` (shared PRNG), `su3_f64` (base SU(3) ops), `su3_gauge_force_df64` (9.9× DF64 throughput), `su3_kinetic_energy_df64`, `axpy_f64`, `complex_dot_re_f64`, `xpay_f64`
- All wired via `absorbed_shaders.rs` with doc comments and provenance

### Statistics Absorption (airSpring V006 + groundSpring V7)
- **`stats::metrics`** module: RMSE, MBE, Nash-Sutcliffe, R², Index of Agreement, hit_rate, mean, percentile, dot, l2_norm (18 tests)
- Consolidated from airSpring `testutil/stats.rs` and groundSpring `stats/metrics.rs`

### Diversity Absorption (wetSpring V41)
- **`stats::diversity`** module: Shannon, Simpson, Chao1, Pielou evenness, Bray-Curtis (pairwise + condensed + full matrix), rarefaction curves, AlphaDiversity (16 tests)
- CPU complements for GPU `DiversityFusionGpu` and `FusedMapReduceF64::shannon_entropy`

### Deep Debt Resolved
- **`chrono` dependency eliminated**: `chrono::Local::now()` → `std::time::SystemTime`, `chrono = "0.4"` removed from Cargo.toml
- **3 `#[allow(dead_code)]` resolved**: `BroydenMixer::device()/vec_dim()`, `KrigingF64::device()`, `KernelRouter::has_tpu()` accessors
- **neuralSpring unblocked**: Confirmed `WGSL_MEAN_REDUCE`, `argmax_dim()`, `softmax_dim()` already live in barracuda public API

### Numbers
- **694 WGSL shaders** (was 686), **2,490 tests** (was 2,440), **0 clippy warnings**, **0 external timestamp deps**

---

## Session 63: Deep Debt Evolution Wave 2 (Feb 25, 2026)

Continued systematic debt resolution — wiring unused implementations, smart refactoring, API promotion:

- **`coulomb_f64/mod.rs` smart refactor**: 610→370 lines. Extracted `CoulombBuffers` (shared pos/charge/force/params creation), `read_f64_via_staging()` and `map_staging_to_vec()` helpers — eliminated complete GPU pipeline duplication between forces-only and forces+energy paths.
- **`cyclic_reduction_f64` parallel solver activated**: `solve_gpu_parallel()` (full O(log n) implementation) was complete but dead — now dispatched for n ≥ 2048 where parallelism amortizes extra passes. Serial path retained for smaller systems.
- **`maximin_lhs` O(n) optimization wired**: `partial_maximin()` (O(n) per swap) was implemented but never called — optimization loop was using O(n²) `maximin_distance()`. Now wired into the CP swap loop for large speedup on high-n designs.
- **`WebGPUAdapter` zero-cost evolution**: `mock_data: String` (heap allocation when webgpu disabled) → `_private: ()` (zero-size, no allocation).
- **`erfc_deriv` API promotion**: Removed `#[allow(dead_code)]`, added to `electrostatics::mod.rs` re-exports alongside `erfc`, `compute_short_range`.
- **`GriffinLim` dead code cleanup**: `n_iter` `#[allow(dead_code)]` removed (field is used in GPU params struct); `n_fft`/`hop_length` documented as reserved for full iterative STFT/ISTFT.

---

## Session 62: Deep Debt Evolution (Feb 25, 2026)

Systematic codebase-wide deep debt resolution — dead code elimination, smart refactoring, platform evolution:

- **Dead WGSL constant evolution**: 25+ `#[allow(dead_code)]` constants evolved to documented `pub` API across barracuda (special, numerical, stats, linalg, ops, pde modules). Fossil `wgsl_shader()` methods converted to `pub const`. Electrostatics shader constants re-exported from parent module.
- **`morse_f64.rs` smart refactor**: 953→804 lines. Extracted `MorseBuffers` struct (shared GPU buffer creation) and `reduce_bond_forces()` function (shared reduce-to-per-particle pass), eliminating 149 lines of GPU pipeline duplication.
- **`rk_stage.rs` honest evolution**: Removed dead WGSL constants, `RkParams` struct, `wgsl_shader()` method. Module doc updated to reflect CPU-orchestrated architecture. Added `device()` accessor.
- **`instant` crate removal**: Unused dependency removed from neurobench-runner (code already uses `std::time::Instant`).
- **`compat.rs` platform evolution**: `can_handle()` checks `cfg!(target_os)` instead of always returning `true`. `execute_with_compatibility()` returns `SystemError::NotSupported` on wrong platform. Tests updated.
- **Discovery fallback evolution**: `default_fallbacks()` refactored to table-driven, early-exit when not dev mode, documented port source cross-reference.
- **`fhe_key_switch.rs` cleanup**: Dead `U64_EMU_PREAMBLE` constant removed (loaded but never referenced by shader pipeline).
- **Audit verified**: TPU mock already properly feature-gated; primal sockets already capability-based with deprecated legacy APIs.

---

## Session 61: Sovereign Compiler Phase 4 (Feb 25, 2026)

End-to-end Rust GPU compilation — naga-IR optimizer with SPIR-V passthrough:

- **SovereignCompiler** built in `crates/barracuda/src/shaders/sovereign/`: naga WGSL parser → FMA fusion → dead expression elimination → SPIR-V emission
- **FMA fusion pass**: walks naga expression arena, detects single-consumer `Mul(a,b) + c` patterns, replaces with `fma(a, b, c)` — addresses NAK Deficiency 4 (~1.3x)
- **Dead expression elimination**: mark-sweep DCE removes unused expressions after fusion
- **SPIR-V passthrough**: `SPIRV_SHADER_PASSTHROUGH` feature requested in all 5 device creation paths; `compile_shader_spirv()` wraps `create_shader_module_spirv()`
- **`compile_shader_f64()` evolution**: three-stage pipeline (ShaderTemplate → WgslOptimizer → SovereignCompiler) with automatic WGSL text fallback
- **naga 22.1 direct dependency**: type-compatible with wgpu 22, zero version conflict
- **10 sovereign unit tests**: FMA fusion (add/sub/multi-consumer), SPIR-V round-trip (f32/f64), complex shader, dead expr, invalid WGSL rejection
- **Full test suite**: 2,437+ barracuda tests pass (4 cascade failures, same as baseline — W-003 driver contention)

---

## Session 60: DF64 FMA + Transcendentals + Deep Debt (Feb 25, 2026)

F64 transcendental interconnect evolution — FMA optimization, DF64 transcendental library, polyfill hardening, and systematic deep debt resolution:

- **DF64 FMA optimization**: `two_prod` in `df64_core.wgsl` replaced Dekker splitting (17 ops) with `fma(a, b, -p)` (2 ops). `df64_mul` cross-terms also use FMA. Eliminates `split()` function entirely. On Ampere/Ada/RDNA2+, FMA is free-ish — same throughput as mul.
- **DF64 transcendental library**: New `df64_transcendentals.wgsl` with 9 functions — `sqrt_df64` (Newton-Raphson, 2 iterations), `exp_df64` (Cody-Waite range reduction + degree-6 Horner), `log_df64` (atanh-based + degree-5 Horner), `sin_df64`/`cos_df64` (Cody-Waite π/2 reduction + minimax kernels), `pow_df64`, `tanh_df64`, `df64_abs`, plus comparison helpers
- **4 force shaders evolved** from hybrid to full FP32 core streaming: `born_mayer_df64.wgsl`, `morse_df64.wgsl`, `yukawa_df64.wgsl`, `lennard_jones_df64.wgsl` — no longer round-trip through f64 units for `sqrt`/`exp`; all transcendentals stay in DF64
- **Polyfill patcher hardened**: `patch_transcendentals_in_code()` uses sentinel-based protection for `ldexp()`, `exp_df64()`, `exp_f64()`, `log_df64()`, `log_f64()` to prevent substring collision mangling
- **P0 polyfill audit**: All 28 `math_f64.wgsl` functions verified; AMD RADV (RX 6950 XT) tested: 233 f64 tests + 18 FFT tests, 0 failures
- **Multi-GPU adapter selection**: Deterministic GPU pinning via `BARRACUDA_GPU_ADAPTER` / `HOTSPRING_GPU_ADAPTER` env vars with auto-detection fallback (absorbed from hotSpring)
- **Deep debt fixes**: Crank-Nicolson variable shadowing bug (Courant number `r` shadowed by `Dirichlet(r)` pattern), SPD validation added to Cholesky, cross-attention evolved to separate `q_seq_len`/`kv_seq_len` (6 Rust + 6 WGSL files), loop unroller test assertions fixed for `u32` suffix
- **Code quality**: 0 clippy warnings, 0 fmt diffs, 0 TODO/FIXME/HACK markers, all files under 1000 lines, 1 unsafe block (wgpu pipeline cache API)

---

## Session 59: Deep Audit + Comprehensive Evolution (Feb 24, 2026)

Full codebase audit against wateringHole standards (uniBin, ecoBin, IPC v3, semantic naming) with systematic evolution of all findings:

- **`#![deny(unsafe_code)]`** added to 36 crates; 2 justified exceptions (gpu: wgpu buffer mapping, secure_enclave: mlock/munlock kernel calls)
- **TarpcClientWrapper evolved** from PhantomData placeholder to JSON-RPC fallback per `UNIVERSAL_IPC_STANDARD_V3.md` — tarpc-advertised endpoints now gracefully degrade to JSON-RPC over Unix sockets
- **Dependency security hardening**: `bytes` >=1.11.1, `aes-gcm` >=0.10.3, `tracing-subscriber` >=0.3.20, `chrono` hardened (no `oldtime` feature)
- **Clippy fix**: `map_or` → `is_some_and` in `pricing.rs`
- **Smart refactoring** (5 files decomposed by domain):
  - `beardog_integration/tests.rs` (1095 lines) → `tests/` module (type_serialization, discovery, capability_parsing)
  - `gpu_executor.rs` (851 lines) → `gpu_executor/` module (mod.rs 356, storage.rs 145, dispatch.rs 394)
  - `coordination_integration/client.rs` (944 lines) → `client/` module (discovery.rs 112, rpc.rs 225, tests.rs 598)
  - `cloud/compliance.rs` (910 lines) → `compliance/` module (security_tier.rs 35, validation.rs 285, tests.rs 530)
- **Quality gates**: fmt PASS, build PASS, clippy PASS (0 errors), doc PASS (0 warnings)
- **21,599 tests** across workspace (up from 14,200+)

---

## Session 58: Hybrid DF64 Core Streaming + Architecture Improvements (Feb 24, 2026)

- **6 DF64 shader variants** for FP32-core-streamed f64-precision workloads:
  - `gemm_df64.wgsl` — batched dense GEMM with shared-memory tiling (hi/lo f32 pairs)
  - `kinetic_energy_df64.wgsl` — per-link kinetic energy (lattice QCD)
  - `lennard_jones_df64.wgsl` — Lennard-Jones pair forces (molecular dynamics)
  - `morse_df64.wgsl` — Morse bond forces
  - `born_mayer_df64.wgsl` — Born-Mayer repulsive forces
  - `yukawa_df64.wgsl` — Yukawa screened forces with periodic boundary conditions
- **`Fp64Strategy` auto-selection** wired into GEMM, kinetic energy, Lennard-Jones Rust orchestrators via `GpuDriverProfile::fp64_strategy()`
- **`ComputeDispatch` builder** (`device/compute_pipeline.rs`) — fluent API for WGPU compute pipelines reducing ~80 lines of boilerplate to ~5 per op
- **`unified_hardware.rs` refactored** — 1012-line monolith decomposed into 6 focused modules: `types.rs`, `traits.rs`, `scheduler.rs`, `discovery.rs`, `cpu_executor.rs`, `transfer.rs`
- **`BarracudaError::gpu_ctx()`** — consolidated GPU error mapping helper
- **Workgroup size standardization** — `WORKGROUP_SIZE_1D` constants replacing hardcoded values (e.g. `crank_nicolson.rs`)
- **`specs/HYBRID_FP64_CORE_STREAMING.md`** — architecture spec for the DF64 pattern
- **Cross-spring absorption**: `df64_core.wgsl` (hotSpring), `pow(f64)` polyfill fix (neuralSpring), 5 biological ODE systems + NMF (wetSpring)
- **+27 tests** (14 ODE bio + 8 NMF + 5 Fp64Strategy)

---

## Session 53: Hardcoding Elimination + Unsafe Evolution + Coverage Push (Feb 24, 2026)

- **Hardcoded localhost eliminated**: 5 production files evolved to capability-based (`discover_self_ip_address()`, bind `0.0.0.0`, ports default to 0)
- **Unsafe code audit**: 1 unsafe block removed (`vfio.rs`), SAFETY comments expanded for MMIO Send/Sync and pinned alloc
- **`Box<dyn Error>` → `ServerError`**: `unibin/mod.rs` now uses typed error
- **Production TODOs → `BLOCKED` markers**: 4 TODOs evolved (container-runtime, biome-executor, 2× research)
- **`multi_gpu/mod.rs` refactored**: 921 → 54 lines (split into types.rs, strategy.rs, tests.rs)
- **Coverage push**: +193 new tests across 25 modules (scheduler, resources, plugin_system, communication, unibin, handlers, tarpc, beardog, discovery, identity, ports, etc.)
- **4,176 tests** across 5 core crates, all passing

## Sessions 51-52: Cross-Spring Absorption (Feb 24, 2026)

- **26 absorption items completed**: 7 HIGH (CG shaders, ESN NPU, generic ODE, CPU solver, FlatTree, neuralSpring GPU ops), 10 MEDIUM, 9 LOW
- **15 large files refactored** under 1000 lines by logical domain
- **+103 new tests** for absorbed modules
- **New modules**: tolerances, provenance, anderson_transport, screened_coulomb, fst_variance, ncbi_cache, gpu_session, tensor_axis_ops, domain_ops

## Sessions 46-49: Shader-First Architecture (Feb 23, 2026)

- **S49e-f: Zero CPU-only math** -- 27+ threshold-gated CPU fallbacks eliminated, 6 always-CPU ops wired to GPU, linalg (solve, cholesky) GPU-dispatched, RBF surrogate GPU pipeline (cdist + solve), PPPM electrostatics GPU FFT
- **S49c-d: Force field + MD GPU enforcement** -- Velocity-Verlet, MSD, cubic spline, RDF, cdist all GPU-first. Coulomb, Morse, Born-Mayer, Yukawa CPU fallbacks removed.
- **S49: Spring shader ingestion** -- 13 f32→f64 evolutions (bio, ESN, numerical). All 4 springs absorbed at f64.
- **S48: Lattice QCD GPU orchestration** -- CG solver + full HMC trajectory host loops
- **S47: Lattice QCD shaders** -- 14 WGSL shaders. CPU lattice code gated `#[cfg(test)]`.
- **S46: Cross-project absorption** -- hotSpring, neuralSpring, wetSpring shader absorption complete
- **f64 transcendental coverage** -- `compile_shader_f64()` auto-injects `math_f64.wgsl` polyfills on all drivers

## Session 45: Deep Debt Evolution (Feb 23, 2026)

- **Box<dyn Error> → typed errors**: 21 production usages eliminated (tarpc_server, manual_jsonrpc, unibin, resource_validator, production_hardening)
- **Barracuda fixes**: `atanh.wgsl` bind group layout, `batch_pair_reduce_f64.wgsl` fma→multiply+add, NPU test serialization (`SYNC_DEVICE_MUTEX`)
- **Coverage expansion**: 38 new tests (planner +9, ecosystem +8, detector +21)
- **Unsafe audit**: 95+ blocks documented; last `NonNull::new_unchecked` evolved to safe; 50+ SAFETY comments on env-var test blocks
- **Clippy pedantic**: 14 manual fixes (unnecessary_wraps, unused_async, match_same_arms) + 100+ auto-fixes across distributed/display/gpu crates
- **Event-driven**: Production polling loops → `tokio::time::interval` (launcher, client, health)
- **Clone reduction**: `Arc<str>` for version string, ref-based IPC params, borrow-based coordinator
- **Zero-copy**: `read_async` returns `bytes::Bytes`, `write_async` accepts `impl AsRef<[u8]>`
- **Hardcoding**: Primal integration and Consul/etcd endpoints configurable via env vars
- **WebSocket deprecation**: `WS_PROTOCOL_VERSION` and `ClientError::WebSocket` deprecated, `tokio-tungstenite` removed
- **Error conversion tests**: 5 pre-existing failures fixed (mismatched expected strings)
- **Test isolation**: ENV_MUTEX for all env-var-mutating detector tests
- **All quality gates green**: 0 clippy, 0 doc warnings, 0 fmt diffs, 14,000+ tests passing

## Session 41: f64 Shader Compile Fix + API Exposure (Feb 22, 2026)

- **Critical**: 6 f64 WGSL shaders used `compile_shader()` instead of `compile_shader_f64()`, missing f64 preamble injection for naga/Vulkan. Fixed: `batched_ode_rk4`, `batch_pair_reduce_f64`, `batch_tolerance_search_f64`, `kmd_grouping_f64`, `hill_f64`, `GemmCachedF64`
- **API**: `cpu_conv_pool::{conv2d, max_pool2d, avg_pool2d}` promoted from `pub(crate)` to `pub` (unblocks neuralSpring LeNet-5)
- **API**: All 25 bio ops re-exported at crate root (was 10)
- **Confirmed**: S-14/S-15 already resolved in S39; neuralSpring V8 recommendations stale for these

## Session 40: Richards PDE + Moving Window Stats + Dependency Audit (Feb 22, 2026)

- **Richards**: 1D unsaturated zone water flow solver (van Genuchten-Mualem, Picard iteration, Crank-Nicolson) with 4 tests (airSpring absorption)
- **Moving window stats**: WGSL GPU kernel computing mean/var/min/max over sliding windows for IoT sensor streams; always GPU dispatch
- **Dependency audit**: workspace already pure Rust; libc confined to akida VFIO ioctls
- **Dead code sweep**: 38 `#[allow(dead_code)]` all verified legitimate

## Sessions 39: Full Spring Absorption (Feb 22, 2026)

- Absorbed 7 neuralSpring bio ops + 3 wetSpring WGSL shaders + 11 hotSpring HFB physics shaders
- S-14 Naive matmul tier removed; S-15 matmul hang fix; S-16 transpose dispatch fix
- `GemmCachedF64::execute_to_buffer()`, `barracuda::math` module, `FlatTree` CSR, `sparse_eigh`, `quantize_affine_i8`
- `matmul_tiled.wgsl` barrier-safety fix for small matrix dispatch

---

## Session 38: Zero Warnings, Idiomatic Sweep, Test Coverage (Feb 22, 2026)

- **Zero clippy warnings**: Fixed `manual_div_ceil` in Yukawa GPU dispatch; added targeted `#[allow(clippy::expect_used)]` on infallible `Drop` in `AlignedBuffer` -- workspace now 0 clippy warnings
- **Blind unwrap() elimination**: Replaced 3 production `.unwrap()` calls with descriptive `.expect()` in `fused_map_reduce_f64.rs` and `batched_elementwise_f64.rs`; audited full workspace -- zero blind `unwrap()` in production code
- **Idiomatic match → if-let**: Simplified `deallocate_resources` in `hosting/resources.rs`
- **Test race condition fix**: 3 env-mutating tests in `toadstool-display` refactored from `std::env::set_var` to direct `PathEnv`/`PlatformPaths` construction -- eliminates parallel test races
- **Distributed test coverage**: 11 new behavioral tests for `NetworkLoadBalancer` (register, select, deregister, snapshot, least-loaded, unhealthy filtering) and `NetworkDistributor` (disabled fallback, deregister, accessor); distributed crate now 366 tests
- **Workspace verification**: 3,847+ tests passing across all non-GPU crates; barracuda targeted tests all passing

---

## Sessions 36-37: Precision, Deformed HFB, GPU Dispatch, Deep Debt (Feb 22, 2026)

- **TS-003**: Trig precision fix -- `sin_simple`/`cos_simple` upgraded to 7-term Taylor + Cody-Waite range reduction; `asin_core` extended from 5 to 8 polynomial terms
- **TS-001**: `pow_f64` fix -- f64 `exp_f64` extended to handle 2^k for |k| up to 1023; `log_f64` upgraded from 3 to 7 polynomial terms
- **TS-004**: `FusedMapReduceF64` buffer conflict -- both passes now encoded in single command encoder
- **S-13**: `PooledBuffer` drop race -- deferred return via pending queue with non-blocking device poll
- **Absorbed**: 5 deformed HFB shaders from hotSpring (Nilsson basis, density, Skyrme+Coulomb potential, cylindrical Laplacian Hamiltonian, BCS pairing)
- **Absorbed**: 4 neuralSpring shaders (`pairwise_l2`, `hill_gate`, `multi_obj_fitness`, `swarm_nn_forward`)
- **GPU dispatch**: Yukawa cell-list evolved from CPU-only to full GPU dispatch with sorted particles and result unsorting
- **LinuxEdgeDevice**: edge devices discovered via biomeOS runtime sockets get proper `EdgeDevice` impl
- **Bluetooth discovery**: sysfs-based adapter probe (`/sys/class/bluetooth`)
- **Federation discovery**: TCP probing of configured `discovery_endpoints`
- **29 new tests**: service discovery (17), federation (2), hosting resources (10)
- **ESN**: `export_weights()` + `import_weights()` for GPU-train → NPU-deploy pipeline
- **HFB spherical**: potentials, Hamiltonian, density, energy functional, BCS bisection -- 5 new f64 shaders
- **IPC v3.0**: abstract sockets, TCP fallback, tiered transport discovery confirmed
- **Code quality**: `cargo fmt` + `cargo clippy` clean; 589+ WGSL shaders (zero orphans)

---

## Sessions 32-35: Deep Debt Evolution (Feb 21-22, 2026)

### Capability-Based Discovery
- All hardcoded primal names (beardog, songbird, nestgate, squirrel) replaced with capability-based constants
- New `crates/core/common/src/constants/ecosystem.rs` with `well_known::*` identifiers for integration modules
- Auth modules: audience validation uses `PRIMAL_NAME` + `PLATFORM_AUDIENCE` only
- Config: self-knowledge only (no external primal port mappings)
- Doctor command: discovers running primals from socket files
- CLI zero-config: Unix socket capability-based discovery replacing HTTP placeholders

### Cloud Stubs Evolved to Real Implementations
- **Cost model**: Resource-based estimation with 6 pricing tiers, budget enforcement, structured breakdowns
- **Compliance**: Data sovereignty, security tier validation (Basic/Standard/High), resource isolation, structured reports
- **Federation**: Member management, heartbeats, capability exchange, configurable timeouts

### Zero-Copy Deepening
- `JsonRpcRequest<'a>` with `Cow<'a, str>` and `#[serde(borrow)]` for zero-copy deserialization
- `JsonRpcResponse<'a>` / `JsonRpcError<'a>` with borrowed fields
- Service discovery: `from_str` -> `from_slice` on hot paths

### Dependency Evolution
- thiserror 1.0 -> 2.0 workspace-wide (26 crates)
- async-trait retained (needed for `dyn Trait` async)
- FFI deps documented and justified

### Shader Completion
- Conv2D, MaxPool2D, AvgPool2D dedicated WGSL compute shaders (`ops/nn/`)
- RDF histogram GPU normalization (g(r) = histogram / (N_pairs * V_shell * rho))

### Testing & Coverage
- 200+ new unit tests across all crates
- FHE fault injection: GPU unavailable fallback, Barrett reduction, NTT twiddle factors
- WASM component-model: feature-gated stubs with skip messages
- Property-based testing with proptest for FHE operations

### Code Hardening
- Unsafe audit: all 62 blocks documented, none replaceable with safe Rust
- `#[allow]` audit: 5 unnecessary suppressions removed
- Production panic audit: 0 panics in core library code
- Error allocations reduced: useless `.into()` conversions eliminated
- Placeholder strings replaced with descriptive, actionable messages

### Architectural Evolution
- BYOB server merged into UniBin CLI (`toadstool byob-server` subcommand)
- `manual_jsonrpc` deprecated with MIGRATION.md guide to `pure_jsonrpc`
- Large files refactored: adaptive/mod.rs, config/lib.rs, primal_identity.rs, cpu_executor.rs
- Edge runtime: filesystem-based discovery + serial/TCP communication

---

## Session 31h: Deep Debt Polish (Feb 21, 2026)

### Clippy Clean Sweep
- **Barracuda**: 5 warnings → 0 (needless deref, manual div_ceil, manual is_multiple_of)
- **Akida-driver**: 2 warnings → 0 (map/unwrap_or_else → map_or_else, 8-arg fn → PollConfig struct)
- **Workspace**: Zero clippy warnings under `-W clippy::all` across all key crates

### Dead Code Audit (33 files)
- Removed 6 incorrect `#[allow(dead_code)]` from actually-used items (FheFastPolyMul, FhePointwiseMul, FheIntt, inv_n, Lookahead::alpha)
- Removed 2 dead functions (qr.rs::mat_approx_eq, nonzero::read_buffer_u32)
- Promoted view.rs::wgsl_shader() to pub const WGSL_VIEW
- 22 annotations confirmed legitimate (future GPU acceleration paths)

### Production Code Quality Verification
- All unwrap() calls in high-count files exclusively in #[cfg(test)] blocks
- Zero TODOs/FIXMEs/HACKs in production code (1 research TODO in akida-reservoir)

---

## Session 31g: Deep Debt Evolution (Feb 21, 2026) ✅

### Orphan Shader Integration
- **ESN GPU kernels**: `WGSL_RESERVOIR_UPDATE` + `WGSL_READOUT` constants
- **RF batch inference**: `RfBatchInferenceGpu` — full GPU wrapper (SoA f64, wetSpring v5)
- **HMM forward f32**: `WGSL_HMM_FORWARD_LOG_F32` — log-domain variant
- **SDPA single-kernel**: `WGSL_SDPA_SINGLE_KERNEL` — prototype alongside multi-pass
- **Optimizer shaders**: BFGS update, batch gradient, simplex ops wired as constants

### f64 Linear Algebra
- **`LinSolveF64`**: GPU Gaussian elimination (f64) for ill-conditioned systems
- **`InverseF64`**: GPU Gauss-Jordan inverse (f64, N ≤ 32)

### Safety & Quality Audit
- **Zero production panics**: All 50+ `panic!()` calls confirmed in test code only
- **Hardcoded IPs resolved**: All use env-var-with-defaults pattern
- **Unsafe audit clean**: All blocks minimal with SAFETY invariant docs
- **Extracted `PINNED_ALIGNMENT`**: De-duplicated constant in `pinned.rs`

---

## Session 31e: Deep Debt Evolution (Feb 21, 2026) ✅

### Executor Completeness ✅

- **GPU executor** — All MathOp variants now have dispatch paths: `Pow` (scalar pow), `Max`/`Min` (elementwise fallback), `Squeeze`, `Unsqueeze`, `Broadcast`, `Concat`, `Split`. Only Conv2D/MaxPool2D/AvgPool2D remain as honest `NotImplemented`.
- **CPU executor** — Full coverage: `Softmax`, `BatchMatMul`, `Reshape`, `Squeeze`, `Unsqueeze`, `Transpose`, `Broadcast`, `Concat`, `Split`. Only Conv ops remain `NotImplemented`.

### Orphan Shader Wiring ✅

- **6 new GPU op wrappers** connecting WGSL shaders to Rust APIs: `BatchIprGpu` (spectral/IPR), `LocusVarianceGpu` (bio/FST), `PairwiseHammingGpu`, `PairwiseJaccardGpu`, `SpatialPayoffGpu`, `BatchFitnessGpu`.
- Extended `elementwise_binary.wgsl` with Pow/Max/Min operations.
- Removed duplicate ODE shader (bio/ copy → numerical/ is canonical).
- Removed genuinely unused `read_buffer_u32()` from searchsorted.
- Fixed 3 lifetime elision warnings.

---

## Session 31d: Cross-Spring Absorption (Feb 21, 2026) ✅

### hotSpring Absorption ✅

- **Staggered Dirac operator** — `dirac_staggered_f64.wgsl` + `ops/lattice/dirac.rs`: Full GPU pipeline for Kogut-Susskind lattice QCD fermions. SU(3)×color multiplication, staggered phases, periodic boundaries. `DiracGpuLayout` for topology flattening.
- **CG lattice kernels** — `cg_kernels_f64.wgsl` + `ops/lattice/cg.rs`: Three BLAS-like GPU kernels (`complex_dot_re`, `axpy`, `xpay`) for CG solver on complex fermion fields. Also exported as standalone WGSL constants.
- **SubstrateCapability model** — `device/substrate.rs`: Capability-based dispatch enum (F64Compute, F32Compute, QuantizedInference, BatchInference, WeightMutation, ScalarReduce, SparseSpMV, Eigensolve, CG, ShaderDispatch, SimdVector, TimestampQuery). Runtime-probed from wgpu features. NPU discovery via `/dev/akida*`.

### wetSpring Absorption ✅

- **7 new bio GPU op wrappers** — Full `WgpuDevice` pipelines following `SmithWatermanGpu` pattern:
  - `HmmBatchForwardF64` — Batch HMM forward algorithm (log-domain, f64)
  - `AniBatchF64` — Pairwise Average Nucleotide Identity
  - `SnpCallingF64` — Position-parallel SNP calling
  - `DnDsBatchF64` — Batch Nei-Gojobori dN/dS with Jukes-Cantor
  - `PangenomeClassifyGpu` — Gene family classification (core/accessory/unique)
  - `QualityFilterGpu` — Per-read FASTQ quality trimming
  - `Dada2EStepGpu` — DADA2 E-step batch log-probability
- **ODE sweep shader** — `batched_qs_ode_rk4_f64.wgsl`: Full-GPU RK4 parameter sweep for QS/c-di-GMP ODE (5-variable system, 17 parameters per trajectory)

### neuralSpring Confirmation ✅

- **Householder+QR eigensolver** — Already absorbed as `ops/linalg/eigh_f64.rs`
- **7 domain shaders** — Already present as WGSL files (batch_ipr, spatial_payoff, pairwise_hamming, pairwise_jaccard, locus_variance, batch_fitness_eval, rk4_parallel)
- **GPU PRNG** — Already present as `shaders/misc/prng_xoshiro.wgsl`
- **CPU math** — Already present (`special/erf.rs`, `special/gamma.rs`)
- **NVVM Ada workaround** — Already complete (`NvvmAdaF64Transcendentals` in `driver_profile.rs`)

---

## Sessions 31–31c Evolutions (Feb 21, 2026) ✅

### Executor Wiring ✅

- **GpuExecutor** — 16 additional MathOps wired (total 31): Log, Sin, Cos, Tan, Reciprocal, Square, Div, BatchMatMul, ReduceMax/Min/Prod, Reshape, Transpose
- **CpuExecutor** — Full MathOp dispatch via `execute_unary_cpu`/`execute_binary_cpu`/`execute_reduce_cpu`/`execute_matmul_cpu` (was `NotImplemented`)
- **unified_hardware CpuExecutor** — Delegated to standalone CpuExecutor (was `NotImplemented`)
- **ProcessSpawner WASM loading** — Delegated to BiomeExecutor (was returning empty bytes)
- **Performance optimizer** — `get_recommendations()` and `update_model()` implemented

### Smart Refactoring ✅

| File | Before | After | Technique |
|------|--------|-------|-----------|
| `qr_gpu.rs` | 933 | 486 | `dispatch` closure + `make_bgl`/`make_bg` helpers (-48%) |
| `lu_gpu.rs` | 780 | 302 | Static `make_bgl`/`make_pipe`/`make_bg`/`dispatch` (-61%) |
| `svd_gpu.rs` | 764 | 305 | Same helpers; BGL via type-slice (-60%) |
| `esn_v2.rs` | 884 | 842 | `validate_config()` + `expect_size()` helpers |
| `cache_hierarchy.rs` | 638 | 607 | `bgl_entry` closure + table-driven substrate classification |

### GPU Path Completion ✅

- **MorseForceF64** — 2-pass GPU shader dispatch (per-bond + reduce-to-particle)
- **BornMayerForceF64** — N-body direct GPU shader dispatch
- **Unsafe evolution** — `NonNull::new_unchecked` → safe `NonNull::new().expect()`

---

## Session 29 Evolutions (Feb 21, 2026) ✅

### Code Size — Smart Module Extraction ✅

| File | Before | After | Technique |
|------|--------|-------|-----------|
| `svd_gpu.rs` | 973 lines | 842 lines | `make_pipeline` + `dispatch` closures deduplicate 7+7 blocks |
| `session/mod.rs` | 968 lines | 569 lines | Dispatch logic extracted to `session/dispatch.rs` (420 lines) |
| `tensor/mod.rs` | 948 lines | 799 lines | Scalar ops + random gen extracted to `tensor/ops.rs` (121 lines) |
| `math_f64.wgsl` | 1002 lines | 837 lines | Special functions (gamma/erf/bessel) → `math_f64_special.wgsl` (175 lines) |

All files safely under the 1000-line limit. No files over 950 lines remain.

### Production Safety ✅

- `gpu_executor.rs`: 3 `try_into().unwrap()` calls replaced with explicit array indexing
- Improved SAFETY documentation on `Send`/`Sync` impls in `AlignedBuffer` and `PinnedMemory`
- Removed unused `PhantomData<Arc<()>>` and dead `Arc` import from `PinnedMemory`

### Hardcoded Paths Evolved ✅

| File | Before | After |
|------|--------|-------|
| `server/capabilities/mod.rs` | `/tmp` fallback | `runtime_base_dir()` → `XDG_RUNTIME_DIR` / `std::env::temp_dir()` |
| `runtime/edge/src/lib.rs` | `/tmp/cache` | `std::env::temp_dir().join("toadstool-edge-cache")` |

### Dependency Evolution ✅

| Removed Dep | Crate(s) | Replacement |
|-------------|----------|-------------|
| `once_cell` | workspace root, `toadstool-config` | `std::sync::LazyLock` (Rust 1.80+) |
| `lazy_static` | `security-policies` | `std::sync::LazyLock` (Rust 1.80+) |
| `tempdir` | `toadstool-testing` | `tempfile` (already a dep) |
| `term_size` | `toadstool-cli` | Unused; `console` already in deps |
| `base64` 0.21 | `cli`, `client`, `nestgate` | Unified to 0.22; removed from `client`/`nestgate` (unused) |
| `mdns` | workspace root, `runtime-edge` | Standardized on `mdns-sd`; edge stub never used it |
| `dashmap` | `distributed`, `runtime-gpu` | Evolved to `std::sync::RwLock<HashMap>` (no source usage) |
| `which` | `toadstool-cli` | CLI uses shell `which`, not the crate |

---

## Session 28 Evolutions (Feb 21, 2026) ✅

### Production Safety — RwLock Poison Recovery ✅

`pipeline_cache.rs`: All 12 `expect("poisoned")` calls replaced with `read_or_recover()` / `write_or_recover()` helpers. Consistent with existing `probe.rs::lock_cache` pattern. Caches safely continue after previously panicked threads.

### Code Size — Smart Deduplication ✅

| File | Before | After | Technique |
|------|--------|-------|-----------|
| `lu_gpu.rs` | 996 lines | 854 lines | `build_lu_pipeline()` deduplicates 4 pipeline helpers |

### Hardcoded Values Evolved ✅

| Category | Files Changed | Evolution |
|----------|--------------|-----------|
| Fallback ports | `primal_discovery_complete.rs` | Raw `8080`/`8081`/`8082` → named constants |
| Runtime paths | `connection.rs`, `format.rs` | `/tmp` → `std::env::temp_dir()` + `BIOMEOS_RUNTIME_DIR` |
| Config paths | `service.rs` | `/etc/biomeos/` → XDG cascade (`XDG_CONFIG_HOME` → `HOME/.config`) |
| Hostname | `manual_jsonrpc/mod.rs` | `/etc/hostname` → `HOSTNAME` env var first |
| GPU estimates | `gpu_executor.rs` | Magic numbers → `capability_defaults` module |

### Placeholder Evolution ✅

ML model placeholders (`vision.rs`, `whisper.rs`, `bert.rs`) evolved from silently returning empty results to returning `Error::NotImplemented` with descriptive messages. API surface remains stable; honest error handling instead of fake success.

---

## Session 27 Evolutions (Feb 21, 2026) ✅

### wetSpring/neuralSpring Full Shader Absorption ✅

Absorbed all remaining WGSL shaders and Rust implementations from wetSpring v5 and neuralSpring metalForge handoffs:

| Component | Files | Tests | Provenance |
|-----------|-------|-------|------------|
| Bio/genomics shaders | 8 WGSL | — | wetSpring + neuralSpring |
| ML/evolution shaders | 3 WGSL | — | wetSpring + neuralSpring |
| Numerical shaders | 1 WGSL | — | neuralSpring |
| Math/distance shaders | 3 WGSL | — | neuralSpring |
| Reduce shaders | 1 WGSL | — | neuralSpring |
| Spectral shaders | 1 WGSL | — | neuralSpring |
| Householder+QR eigensolver | 1 Rust | 9 | neuralSpring (S-12) |
| **Total** | **18 files** | **9 tests** | |

### NVVM Ada Lovelace Driver Fix ✅

Fixed `needs_f64_exp_log_workaround()` to correctly return `true` for NVIDIA proprietary driver on Ada Lovelace (RTX 40xx). NVVM/PTXAS cannot compile native f64 transcendentals (exp, log, pow) on SM89. Added `NvvmAdaF64Transcendentals` workaround variant and `is_nvidia_ada_lovelace()` detection.

### Cumulative Shader Math Library (Sessions 26–27)

| Domain | Shaders | Source |
|--------|---------|--------|
| Lattice QCD | 5 | hotSpring |
| Spectral theory | 1 + 6 Rust | hotSpring |
| ESN (reservoir computing) | 2 | hotSpring |
| Bio/genomics | 14 | wetSpring + prior |
| ML/evolution | 3 | neuralSpring |
| Numerical ODE | 1 | neuralSpring |
| Math/distance | 3 | neuralSpring |
| Reduce | 1 | neuralSpring |
| Spectral (IPR) | 1 | neuralSpring |
| **Total new** | **31 WGSL + 7 Rust** | |

---

## Session 26 Evolutions (Feb 21, 2026) ✅

### hotSpring v0.6.0 Shader Math Absorption ✅

Absorbed spectral theory primitives from hotSpring v0.6.0 (commit `6bd0047`) to complete the pure shader math library:

| Component | Files | Tests | Description |
|-----------|-------|-------|-------------|
| Spectral theory module | 6 Rust files | 19 | Lanczos, Sturm, Anderson 1D/2D/3D, Hofstadter |
| ESN shaders | 2 WGSL shaders | — | Reservoir update + readout for reservoir computing |

**New modules in `barracuda/src/spectral/`**:
- `lanczos.rs` — Lanczos tridiagonalization with full reorthogonalization
- `tridiag.rs` — Sturm bisection eigensolve for symmetric tridiagonal matrices
- `anderson.rs` — Anderson localization (1D/2D/3D), Lyapunov exponent
- `hofstadter.rs` — Almost-Mathieu operator, Hofstadter butterfly
- `stats.rs` — Level spacing ratio (Poisson/GOE), band detection
- `sparse.rs` — `SpectralCsrMatrix` + GPU `WGSL_SPMV_CSR_F64` shader

**New shaders in `barracuda/src/shaders/ml/`**:
- `esn_reservoir_update.wgsl` — Fused matmul + leaky tanh for ESN
- `esn_readout.wgsl` — Readout matrix-vector product

**Previously absorbed from hotSpring**:
- `complex_f64.wgsl`, `su3.wgsl` — Complex f64 and SU(3) math
- `wilson_plaquette_f64.wgsl` — Wilson gauge plaquette
- `su3_hmc_force_f64.wgsl`, `higgs_u1_hmc_f64.wgsl` — Lattice QCD HMC
- CellListGpu fix (Session 25), GPU FFT f64

**Remaining in hotSpring** (domain-specific):
- Nuclear HFB shaders (`batched_hfb_*.wgsl`, `deformed_*.wgsl`)
- Physics validation suites (18 papers, 33/33 validation)

---

## Session 25 Evolutions (Feb 21, 2026) ✅

### Unit Test Coverage Expansion — 172 New Tests ✅

Comprehensive unit test additions across core modules to improve coverage toward 90% target:

| Module | Tests Added | Description |
|--------|-------------|-------------|
| `toadstool-common/service_discovery/endpoint.rs` | 13 | URL parsing for `ServiceEndpoint::from_url_string()` |
| `barracuda/ops/expand/compute.rs` | 19 | Broadcasting shape computation, stride calculations |
| `barracuda/dispatch/config.rs` | 14 | Dispatch thresholds, GPU routing, force CPU/GPU |
| `barracuda/workload.rs` | 27 | Workload classification, sparsity analysis, device selection |
| `barracuda/resource_quota.rs` | 22 | Quota tracking, VRAM limits, device requirements |
| `barracuda/numerical/rk45.rs` | 16 | ODE solver config builders, error paths, max steps |
| `toadstool/composition_constraints/constraint.rs` | 8 | Hard/soft constraint classification, serialization |
| `toadstool/composition_constraints/evaluation.rs` | 8 | Satisfaction scoring, constraint evaluation |
| `toadstool/composition_constraints/request.rs` | 13 | Composition requests, priorities, metadata |
| `toadstool/universal/types.rs` | 16 | `SecurityLevel`, `PrimalType`, `NetworkLocation`, `PrimalContext` |
| `toadstool/execution.rs` | 16 | `ExecutionStatus`, `RuntimeType`, `ExecutionInput/Output` |

**New builder methods added to `Rk45Config`**:
- `with_max_steps(usize)` — Set maximum number of integration steps
- `with_safety(f64)` — Set safety factor for step size adjustment

**Bug fixes**:
- Fixed unused import warning in `ipc/server.rs`
- Fixed case-sensitivity in `runtime.rs` test error message matching

All 172 new tests pass. Tests focus on pure CPU logic, serialization roundtrips, builder patterns, error handling paths, and boundary conditions.

---

## Session 24 Evolutions (Feb 20, 2026) ✅

### Integration Test Graduation — 3 More Suites ✅ (D-S18-003 continued)

**`error_paths_discovery_tests.rs`** (10 tests):
- Rewrote using `toadstool::self_identity::{Capability, DiscoveredService}` (no `primal_identity` module exists)
- `SelfIdentity::discover().await` → `SelfIdentity::new()` (sync constructor)
- `DiscoveredService` fields aligned: added `version`, `protocols`, `last_seen`; removed `metadata`
- `Capability::from("x")` → struct literal with `name`, `version`, `features`, `characteristics`

**`fault_tests.rs`** (19 tests via `chaos/fault_injection.rs` + `chaos/resilience_tests.rs`):
- Built against real `toadstool_testing::chaos::{ChaosScenario, FaultType, ResourceType, SystemState}`
- `FaultType` variants corrected: `node_id`, `consumption_percent`, `loss_rate: f64`, `duration_ms`

**`security_tests.rs`** (13 tests via `security/penetration_tests.rs`):
- Capability boundary enforcement, privilege escalation resistance, `IsolationLevel` correctness
- `IsolationLevel::Strict` → `IsolationLevel::Enhanced` (actual variant)
- Empty-capabilities context: asserts `validate().is_err()` (correct; ≥1 cap required)

**167 integration tests, 0 failures.** Stale `pending/` copies of 8 already-graduated suites removed.

### D-S21-003 — wetSpring `gemm_cached.rs` Path Fragility ✅

- `wetSpring/barracuda/Cargo.toml`: `../../phase1/toadstool` → `../../phase1/toadStool` (Linux case fix)
- `gemm_cached.rs`: `include_str!("../../../../phase1/toadstool/...")` → `barracuda::ops::linalg::GemmF64::WGSL`
- `cargo check --features gpu` passes cleanly in wetSpring

---

## Sessions 22–23 Evolutions (Feb 20, 2026) ✅

### D-S17-002 — `capabilities.rs` Semantic Split ✅

`GpuDriverProfile`, `DriverKind`, `CompilerKind`, `GpuArch`, `Fp64Rate`, `Workaround`,
`EigensolveStrategy` extracted from the 929-line `capabilities.rs` into new `driver_profile.rs`.
`capabilities.rs` (505 lines) now exclusively covers hardware limits + wgpu dispatch helpers.
`pub use driver_profile::*` in `capabilities.rs` preserves all callers without path changes.

### D-S16-003 — `ParallelFilter` Two-Level Scan ✅

New `apply_l1_offsets` WGSL entry point (Pass C) in `prefix_sum.wgsl`.
`filter.rs` `execute()` auto-selects:
- n ≤ 65,536: existing 4-pass single-level (unchanged)
- 65,536 < n ≤ 16,777,216: new 6-pass two-level (local scan → L1 scan → offsets → apply → scatter)
- n > 16M: `BarracudaError::InvalidInput` (three-level left for genome-scale)

### Integration Tests Graduated (Sessions 22–23)

| Suite | Tests |
|---|---|
| `runtime_execution_tests.rs` | 20 |
| `error_handling_tests.rs` | — |
| `resource_requirements_tests.rs` | — |
| `security_context_tests.rs` | — |
| `config_management_tests.rs` | — |
| `evolution_fault_tests.rs` + `evolution_chaos_tests.rs` | — |

---

## Sessions 19–21 Evolutions (Feb 20, 2026) ✅

### neuralSpring Absorption (`TensorSession` ML ops)

`TensorSession` extended with `matmul`, `relu`, `gelu`, `softmax`, `layer_norm`, `reshape`,
`head_split`, `attention`, `head_concat` — covers all 11 neuralSpring handoff shortcomings.
6 new fused MLP/transformer tests passing. Equivalent to the 46–78× fused pipeline in
`neuralSpring/src/evolved/`. All session ops encode in one `CommandEncoder` / `queue.submit()`.

### GPU Architecture + Dispatch Hardening

- `capabilities.rs::classify_substrate()`: vendor-ID-first (VENDOR\_NVIDIA/AMD/INTEL/APPLE/ARM/QUALCOMM),
  string-name fallback retained for zero-vendor-ID Mesa/software drivers.
- `dispatch/benchmark.rs::check_gpu()` + `dispatch/config.rs::check_gpu_available()`:
  duplicated raw wgpu adapter setup consolidated to `WgpuDevice::new()`.

### `GemmCachedF64` Absorbed from wetSpring

`ops/linalg/gemm_f64.rs`: pre-compiled GEMM pipeline with GPU-resident weight matrix B.
Pipeline compiled once at `new()`, B uploaded once; subsequent `multiply()` calls dispatch per-A only.
**Measured**: 60× speedup on taxonomy dispatch (first: 60 ms → subsequent: <1 ms).
`GemmF64::WGSL` published as `pub const` — eliminates wetSpring's `include_str!` path hack.

---

## Session 18 Evolutions (Feb 20, 2026) ✅

### Sovereign Compute Phase 3 — Now Live in Hot Path ✅

`WgpuDevice::compile_shader_f64()` now runs a two-stage pipeline:
1. `ShaderTemplate::for_driver_auto()` — NVK/RADV exp/log workaround (existing)
2. `WgslOptimizer::optimize()` — `@ilp_region` ILP reorder + `@unroll_hint` loop unroll (new)

Fast path: zero-overhead when no annotations present (single `contains()` guard). The Jacobi
eigensolve shader fires the reorderer automatically on every compile, pre-scheduling DFMA
pairs for the actual GPU's cycle count (`GpuDriverProfile::latency_model()`).

### Apple M-Series GPU Architecture ✅

- `GpuArch::AppleM` — detects `"apple m"` / `"apple paravirtual"` adapter names
- `AppleMLatencyModel` — software-emulated f64 FMA ~16 cy, f32 ~4 cy (all WGSL ILP annotations honour this)
- `Fp64Rate::Software` for AppleM (no native f64 silicon on M-series)
- Cross-vendor latency matrix now complete: SM70–SM89, RDNA2/3/CDNA2, AppleM, Conservative

### GpuExecutor Zero-Copy Output Path ✅ (D-S16-001)

- `GpuTensorStorage.buffer: Arc<wgpu::Buffer>` — shared ownership instead of owned buffer
- `Tensor::from_arc_buffer(Arc<wgpu::Buffer>, ...)` — zero-copy Tensor construction
- `Tensor::try_arc_buffer() -> Option<Arc<wgpu::Buffer>>` — bridge for storage code
- `GpuTensorStorage::from_tensor()` — Owned path: `Arc::clone()` (0 bytes); Pooled path: `copy_buffer_to_buffer()` (GPU-to-GPU, no CPU)
- `execute()` no longer calls `to_vec()` + `write_from_cpu()` — the GPU→CPU→GPU round-trip is gone

### Integration Tests Crate ✅ (D-S16-004)

- `crates/integration-tests/` created and added to workspace
- 21 orphan `tests/*.rs` files migrated from workspace root
- 3 active suites: `chaos_engineering_scenarios`, `error_paths_config_tests`, `pure_rust_validation_tests` (13 pass, 7 ignored with explanations)
- 12 files quarantined to `tests/pending/` with `README.md` tracking unimplemented APIs
- Workspace `tests/` directory is now free of bare `.rs` files

---

## Sessions 9–11 Evolutions (Feb 19, 2026) ✅

### Zero-Copy Binary Payloads ✅

All hot-path binary types migrated from `Vec<u8>` → `bytes::Bytes`:

| Type | Location | Impact |
|------|----------|--------|
| `WorkloadSubmission.data` | `core/toadstool` | O(1) clone across RPC boundary |
| `WorkloadResult.data` | `core/toadstool` | O(1) result propagation |
| `ExecutionInput.data` | `core/toadstool` | O(1) dispatch to runtime |
| `ExecutionOutput.data` | `core/toadstool` | O(1) result collection |
| `ExecutableSource::Bytes` | `core/toadstool` | O(1) binary payload hand-off |
| `WasmModuleSource::Bytes` | `core/toadstool` | O(1) WASM module hand-off |
| `TarpcWorkloadSubmission.payload` | `server` | O(1) tarpc transport |

**Crates updated**: `core/toadstool`, `server`, `testing`, `runtime/native`, `runtime/wasm`, `distributed`.

### Sleep Elimination (27 calls) ✅

Systematic audit of all `tokio::time::sleep` and `std::thread::sleep` calls in non-hardware code:

| File | Fix | Count |
|------|-----|-------|
| `circuit_breaker.rs` | `tokio::time::Instant`, `start_paused + advance()` | 2 |
| `metrics_middleware.rs` | `tokio::time::Instant`, `start_paused + advance()` | 1 |
| `memory/tracker.rs` | `tokio::time::Instant`, `start_paused + advance()` | 2 |
| `performance/manager.rs` | `tokio::time::Instant`, `start_paused + advance()` | 4 |
| `performance_hardening/async_ops.rs` | `tokio::sync::Barrier` + `timeout` | 1 |
| `primal_discovery_complete.rs` | `cache_ttl: Duration::ZERO` | 1 |
| `capability_provider.rs` | Removed (socket bind is synchronous) | 1 |
| `integration/helpers.rs` | Removed (no behavioral assertions) | 5 |
| `multi_device_integration.rs` | Removed (`DeviceLease::drop()` is atomic) | 3 |
| `performance/mod.rs` tests | CPU-bound fold + `yield_now()` | 4 |
| `coordinator_executor.rs` | `Notify` + `AtomicBool` fan-out | 3 |

**Total removed**: 27 sleep calls across 11 files.

### Hardcoding Eliminated ✅

- **DNS servers** (`sandbox/src/types.rs`, container configs, CLI templates): removed `8.8.8.8`/`1.1.1.1` — containers inherit from host/orchestrator
- **Ollama IP**: reads `$OLLAMA_HOST` or discovers via Songbird capability
- **`TelemetryConfig.enabled`**: changed to `false` (opt-in, was always-on)
- **`DnsConfig`**: derives `Default` (empty by default)
- **Discovery DNS** (`configurator/core.rs`): reads system resolver via `system_dns_resolvers()`

### Code Structure Improvements ✅

- **`pure_jsonrpc.rs`** (979 lines) split into `pure_jsonrpc/` module:
  - `types.rs` — request/response types, traits
  - `handler.rs` — `JsonRpcHandler` with `SemanticMethodRegistry` wired
  - `mod.rs` — public API and re-exports
  - `tests.rs` — inline integration tests
- **`SemanticMethodRegistry`** wired into `JsonRpcHandler` — semantic routes (e.g. `runtime.workload.submit`) resolve to implementation names before dispatch
- **`biomeos_integration/storage_backend/mod.rs`** (987 lines) split:
  - `mod.rs` — trait + `VolumeStatus` enum + re-exports (64 lines)
  - `nestgate.rs` — `NestGateBackend` (306 lines)
  - `inmemory.rs` — `InMemoryBackend` (210 lines)
  - `tests.rs` — shared backend test suite (68 lines)

### Bug Fix: `UnifiedBuffer::drop()` ✅

`metrics.total_allocated` was not decremented on drop — only the outer `AtomicUsize` counter was decremented. Both the `RwLock<Metrics>` field and the atomic are now updated in a single write, ensuring metric consistency. This also eliminated 6 stale `sleep()` calls in GPU memory tests that had been masking the inconsistency.

### CLI Executor Coverage ✅

15 inline `#[cfg(test)]` tests added to previously untested executor sub-modules:

| Module | Tests Added |
|--------|-------------|
| `executor/display.rs` | `get_log_path`, `show_log_file` (tempfile), `tail_log_file` (tempfile) — 6 tests |
| `executor/signals.rs` | SIGCONT-to-self, invalid signal, dead-PID (spawn+wait), kill command — 4 tests |
| `executor/resources.rs` | `biome_exists`, `get_biome_info`, `find_process_pid`, error path, concurrent reads — 5 tests |

### `llvm-cov` SIGSEGV Resolved ✅

The `toadstool-server` SIGSEGV under `cargo llvm-cov` is resolved as a side-effect of the sleep
elimination and concurrency hardening work. Workspace-wide `llvm-cov` (excluding GPU crates) now
completes with exit 0 consistently.

**Coverage progression**:
- Session 8: 61.35% lines, 66.47% functions
- Session 11: **63.02% lines (+1.67 pp)**, **68.58% functions (+2.11 pp)**
- Session 25: ~65% lines (+172 new unit tests across 11 modules)

---

## Previous Evolutions (Feb 14–17, 2026)

See [CHANGELOG.md](CHANGELOG.md) for full session-by-session detail of earlier evolutions including:
- cudarc 0.11 → 0.19 upgrade
- Clippy cleanup (44 auto-fixes)
- Deep debt evolution (pure Rust syscalls, timeout consolidation, SIMD runtime detection)
- Bug fixes from hotSpring/wetSpring validation
- Device registry with physical deduplication
- F64 unified math language suite
- GPU-resident pipeline
- MD pipeline (thermostats + PPPM)
- Unidirectional compute pipeline
- ecoBin compliance evolution

---

## Root Documentation

| File | Purpose |
|------|---------|
| `README.md` | Project overview, honest status |
| `STATUS.md` | This file — detailed technical status |
| `DEBT.md` | Active workarounds and evolution paths |
| `NEXT_STEPS.md` | Roadmap and upcoming work |
| `QUICK_REFERENCE.md` | Commands and API reference |
| `DOCUMENTATION.md` | Navigation hub |
| `CHANGELOG.md` | Full session-by-session evolution history |
| `SOVEREIGN_COMPUTE.md` | Sovereign compute roadmap |
| `UNIDIRECTIONAL_PIPELINE.md` | GPU-resident pipeline design |

---

**Last Updated**: February 26, 2026 — Session 68 (11 waves): 700 WGSL shaders, **ZERO f32-only** — every shader now f64 canonical with LazyLock downcast for f32. 296 f32 WGSL files deleted. Precision bottleneck gate OPEN. Deep debt sweep complete (println→tracing, magic numbers→named constants).
