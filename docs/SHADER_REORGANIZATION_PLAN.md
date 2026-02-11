# BarraCUDA Shader Library Reorganization Plan

**Date**: February 11, 2026  
**Status**: APPROVED FOR EXECUTION

---

## Problem Statement

With 414 WGSL shaders, the current flat structure is unmaintainable:
- **378 shaders** in a single flat `src/shaders/` directory
- Hard to find related operations
- No clear structure for adding new shaders
- Poor discoverability for reusable components

---

## Proposed Directory Structure

```
crates/barracuda/src/shaders/
├── activation/         ~28  (gelu, silu, elu, leaky_relu, mish, swish, etc.)
├── loss/               ~32  (focal, dice, mse, bce, iou, chamfer, etc.)
├── optimizer/          ~9   (adam*, sgd, lamb, rmsprop, nadam, etc.)
├── pooling/            ~18  (avg_pool*, max_pool*, adaptive_*, roi_*, etc.)
├── conv/               ~12  (conv1d/2d/3d, depthwise, separable, dilated, etc.)
├── norm/               ~22  (batch_norm, layer_norm, group_norm, instance_norm, etc.)
├── math/               ~45  (trig, exp, log, floor, ceil, sqrt, etc.)
├── reduce/             ~25  (sum, mean, argmax, logsumexp, variance, etc.)
├── linalg/             ~12  (cholesky, triangular_solve, eigh, linsolve, inverse, etc.)
├── tensor/             ~55  (concat, slice, scatter, gather, reshape, transpose, etc.)
├── attention/          ~11  (attention_*, gqa_*, flash_attention, alibi, etc.)
├── rnn/                ~5   (lstm_cell, gru_cell, bi_lstm)
├── gnn/                ~9   (gcn_conv, gat_conv, graph_conv, edge_conv, etc.)
├── detection/          ~6   (anchor_generator, box_iou, nms, bbox_transform, etc.)
├── augmentation/       ~12  (cutmix, mixup, random_*, elastic_transform, etc.)
├── audio/              ~12  (stft, istft, mfcc, mel_scale, spectrogram, etc.)
├── gradient/           2    (clip_grad_norm, clip_grad_value)
├── dropout/            2    (dropout, spatial_dropout)
├── special/            4    (bessel_j0, bessel_j1, bessel_i0, bessel_k0)
├── interpolation/      1    (rbf_kernel)
├── fhe/                13   (ntt, intt, poly_add, poly_mul, key_switch, etc.)
├── complex/            10   (add, sub, mul, div, exp, log, pow, sqrt, abs, conj)
├── fft/                2    (fft_1d, ifft_normalize)
├── md/
│   ├── forces/         5    (coulomb, lennard_jones, yukawa, morse, born_mayer)
│   ├── integrators/    3    (velocity_verlet, rk4, laplacian)
│   └── pbc.wgsl        1    (periodic boundary conditions)
└── misc/               ~42  (matmul*, embedding, quantize, u64_emu, etc.)
```

---

## Migration Strategy

### Phase 1: Create Directory Structure (5 min)
```bash
cd crates/barracuda/src/shaders
mkdir -p activation loss optimizer pooling conv norm math reduce linalg tensor \
         attention rnn gnn detection augmentation audio gradient dropout special \
         interpolation fhe complex fft md/forces md/integrators misc
```

### Phase 2: Move Shaders by Category (automated script, 10 min)
- Generate move script from analysis
- Execute moves in batches by category
- Verify no files lost

### Phase 3: Update `include_str!` References (automated, 15 min)
- Find all `include_str!("../shaders/{name}.wgsl")` in `src/ops/`
- Replace with `include_str!("../shaders/{category}/{name}.wgsl")`
- 445 Rust files to update

### Phase 4: Verification (5 min)
```bash
cargo check -p barracuda
cargo test -p barracuda --lib
cargo clippy -p barracuda
```

### Phase 5: Documentation (10 min)
- Update `docs/shaders/README.md` with new structure
- Add category index
- Document common patterns per category

---

## Implementation Details

### Categorization Rules

| Category | Rule |
|----------|------|
| `activation/` | Non-linear point-wise: relu, gelu, sigmoid, tanh, etc. |
| `loss/` | Loss functions and distance metrics |
| `optimizer/` | Weight update rules: adam, sgd, momentum, etc. |
| `pooling/` | Spatial reduction: max_pool, avg_pool, roi_pool, etc. |
| `conv/` | Convolution operations (all dimensions) |
| `norm/` | Normalization: batch, layer, group, instance, etc. |
| `math/` | Element-wise math: trig, exp, log, round, etc. |
| `reduce/` | Tensor reduction: sum, mean, argmax, etc. |
| `linalg/` | Linear algebra: decompositions, solves, etc. |
| `tensor/` | Shape manipulation: concat, slice, reshape, etc. |
| `attention/` | Attention mechanisms |
| `rnn/` | Recurrent networks |
| `gnn/` | Graph neural networks |
| `detection/` | Object detection primitives |
| `augmentation/` | Data augmentation |
| `audio/` | Audio/signal processing |
| `gradient/` | Gradient manipulation |
| `dropout/` | Regularization |
| `special/` | Special mathematical functions |
| `interpolation/` | Interpolation kernels |
| `fhe/` | Fully homomorphic encryption |
| `complex/` | Complex number arithmetic |
| `fft/` | Fast Fourier transforms |
| `md/` | Molecular dynamics |
| `misc/` | Utilities that don't fit elsewhere |

### Example Migrations

**Before:**
```rust
// crates/barracuda/src/ops/gelu.rs
const SHADER: &str = include_str!("../shaders/gelu.wgsl");
```

**After:**
```rust
// crates/barracuda/src/ops/gelu.rs
const SHADER: &str = include_str!("../shaders/activation/gelu.wgsl");
```

---

## Benefits

1. **Discoverability**: Find related shaders by category
2. **Reusability**: Identify common patterns within categories
3. **Maintainability**: Easier to add new shaders in the right place
4. **Documentation**: Category-level documentation and examples
5. **Testing**: Category-level test suites
6. **Performance**: Category-specific optimizations

---

## Rollback Plan

If issues arise:
```bash
cd crates/barracuda/src/shaders
find activation loss optimizer ... -name "*.wgsl" -exec mv {} . \;
rmdir activation loss optimizer ...
git checkout src/ops/
```

---

## Timeline

- **Total**: ~45 minutes
- Can be done incrementally by category if needed
- No breaking changes to public API

---

## Success Criteria

- [ ] All 414 shaders organized into categories
- [ ] `cargo check -p barracuda` passes
- [ ] `cargo test -p barracuda --lib` passes
- [ ] `cargo clippy -p barracuda` passes
- [ ] Documentation updated
- [ ] Zero files lost (verify count)

---

**Approved by**: ToadStool Team  
**Execution start**: TBD
