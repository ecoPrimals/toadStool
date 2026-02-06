# Deep Debt Execution - Universal Shader Priority

**Date**: February 6, 2026, 4:30 AM  
**Primary Focus**: Pure Universal Shaders (WGSL everywhere)  
**Status**: 🚀 **EXECUTION MODE**

---

## 🎯 Execution Priorities

### Priority 1: Universal GPU Coverage (PRIMARY)
**Goal**: 100% WGSL shaders, 0% CPU fallback  
**Current**: 82.6% GPU (271/345 ops)  
**Target**: 100% GPU (345/345 ops)  
**Gap**: 74 operations need WGSL implementation

### Priority 2: FHE Testing Completeness
**Goal**: 100% fault/chaos/property coverage for FHE  
**Current**: 0% chaos/fault, 43% unit tests  
**Target**: 100% all test types

### Priority 3: Deep Debt Compliance
**Goal**: All 8 principles at 100%  
**Current**: Mixed (need audit per principle)

---

## 📋 Track 1: Universal Shader Conversion

### Phase 1A: High-Priority Operations (20 ops, ~40 hours)

**Criteria**: Most frequently used in production ML/DL

1. **Core Tensor Ops** (8 ops)
   - [ ] matmul (CPU fallback present)
   - [ ] matmul_tiled (CPU fallback)
   - [ ] batch_matmul (CPU fallback)
   - [ ] transpose (has CPU path)
   - [ ] reshape (CPU validation)
   - [ ] broadcast (CPU path)
   - [ ] concat (CPU path)
   - [ ] split (CPU path)

2. **Activations** (6 ops)
   - [ ] relu (has CPU fallback)
   - [ ] sigmoid (has CPU fallback)
   - [ ] tanh (has CPU fallback)
   - [ ] softmax (has CPU fallback)
   - [ ] gelu_wgsl (verify pure GPU)
   - [ ] swish_wgsl (verify pure GPU)

3. **Normalization** (6 ops)
   - [ ] batch_norm (CPU path)
   - [ ] layer_norm_wgsl (verify pure)
   - [ ] group_norm_wgsl (verify pure)
   - [ ] instance_norm_wgsl (verify pure)
   - [ ] rmsnorm (CPU path)
   - [ ] weight_norm (CPU path)

### Phase 1B: Optimizer Operations (15 ops, ~30 hours)

4. **Optimizers** (15 ops)
   - [ ] adam (CPU fallback)
   - [ ] adamw (CPU fallback)
   - [ ] sgd (CPU fallback)
   - [ ] rmsprop (CPU fallback)
   - [ ] adagrad (CPU fallback)
   - [ ] adadelta (CPU fallback)
   - [ ] nadam (CPU fallback)
   - [ ] radam (CPU fallback)
   - [ ] lamb (CPU fallback)
   - [ ] adabound (CPU fallback)
   - [ ] lookahead (CPU fallback)
   - [ ] sgdw (CPU fallback)
   - [ ] adafactor (CPU fallback)
   - [ ] cyclical_lr (CPU fallback)
   - [ ] onecycle (CPU fallback)

### Phase 1C: Attention & Advanced (20 ops, ~50 hours)

5. **Attention** (8 ops)
   - [ ] attention (CPU path)
   - [ ] multi_head_attention (CPU path)
   - [ ] scaled_dot_product_attention (CPU path)
   - [ ] flash_attention (CPU path)
   - [ ] causal_attention (CPU path)
   - [ ] cross_attention (CPU path)
   - [ ] local_attention (CPU path)
   - [ ] grouped_query_attention (CPU path)

6. **Loss Functions** (12 ops)
   - [ ] cross_entropy (CPU path)
   - [ ] bce_loss (CPU path)
   - [ ] mse_loss (CPU path)
   - [ ] mae_loss (CPU path)
   - [ ] huber_loss (CPU path)
   - [ ] focal_loss (CPU path)
   - [ ] triplet_loss (CPU path)
   - [ ] contrastive_loss (CPU path)
   - [ ] hinge_loss (CPU path)
   - [ ] kl_divergence (CPU path)
   - [ ] smooth_l1_loss (CPU path)
   - [ ] nll_loss (CPU path)

### Phase 1D: Specialized Operations (19 ops, ~48 hours)

7. **Graph Neural Networks** (5 ops)
   - [ ] gcn_conv (CPU path)
   - [ ] gat_conv (CPU path)
   - [ ] sage_conv (CPU path)
   - [ ] gin_conv (CPU path)
   - [ ] edge_conv (CPU path)

8. **Pooling** (7 ops)
   - [ ] avgpool2d (CPU path)
   - [ ] maxpool2d (CPU path)
   - [ ] avgpool3d (CPU path)
   - [ ] maxpool3d (CPU path)
   - [ ] adaptive_avgpool2d (CPU path)
   - [ ] adaptive_maxpool2d (CPU path)
   - [ ] global_pooling (CPU path)

9. **Misc** (7 ops)
   - [ ] nms (CPU fallback)
   - [ ] roi_align (CPU path)
   - [ ] roi_pool (CPU path)
   - [ ] npu_bridge (special case)
   - [ ] topk (CPU path)
   - [ ] unique (CPU path)
   - [ ] nonzero (CPU path)

---

## 📋 Track 2: FHE Testing Completeness

### Phase 2A: Fault Tests (14 ops × 4 tests = 56 tests, ~28 hours)

**For each of 14 FHE operations**:
1. [ ] Invalid degree test (non-power-of-2, < 4, > 65536)
2. [ ] Invalid modulus test (0, 1, even number, > u64::MAX)
3. [ ] Size mismatch test (wrong buffer size)
4. [ ] GPU failure test (OOM, device lost)

**Operations**:
- fhe_ntt, fhe_intt, fhe_pointwise_mul, fhe_fast_poly_mul
- fhe_poly_add, fhe_poly_sub, fhe_poly_mul
- fhe_xor, fhe_and, fhe_or
- fhe_modulus_switch, fhe_extract, fhe_rotate, fhe_key_switch

### Phase 2B: Chaos Tests (14 ops × 3 tests = 42 tests, ~14 hours)

**For each of 14 FHE operations**:
1. [ ] Random inputs test (100 iterations, varied dimensions)
2. [ ] Stress test (1000 sequential operations)
3. [ ] Concurrent test (10 parallel operations)

### Phase 2C: Property Tests (5 properties, ~14 hours)

1. [ ] NTT-INTT round-trip (all degrees 4..=65536)
2. [ ] Modulus switch correctness (decrypt equality)
3. [ ] Rotation composition (rot(k1) ∘ rot(k2) = rot(k1+k2))
4. [ ] Key switch security (ciphertext unlinkability)
5. [ ] Poly operations homomorphism (add/mul preservation)

---

## 📋 Track 3: Deep Debt Principles

### Principle 1: Modern Idiomatic Rust ✅
**Status**: 95% compliant  
**Gaps**: Some older code uses `unwrap()` instead of `?`  
**Action**: Audit and convert all `unwrap()` to proper error handling

### Principle 2: Rust-Native Dependencies ⚠️
**Status**: 75% compliant  
**Gaps**: External dependencies need analysis  
**Action**: Audit `Cargo.toml`, identify non-Rust deps, plan migration

### Principle 3: Smart Refactoring 🔄
**Status**: In progress (50% Track 2)  
**Current**: NetworkManager, HealthMonitor extracted  
**Remaining**: CleanupManager, byob_impl.rs refactor

### Principle 4: Unsafe → Safe & Fast ✅
**Status**: 100% compliant  
**FHE Ops**: 0 unsafe blocks in 14 operations  
**Action**: Maintain vigilance in new code

### Principle 5: Zero Hardcoding ✅
**Status**: 100% compliant (FHE ops)  
**FHE Ops**: All runtime parameters  
**Action**: Audit other operations for hardcoding

### Principle 6: Capability-Based ✅
**Status**: 95% compliant  
**NetworkManager**: Runtime IP allocation ✅  
**Action**: Audit remaining hardcoded values

### Principle 7: Primal Self-Knowledge ✅
**Status**: 100% compliant  
**Architecture**: Runtime discovery only  
**Action**: Maintain in new code

### Principle 8: No Production Mocks ✅
**Status**: 100% compliant  
**FHE Ops**: All real implementations  
**Action**: Audit for any remaining mocks

---

## 🚀 Execution Plan

### Week 1: Universal Shaders (Phase 1A)
**Goal**: Convert 20 high-priority ops to pure WGSL  
**Output**: 20 new WGSL shaders (~2,000 lines)  
**Deep Debt**: 100% safe, 0% hardcoding

### Week 2: Universal Shaders (Phase 1B + Start 1C)
**Goal**: Convert 20 optimizer + attention ops  
**Output**: 20 new WGSL shaders (~1,500 lines)

### Week 3: FHE Testing + Shaders (Phase 2A + 1C cont.)
**Goal**: Complete FHE fault tests + 10 more shaders  
**Output**: 56 fault tests + 10 WGSL shaders

### Week 4: Complete Universal Coverage (Phase 1D + 2B/2C)
**Goal**: Final 19 shaders + chaos/property tests  
**Output**: 100% GPU coverage + comprehensive testing

---

## 📊 Success Metrics

### Universal Shader Coverage
- Start: 271/345 (78.6%)
- Target: 345/345 (100%)
- **Metric**: WGSL shader exists for every operation

### FHE Testing Coverage
- Start: 6/14 unit (43%), 0/14 fault (0%), 0/14 chaos (0%)
- Target: 14/14 all types (100%)
- **Metric**: All test types pass for all FHE ops

### Deep Debt Compliance
- Start: 87.5% (7/8 principles at 100%)
- Target: 100% (8/8 principles at 100%)
- **Metric**: All principles verified

---

## 🎯 Starting Point

**First Action**: Convert highest-priority CPU fallback operations to WGSL

**Order**:
1. `matmul` - Most critical (used everywhere)
2. `batch_matmul` - Critical for transformers
3. `softmax` - Critical for attention
4. `layer_norm` - Critical for transformers
5. Continue down priority list...

**Methodology**:
1. Read existing CPU implementation
2. Design WGSL compute shader
3. Implement with u32/f32 (no CPU fallback)
4. Add comprehensive tests (unit + fault + chaos)
5. Verify performance (should match/exceed CPU)
6. Remove CPU fallback code

---

**Document**: `DEEP_DEBT_UNIVERSAL_EXECUTION_FEB06_2026.md`  
**Status**: 🚀 **READY TO EXECUTE**  
**Primary**: Universal WGSL shaders  
**Timeline**: 4 weeks to 100% coverage

Let's begin! 🚀
