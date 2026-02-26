# Precision Bottleneck — Evolution Gate

**Date**: February 24, 2026 — Session 67
**Status**: ACTIVE — solve all precision debt before absorbing more from springs

---

## Gate Rule

**No new spring absorptions until all phases below are complete.**

The universal precision architecture is an evolution bottleneck. Absorbing
more shaders while the foundation is split between separate f32/f64 files
creates compound debt. Solve the bottleneck first, then every future
absorption automatically gets multi-precision support.

---

## Phase Status

| Phase | Description | Items | Status |
|-------|-------------|-------|--------|
| **1** | Infrastructure (pipeline + templates) | 6 | DONE |
| **2** | Consolidate 50 duplicate f32/f64 pairs | 50 | TODO |
| **3** | f32-only universals → f64 canonical | ~30 | TODO |
| **4** | Transcendental-dependent (activation/norm) | ~64 | TODO |
| **5** | Domain-specific (lattice/MD/HFB) — no change | 0 | N/A |

**Gate opens when**: Phases 2-3 are complete. Phase 4 can proceed in parallel
with spring absorptions since it adds coverage without creating new debt.

---

## Phase 1: Infrastructure — DONE

- [x] `Precision::Df64` enum variant
- [x] `compile_shader_universal(source, precision)` on `WgpuDevice`
- [x] `compile_template(template, precision)` on `WgpuDevice`
- [x] `downcast_f64_to_f32()` with sentinel-protected `_f64(` names
- [x] `downcast_f64_to_f32_with_transcendentals()` (polyfill → native)
- [x] 12 universal `{{SCALAR}}` templates

---

## Phase 2: Consolidate Duplicate Pairs — TODO

Each pair: f64 becomes source of truth, f32 produced by downcast, remove
separate f32 file. Verify callers still work.

### reduce/ (11 pairs) — highest impact, simplest logic

| Shader | f32 | f64 | Status |
|--------|-----|-----|--------|
| sum_reduce | `reduce/sum_reduce.wgsl` | `reduce/sum_reduce_f64.wgsl` | TODO |
| mean_reduce | `reduce/mean_reduce.wgsl` | `reduce/mean_reduce_f64.wgsl` | TODO |
| std_reduce | `reduce/std_reduce.wgsl` | `reduce/std_reduce_f64.wgsl` | TODO |
| variance_reduce | `reduce/variance_reduce.wgsl` | `reduce/variance_reduce_f64.wgsl` | TODO |
| sum_dim | `reduce/sum_dim.wgsl` | `reduce/sum_dim_f64.wgsl` | TODO |
| mean_dim | `reduce/mean_dim.wgsl` | `reduce/mean_dim_f64.wgsl` | TODO |
| std_dim | `reduce/std_dim.wgsl` | `reduce/std_dim_f64.wgsl` | TODO |
| norm_reduce | `reduce/norm_reduce.wgsl` | `reduce/norm_reduce_f64.wgsl` | TODO |
| prod_reduce | `reduce/prod_reduce.wgsl` | `reduce/prod_reduce_f64.wgsl` | TODO |
| cumsum | `reduce/cumsum.wgsl` | `reduce/cumsum_f64.wgsl` | TODO |
| cumprod | `reduce/cumprod.wgsl` | `reduce/cumprod_f64.wgsl` | TODO |

### math/ (4 pairs)

| Shader | f32 | f64 | Status |
|--------|-----|-----|--------|
| elementwise_add | `math/elementwise_add.wgsl` | `math/elementwise_add_f64.wgsl` | TODO |
| elementwise_mul | `math/elementwise_mul.wgsl` | `math/elementwise_mul_f64.wgsl` | TODO |
| cosine_similarity | `math/cosine_similarity.wgsl` | `math/cosine_similarity_f64.wgsl` | TODO |
| logsumexp | `math/logsumexp.wgsl` | `math/logsumexp_f64.wgsl` | TODO |

### loss/ (3 pairs)

| Shader | f32 | f64 | Status |
|--------|-----|-----|--------|
| mse_loss | `loss/mse_loss.wgsl` | `loss/mse_loss_f64.wgsl` | TODO |
| mae_loss | `loss/mae_loss.wgsl` | `loss/mae_loss_f64.wgsl` | TODO |
| kl_divergence | `loss/kl_divergence.wgsl` | `loss/kl_divergence_f64.wgsl` | TODO |

### linalg/ (9 pairs)

| Shader | f32 | f64 | Status |
|--------|-----|-----|--------|
| cholesky | `linalg/cholesky.wgsl` | `linalg/cholesky_f64.wgsl` | TODO |
| eigh | `linalg/eigh.wgsl` | `linalg/eigh_f64.wgsl` | TODO |
| inverse | `linalg/inverse.wgsl` | `linalg/inverse_f64.wgsl` | TODO |
| linsolve | `linalg/linsolve.wgsl` | `linalg/linsolve_f64.wgsl` | TODO |
| lu_decomp | `linalg/lu_decomp.wgsl` | `linalg/lu_decomp_f64.wgsl` | TODO |
| qr_decomp | `linalg/qr_decomp.wgsl` | `linalg/qr_decomp_f64.wgsl` | TODO |
| svd | `linalg/svd.wgsl` | `linalg/svd_f64.wgsl` | TODO |
| triangular_solve | `linalg/triangular_solve.wgsl` | `linalg/triangular_solve_f64.wgsl` | TODO |
| cyclic_reduction | `linalg/cyclic_reduction.wgsl` | `linalg/cyclic_reduction_f64.wgsl` | TODO |

### special/ (13 pairs) — transcendental-dependent

| Shader | f32 | f64 | Status |
|--------|-----|-----|--------|
| bessel_i0 | ✓ | ✓ | TODO |
| bessel_j0 | ✓ | ✓ | TODO |
| bessel_j1 | ✓ | ✓ | TODO |
| bessel_k0 | ✓ | ✓ | TODO |
| beta | ✓ | ✓ | TODO |
| correlation | ✓ | ✓ | TODO |
| covariance | ✓ | ✓ | TODO |
| digamma | ✓ | ✓ | TODO |
| hermite | ✓ | ✓ | TODO |
| laguerre | ✓ | ✓ | TODO |
| legendre | ✓ | ✓ | TODO |
| spherical_harmonics | ✓ | ✓ | TODO |
| variance | ✓ | ✓ | TODO |

### bio/ (6 pairs)

| Shader | Status |
|--------|--------|
| hill_gate | TODO |
| locus_variance | TODO |
| multi_obj_fitness | TODO |
| stencil_cooperation | TODO |
| swarm_nn_forward | TODO |
| wright_fisher_step | TODO |

### ml/ + misc/ + numerical/ + pde/ (9 pairs)

| Shader | Status |
|--------|--------|
| batch_fitness_eval | TODO |
| esn_readout | TODO |
| hmm_forward_log | TODO |
| cdist | TODO |
| prng_xoshiro | TODO |
| sparse_matvec | TODO |
| rk45_adaptive | TODO |
| rk_stage | TODO |
| crank_nicolson | TODO |

---

## Phase 3: f32-Only Universals → f64 Canonical — TODO

Shaders that are purely arithmetic (no transcendentals) but only exist as f32.
Write f64 canonical, callers use `downcast_f64_to_f32()`.

### Priority 1: Core arithmetic (math/)

| Shader | Logic | Status |
|--------|-------|--------|
| elementwise_div | `a / b` | TODO |
| elementwise_sub | `a - b` | TODO |
| abs | `abs(x)` | TODO |
| neg | `-x` | TODO |
| clamp | `clamp(x, lo, hi)` | TODO |
| floor | `floor(x)` | TODO |
| ceil | `ceil(x)` | TODO |
| round | `round(x)` | TODO |
| sign | `sign(x)` | TODO |
| frac | `fract(x)` | TODO |
| min | `min(a, b)` | TODO |
| max | `max(a, b)` | TODO |
| reciprocal | `1.0 / x` | TODO |
| sqrt | `sqrt(x)` | TODO |

### Priority 2: Reduce ops

| Shader | Logic | Status |
|--------|-------|--------|
| logsumexp_reduce | log + sum + exp | TODO |
| norm_dim | sqrt(sum(x²)) | TODO |
| prod_dim | product reduction | TODO |
| variance_dim | var = E[x²] - E[x]² | TODO |

### Priority 3: Core losses

| Shader | Logic | Status |
|--------|-------|--------|
| l1_loss | abs(pred - target) | TODO |
| huber_loss | smooth L1 | TODO |
| smooth_l1_loss | smooth L1 variant | TODO |
| bce_loss | binary cross entropy | TODO |
| cross_entropy | categorical CE | TODO |

---

## Phase 4: Transcendental-Dependent — TODO (parallel-safe)

These use `exp`, `sin`, `cos`, `tanh` etc. Write as f64 with polyfill calls,
use `downcast_f64_to_f32_with_transcendentals()` for f32 variant.

### activation/ (37 shaders)

relu, gelu, silu, selu, elu, leaky_relu, softmax, log_softmax, tanh,
sigmoid, swish, mish, hardswish, hardsigmoid, softplus, etc.

### norm/ (27 shaders)

batch_norm, layer_norm, group_norm, instance_norm, rms_norm, etc.

### Remaining losses (22 shaders)

focal_loss, dice_loss, contrastive_loss, triplet_loss, etc.

---

## Metrics

| Metric | Current | Target |
|--------|---------|--------|
| Duplicate f32/f64 pairs | 50 | 0 |
| f32-only universal shaders | ~30 | 0 |
| Universal templates | 12 | 25+ |
| Shaders compilable at all precisions | ~50 (via templates) | 200+ |
| `downcast_f64_to_f32` callers | 0 (infrastructure only) | 50+ |

---

## How To Consolidate a Pair

1. Verify f32 and f64 files have identical logic (diff ignoring types)
2. The f64 file becomes the canonical source
3. In the Rust op module, replace `include_str!("foo.wgsl")` with:
   ```rust
   const SHADER_F64: &str = include_str!("foo_f64.wgsl");
   // f32 variant produced by downcast at compile time
   fn shader_f32() -> String {
       crate::shaders::precision::downcast_f64_to_f32(SHADER_F64)
   }
   ```
4. For transcendental shaders, use `downcast_f64_to_f32_with_transcendentals()`
5. Run tests, verify identical results
6. Remove the f32 file
7. Update this tracker

---

## References

- [`specs/UNIVERSAL_PRECISION_ARCHITECTURE.md`](specs/UNIVERSAL_PRECISION_ARCHITECTURE.md) — Design spec
- [`specs/HYBRID_FP64_CORE_STREAMING.md`](specs/HYBRID_FP64_CORE_STREAMING.md) — DF64 streaming
- [`specs/FP64_GPU_EVOLUTION.md`](specs/FP64_GPU_EVOLUTION.md) — f64 polyfill evolution
