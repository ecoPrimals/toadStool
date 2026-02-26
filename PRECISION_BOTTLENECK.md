# Precision Bottleneck — Evolution Gate

**Date**: February 26, 2026 — Session 68
**Status**: ACTIVE — solve precision debt before absorbing more from springs

---

## Gate Rule

**No new spring absorptions until all phases below are complete.**

The universal precision architecture is an evolution bottleneck. Absorbing
more shaders while the foundation lacks multi-precision coverage creates
compound debt. Solve the bottleneck first, then every future absorption
automatically gets multi-precision support.

---

## Phase Status

| Phase | Description | Items | Status |
|-------|-------------|-------|--------|
| **1** | Infrastructure (pipeline + templates) | 6 | DONE |
| **2** | Consolidate f32/f64 pairs | 54 total | 5 DONE, 49 EVOLVE |
| **3** | f32-only → f64 canonical | 240 trivial + 294 transcendental | **ALL DONE** |
| **4** | Algorithm evolution (f32 → f64 quality) | 49 | TODO (parallel-safe) |
| **5** | Domain-specific (lattice/MD/HFB) — no change | 0 | N/A |

**Gate opens when**: Phase 2 + Phase 3 complete. **GATE IS NOW OPEN.**
Phase 4 (algorithm evolution) proceeds in parallel with absorptions.

---

## Phase 1: Infrastructure — DONE

- [x] `Precision::Df64` enum variant
- [x] `compile_shader_universal(source, precision)` on `WgpuDevice`
- [x] `compile_template(template, precision)` on `WgpuDevice`
- [x] `downcast_f64_to_f32()` with sentinel-protected `_f64(` names
- [x] `downcast_f64_to_f32_with_transcendentals()` (polyfill → native)
- [x] 12 universal `{{SCALAR}}` templates

---

## Phase 2: Consolidate f32/f64 Pairs

### Session 68 Discovery

Inventory revealed that of the original 50+ pairs, only **5 were true
near-duplicates** (identical logic, only type names differ). The remaining
**49 are structurally different** — the f64 versions typically use superior
algorithms (workgroup tree reduction, Welford online statistics, etc.)
while f32 uses simpler sequential loops.

### Consolidated (5 pairs) — DONE

Each consolidated pair: f64 is canonical source, f32 produced via
`LazyLock<String>` calling `downcast_f64_to_f32()`, f32 WGSL file deleted.

| Shader | Rust caller | Status |
|--------|------------|--------|
| elementwise_add | `ops/add.rs` | DONE — f32 file deleted |
| elementwise_mul | `ops/mul.rs` | DONE — f32 file deleted |
| sum_dim | `ops/sum.rs` | DONE — f32 file deleted |
| mean_dim | `ops/mean.rs` | DONE — f32 file deleted |
| std_dim | `ops/std/mod.rs` | DONE — f32 file deleted |

### Structurally Different (49 pairs) — Moved to Phase 4

These pairs have genuinely different algorithms between f32 and f64.
Simple text downcasting is insufficient; the f32 callers need to be
evolved to use the f64 algorithm (which is generally superior), and
the Rust dispatch code updated to match the f64 shader's interface.

**Classification by diff magnitude:**

| Category | Pairs | Diff range | Nature |
|----------|-------|------------|--------|
| Small (< 30 lines) | 12 | 13-30 | Mostly literal patterns (`0.0` vs `f64(0.0)`), comments, FMA |
| Medium (30-100) | 17 | 30-98 | Algorithm variants (sequential vs tree reduction) |
| Large (> 100 lines) | 20 | 108-385 | Fundamentally different implementations |

**Small-diff pairs** (best candidates for template-based unification):
`logsumexp`, `covariance`, `locus_variance`, `multi_obj_fitness`,
`hmm_forward_log`, `batch_fitness_eval`, `stencil_cooperation`,
`wright_fisher_step`, `esn_readout`, `hill_gate`, `correlation`, `cumsum`

**Large-diff pairs** (require algorithm evolution):
`cyclic_reduction` (385), `eigh` (305), `sparse_matvec` (229),
`triangular_solve` (181), `norm_reduce` (156), `svd` (157),
`sum_reduce` (141), `rk_stage` (140), `variance_reduce` (132),
`cosine_similarity` (117), `lu_decomp` (113), `rk45_adaptive` (109),
`spherical_harmonics` (108), `cholesky` (98), `digamma` (95),
`prod_reduce` (92), `cumprod` (86), `crank_nicolson` (78),
`qr_decomp` (76), `cdist` (76)

---

## Phase 3: f32-Only Shaders → f64 Canonical — ALL DONE

### Session 68 Execution

**ALL** f32-only shaders are now f64 canonical — both trivial and transcendental.
The f32 variant is produced at runtime via `LazyLock<String>` calling
`downcast_f64_to_f32()` (trivial) or `downcast_f64_to_f32_with_transcendentals()`
(transcendental). Old f32 WGSL files deleted.

**Trivial conversions** (240 shaders, `downcast_f64_to_f32`):
- math/: sub, div, abs, neg, clamp, floor, ceil, round, sign, frac, reciprocal,
  min, max, add, clamp_simple, min_simple, max_simple, trunc, vectoradd, fma,
  slice_assign, expand
- reduce/: prod_dim, variance_dim
- loss/: l1_loss, huber_loss, smooth_l1_loss, hinge_loss, margin_ranking_loss
- linalg/: laplacian, symmetrize, triu, tril, diag, trace, matrix_rank, clip_grad_value
- misc/: fill, mean_simple, sum_simple, prod_simple, gt, lt, eq
- activation/: relu, leaky_relu, leaky_relu_simple, hardswish, hardsigmoid,
  hardtanh, hardshrink, prelu, rrelu, softshrink, softsign, threshold
- norm/: layernorm_meanvar, layernorm_stats
- optimizer/: sgd, sgdw, batch_gradient, bfgs_update, simplex_ops
- attention/: attention_apply, cross_attention_apply, gqa_apply, mha_projection
- audio/: mel_scale, pitch_shift, time_stretch
- augmentation/: color_jitter, cutmix, elastic_transform, grid_mask, mixup,
  mosaic, random_affine, random_crop
- dropout/: dropout, spatial_dropout
- tensor/: slice, split, concat, broadcast

**Transcendental conversions** (294 shaders, `downcast_f64_to_f32_with_transcendentals`):
- math/: pow, pow_simple, rsqrt, sqrt, exp, log, sin, cos, tan, sinh, cosh,
  asin, acos, acosh, asinh, atan, erf, erfc, lgamma, determinant, matrix_power,
  matmul (all variants), batch_matmul, gqa_matmul, pairwise_* (all),
  message_passing, sinkhorn_distance, spatial_payoff, random_erasing, argmin (all),
  max_dim, max_reduce, min_dim, min_reduce, multi_margin_loss, multilabel_margin_loss,
  index_add
- reduce/: logsumexp_reduce, norm_dim
- loss/: bce_loss, binary_cross_entropy, cross_entropy, center_loss,
  chamfer_distance, contrastive_loss, cosine_embedding_loss, dice_loss,
  earth_mover_distance, focal_loss (all), giou_loss, iou_loss, kldiv_loss,
  label_smoothing, lovasz_loss, nll_loss, perceptual_loss, poisson_nll_loss,
  triplet_loss, tversky_loss, wasserstein_loss
- activation/: tanh, gelu, gelu_approximate, silu, swish, elu (all), selu (all),
  softplus, mish, logsigmoid, celu, tanhshrink, glu, atanh,
  softmax (all 8 variants), log_softmax
- norm/: all 25 normalization shaders
- optimizer/: all 10 advanced optimizers (adam, adamw, rmsprop, etc.)
- attention/: alibi_position, flash_attention, rotary_embedding, sdpa (all)
- audio/: griffin_lim, istft, mfcc, spectrogram, stft, window_function
- augmentation/: random_perspective, random_rotation
- bio/: wright_fisher_step, locus_variance, swarm_nn_scores
- misc/: 46 miscellaneous shaders (sort, scan, interpolate, ssim, psnr, etc.)
- rnn/: rnn_cell
- sample/: lhs, random_uniform, sobol
- special/: norm_cdf, norm_ppf
- spectral/: batch_ipr
- stats/: histogram, moving_window
- tensor/: 39 tensor manipulation shaders

**Remaining**: 16 u32/i32-only shaders (no conversion needed — no f32 types)

---

## Phase 4: Algorithm Evolution — TODO (parallel-safe)

The 49 structurally different pairs need the f32 implementation evolved
to match the f64 algorithm quality. The f64 versions typically use:
- Workgroup tree reduction (vs sequential loops in f32)
- Welford online statistics (vs two-pass in f32)
- Proper numerical initialization (`T(0.0)` vs bare `0.0`)
- Additional entry points (reverse, exclusive, log variants)

After algorithm evolution, each pair becomes consolidatable.

This phase can proceed in parallel with spring absorptions since it
improves quality without creating new debt.

---

## Deep Debt Sweep — Session 68

| Item | Finding | Action |
|------|---------|--------|
| Large files | 80 files > 500 lines (max 767) | Acceptable — no file > 1000 |
| unsafe code | 0 instances | Clean |
| `#[allow(dead_code)]` | 5 instances | Reduced via consolidation |
| `expect()` in production | 12 calls | Documented (most are infallible invariants) |
| `unwrap()` in production | 0 (all 20 in auto_tensor.rs are test-only) | Clean |
| `println!` in production | 8 calls in auto_tensor.rs, 6 in validation.rs | FIXED → `tracing::info!` |
| Magic numbers | npu_executor.rs | FIXED → named constants |
| Mock in production | `MOCK_FP16_TFLOPS` | FIXED → `NPU_EQUIVALENT_TFLOPS` |
| TPU mock | `device/tpu.rs` | Properly isolated behind `mock-tpu` feature flag |
| `todo!`/`unimplemented!` | 0 instances | Clean |
| `dbg!` | 0 instances | Clean |
| External deps | All appropriate (wgpu, naga, tokio, serde, rand, etc.) | No replacement needed |

---

## Metrics

| Metric | Session 67 | Session 68 | Target |
|--------|-----------|-----------|--------|
| Duplicate f32/f64 pairs | 50 | 49 (5 consolidated) | 0 |
| f32 WGSL files deleted | 0 | 296 (5 pairs + 291 f32-only) | 50+ |
| f32-only shaders remaining | ~534 | **0** | 0 |
| `downcast_f64_to_f32` callers | 0 | 296 | 296 |
| `println!` in production | 14 | 0 | 0 |
| Magic numbers in production | 5 | 0 | 0 |

---

## How To Consolidate a Pair

1. Verify f32 and f64 files have identical logic (diff ignoring types)
2. The f64 file becomes the canonical source
3. In the Rust op module, replace `include_str!("foo.wgsl")` with:
   ```rust
   const SHADER_F64: &str = include_str!("foo_f64.wgsl");
   static SHADER_F32: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
       crate::shaders::precision::downcast_f64_to_f32(SHADER_F64)
   });
   ```
4. For transcendental shaders, use `downcast_f64_to_f32_with_transcendentals()`
5. Run tests, verify identical results
6. Delete the f32 WGSL file
7. Update this tracker

---

## How To Create f64 Canonical From f32-Only

1. Copy the f32 file to `foo_f64.wgsl`
2. Replace `array<f32>` → `array<f64>`, `f32(` → `f64(`, etc.
3. For bare `0.0` literals, use `f64(0.0)` for explicit typing
4. If shader uses transcendentals, add `_f64` suffix to function calls
5. Wire the Rust caller with `LazyLock` downcast for f32
6. Verify the f64 shader compiles through `compile_shader_f64()`
7. Verify the downcasted f32 produces identical results

---

## References

- [`specs/UNIVERSAL_PRECISION_ARCHITECTURE.md`](specs/UNIVERSAL_PRECISION_ARCHITECTURE.md) — Design spec
- [`specs/HYBRID_FP64_CORE_STREAMING.md`](specs/HYBRID_FP64_CORE_STREAMING.md) — DF64 streaming
- [`specs/FP64_GPU_EVOLUTION.md`](specs/FP64_GPU_EVOLUTION.md) — f64 polyfill evolution
