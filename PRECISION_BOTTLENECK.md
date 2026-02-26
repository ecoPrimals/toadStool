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
| **3** | f32-only → f64 canonical | 240 trivial, 138 transcendental | TODO |
| **4** | Algorithm evolution (f32 → f64 quality) | 49 | TODO |
| **5** | Domain-specific (lattice/MD/HFB) — no change | 0 | N/A |

**Gate opens when**: Phase 2 complete + Phase 3 trivials converted.
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

## Phase 3: f32-Only Shaders → f64 Canonical — TODO

### Session 68 Discovery

Comprehensive inventory found **393 f32-only shaders** (no f64 counterpart):
- **240 TRIVIAL**: Only basic arithmetic, abs, min/max — type replacement
- **138 TRANSCENDENTAL**: Use exp, log, sin, cos, sqrt, pow — need f64 polyfills
- **15 NO_F32_TYPE**: u32/i32 only — no precision conversion needed

### Priority 1: Core arithmetic (math/) — trivial type replacement

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
| f32 WGSL files deleted | 0 | 5 | 50+ |
| f32-only shaders identified | ~30 | 240 trivial + 138 transcendental | 0 |
| `downcast_f64_to_f32` callers | 0 | 5 | 50+ |
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
