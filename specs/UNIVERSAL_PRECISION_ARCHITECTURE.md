# Universal Precision Architecture

**Date**: February 26, 2026
**Status**: Dual-layer universal precision — op_preamble + naga IR rewrite both operational
**Classification**: Core architecture — applies to every shader in BarraCuda

---

## Principle

**Math is universal, precision is silicon.**

The same algorithm written once compiles to any precision. The WGSL source
is the true math (conceptually f64). The compilation pipeline handles
type specialization, polyfill injection, and driver patching — the same
pattern that solved f64 builtins with polyfills, extended to all precisions.

Every shader must run at every precision. Everything else is implementation
evolution.

---

## Architecture

```
Shader source (f64 — true math)
       │
compile_shader_universal(source, precision)
       │
  ┌────┼────────────────┐
  │    │                 │
  F32  F64               Df64
  │    │                 │
  downcast_f64_to_f32    polyfill + ILP     inject df64_core +
  (text transform)       + sovereign        df64_transcendentals
  │                      compiler           │
  compile_shader()       compile_shader_f64()  compile_shader_df64()
  │                      │                     │
  WGSL → wgpu module    WGSL → naga-IR →      WGSL → naga-IR →
                         FMA fusion → DCE →    FMA fusion → DCE →
                         SPIR-V passthrough    SPIR-V passthrough
```

### Compilation Pipelines

| Precision | Pipeline | Key Transforms |
|-----------|----------|---------------|
| **F32** | `compile_shader_universal(src, F32)` | `downcast_f64_to_f32()`: `array<f64>` → `array<f32>`, `: f64` → `: f32`, `f64(` → `f32(`. Sentinel-protects `_f64(` function names. |
| **F64** | `compile_shader_universal(src, F64)` | `compile_shader_f64()`: driver-aware transcendental patching (28 polyfills), ILP optimizer, sovereign compiler SPIR-V passthrough. |
| **Df64** | `compile_shader_universal(src, Df64)` | `compile_shader_df64()`: auto-inject `df64_core.wgsl` + `df64_transcendentals.wgsl`, ILP optimizer, sovereign compiler. Separate source required (structural layout differs: `vec2<f32>` storage, `df64_add`/`df64_mul` calls). |
| **F16** | `compile_shader_universal(src, F16)` | `downcast_f64_to_f16()`: sentinel-protected type transform + `clamp_f64_range_literals_f16()` (±65504.0). Requires `SHADER_F16` feature. |

### Template System

`{{SCALAR}}`-parameterized templates generate valid WGSL at any precision:

```rust
let shader = ShaderTemplate::elementwise_add(Precision::F64);
// Produces: array<f64>, f64 arithmetic
let shader = ShaderTemplate::elementwise_add(Precision::F32);
// Produces: array<f32>, f32 arithmetic
let shader = ShaderTemplate::elementwise_add(Precision::Df64);
// Produces: array<vec2<f32>>, but NOTE: operator overloading
// not available — Df64 shaders need df64_add() calls
```

**12 universal templates** (all generate valid f32/f64):
- Elementwise: add, mul, sub, fma, abs, neg, clamp, saxpy
- Reduction: sum, mean
- Loss: MSE, MAE
- Inner product: dot

### Downcast Functions

| Function | Use Case |
|----------|----------|
| `downcast_f64_to_f32(source)` | Universal math shaders using basic arithmetic (`+`, `-`, `*`, `/`, `fma`, `abs`, `clamp`). No transcendentals. |
| `downcast_f64_to_f32_with_transcendentals(source)` | Shaders that call `math_f64` polyfills. Maps `exp_f64` → `exp`, `sin_f64` → `sin`, etc. |

### Dual-Layer DF64 Coverage

DF64 precision is handled by two complementary layers:

**Layer 1 — Operation Preamble (source level):**
New shaders use abstract operations (`op_add`, `op_mul`, `Scalar` type alias).
The preamble provides precision-specific implementations:
- f32/f64: trivial wrappers around native operators (compiler inlines)
- DF64: routes to `df64_add`, `df64_mul`, etc.

```rust
// One shader, all precisions:
device.compile_op_shader(source, Precision::Df64, label);
```

**Layer 2 — Naga IR Rewrite (compiler level):**
Existing f64 shaders with infix operators get transformed automatically.
The sovereign compiler's `df64_rewrite` module:
1. Parses f64 WGSL with naga for type analysis
2. Walks the typed IR to find f64 `Binary{+,-,*,/}` and `Unary{-}`
3. Replaces with bridge functions (`_df64_add_f64(a, b)` etc.) that
   accept f64, compute in Df64, return f64 — type system untouched
4. Bridge functions get prepended alongside df64 core library

Together: op_preamble makes new code portable by design,
naga makes everything portable by force.

---

## Precision Inventory (700 shaders — Session 68)

| Category | Count | Coverage |
|----------|-------|----------|
| **f32 (LazyLock downcast)** | 497 (71%) | All generated from f64 canonical via `downcast_f64_to_f32()`. Zero f32-only. |
| **Native f64** | 182 (26%) | Scientific computing, lattice QCD, MD forces |
| **Df64** | 19 (3%) | Consumer GPU f64-class on FP32 cores |
| **Df64 infrastructure** | 2 | `df64_core.wgsl`, `df64_transcendentals.wgsl` |

### By Directory

| Directory | Total | f32 | f64 | df64 | Notes |
|-----------|-------|-----|-----|------|-------|
| math/ | 108 | 88 | 15 | 5 | Core arithmetic, 4 duplicate pairs |
| reduce/ | 31 | 15 | 14 | 2 | Well covered |
| linalg/ | 32 | 17 | 14 | 1 | 9 duplicate pairs |
| special/ | 36 | 15 | 21 | 0 | f64 dominant (transcendentals) |
| loss/ | 34 | 31 | 3 | 0 | Mostly f32-only |
| activation/ | 37 | 37 | 0 | 0 | All f32-only (transcendental-dependent) |
| bio/ | 38 | 13 | 25 | 0 | 6 duplicate pairs |
| lattice/ | 36 | 0 | 30 | 6 | All f64/df64 (physics-critical) |
| tensor/ | 43 | 43 | 0 | 0 | Shape ops (precision-agnostic) |
| norm/ | 27 | 27 | 0 | 0 | Normalization (transcendental-dependent) |
| pooling/ | 17 | 17 | 0 | 0 | Max/avg/adaptive pooling |
| conv/ | 11 | 11 | 0 | 0 | Convolution (f32 inference) |
| misc/ | 64 | 59 | 5 | 0 | Mixed utilities |
| science/ | 13 | 0 | 13 | 0 | HFB physics (all f64) |
| sparse/ | 5 | 0 | 5 | 0 | Sparse linear algebra (all f64) |

### Existing Duplicate Pairs (50)

These have identical logic in both f32 and f64 files — consolidation candidates:

| Domain | Pairs |
|--------|-------|
| reduce/ | cumprod, cumsum, mean_dim, mean_reduce, norm_reduce, prod_reduce, std_dim, std_reduce, sum_dim, sum_reduce, variance_reduce (11) |
| linalg/ | cholesky, cyclic_reduction, eigh, inverse, linsolve, lu_decomp, qr_decomp, svd, triangular_solve (9) |
| special/ | bessel_i0, bessel_j0, bessel_j1, bessel_k0, beta, correlation, covariance, digamma, hermite, laguerre, legendre, spherical_harmonics, variance (13) |
| bio/ | hill_gate, locus_variance, multi_obj_fitness, stencil_cooperation, swarm_nn_forward, wright_fisher_step (6) |
| math/ | cosine_similarity, elementwise_add, elementwise_mul, logsumexp (4) |
| loss/ | kl_divergence, mae_loss, mse_loss (3) |
| ml/ | batch_fitness_eval, esn_readout, hmm_forward_log (3) |
| misc/ | cdist, prng_xoshiro, sparse_matvec (3) |
| numerical/ | rk45_adaptive, rk_stage (2) |
| pde/ | crank_nicolson (1) |

---

## Evolution Strategy

### Phase 1: Infrastructure (DONE — Session 67-68)

- [x] `Precision::Df64` enum variant
- [x] `compile_shader_universal(source, precision)`
- [x] `compile_op_shader(source, precision)` — operation preamble path
- [x] `compile_template(template, precision)`
- [x] `downcast_f64_to_f32()` with sentinel protection
- [x] `downcast_f64_to_f32_with_transcendentals()`
- [x] `downcast_f64_to_df64()` — text-based DF64 type transform
- [x] `rewrite_f64_infix_full()` — naga-guided infix operator rewrite (Phase 5)
- [x] `Precision::op_preamble()` — abstract operations for all 4 precisions
- [x] Bridge functions for f64→DF64→f64 transparent routing
- [x] 12 universal `{{SCALAR}}` templates
- [x] Full precision inventory
- [x] 122 passing tests (precision + sovereign + df64 rewrite + universal shader validation + chaos + fault)

### Phase 2: Consolidate Existing Duplicates

For each of the 50 f32/f64 duplicate pairs:
1. The f64 file becomes the canonical source
2. Callers that load the f32 file switch to `downcast_f64_to_f32(f64_source)`
3. The f32 file is removed (the template or downcast produces it)
4. Tests verify identical behavior

Priority order: reduce/ (11 pairs), math/ (4 pairs), loss/ (3 pairs),
linalg/ (9 pairs), special/ (13 pairs), rest.

### Phase 3: Extend Universal Coverage

f32-only shaders that are precision-agnostic (basic arithmetic only):
- math/: abs, neg, elementwise_div, elementwise_sub, floor, ceil, round, etc.
- reduce/: logsumexp_reduce, norm_dim, prod_dim, variance_dim
- loss/: basic losses (bce, cross_entropy, huber, l1, smooth_l1)

For each: write f64 canonical source, downcast to f32 for existing callers.

### Phase 4: Transcendental-Dependent Shaders

Activation functions (37), normalization (27), some losses:
- Write as f64 using polyfill calls (`exp_f64`, `sin_f64`, etc.)
- `downcast_f64_to_f32_with_transcendentals()` produces f32 (native builtins)
- `compile_shader_f64()` handles the f64 compilation (polyfill injection)

### Phase 5: Domain-Specific (No Change Needed)

Lattice QCD (36), science/HFB (13), MD forces, sparse linear algebra:
- Already f64 or df64 — physics demands it
- Keep precision-specific source files
- Already compiled through `compile_shader_f64()` / `compile_shader_df64()`

---

## Proven Validation

This architecture is not theory:

| Evidence | Result |
|----------|--------|
| **hotSpring QCD on 3090** | 32⁴ lattice on 1.6% of chip (native f64). DF64 gives 9.9× throughput. |
| **5 Springs validation** | 4,000+ acceptance checks across physics, chemistry, biology, agriculture |
| **GPU FP64 accuracy** | 4.55e-13 MeV max error vs CPU reference (hotSpring) |
| **Cross-vendor** | Bit-identical results: RTX 4070 / RTX 3090 / RX 6950 XT |
| **DF64 throughput** | 3.24 TFLOPS DF64 vs 0.33 TFLOPS native f64 on RTX 3090 |

---

## References

- [`HYBRID_FP64_CORE_STREAMING.md`](./HYBRID_FP64_CORE_STREAMING.md) — DF64 core streaming spec
- [`FP64_GPU_EVOLUTION.md`](./FP64_GPU_EVOLUTION.md) — f64 polyfill library and evolution
- [`SOVEREIGN_COMPUTE_EVOLUTION.md`](./SOVEREIGN_COMPUTE_EVOLUTION.md) — naga-IR optimizer and SPIR-V passthrough
- Precision bottleneck — RESOLVED, archived to `ecoPrimals/fossil/`
