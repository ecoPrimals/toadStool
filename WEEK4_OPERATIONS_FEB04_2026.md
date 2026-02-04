# Week 4 Operations - February 4, 2026

## Goal
Implement 15 WGSL operations: **184 → 199** (58.6% → 63.4% coverage)

## Selected Operations (15 total)

### Priority 1: Linear Algebra & Core Ops (2)
1. **determinant** - Matrix determinant calculation
   - Essential for linear algebra
   - Used in matrix inverse, LU decomposition
   - Importance: Critical for numerical methods

2. **diag** - Diagonal matrix operations
   - Extract/create diagonal matrices
   - Used in eigenvalue computations
   - Importance: Core linear algebra primitive

### Priority 2: CNN Advanced Features (4)
3. **circular_pad2d** - Circular padding for convolutions
   - Enables wraparound padding
   - Used in style transfer, texture synthesis
   - Importance: Completes padding options

4. **dilated_conv2d** - Atrous/dilated convolutions
   - Expands receptive field without parameters
   - Used in semantic segmentation (DeepLab)
   - Importance: State-of-the-art segmentation

5. **fractional_max_pool2d** - Fractional pooling
   - Stochastic pooling with fractional ratios
   - Reduces overfitting
   - Importance: Advanced regularization

6. **flash_attention** - Optimized attention mechanism
   - Memory-efficient attention computation
   - 2-4x faster than standard attention
   - Importance: Critical for long sequences

### Priority 3: Loss Functions & Medical Imaging (2)
7. **dice_loss** - Dice coefficient loss
   - Medical image segmentation standard
   - Handles class imbalance
   - Importance: Medical imaging essential

8. **earth_mover_distance** - Wasserstein distance
   - Distribution comparison metric
   - Used in GANs (WGAN)
   - Importance: Advanced generative models

### Priority 4: Quantization & Optimization (2)
9. **dequantize** - Reverse quantization
   - Complements quantize operation
   - Used in inference pipelines
   - Importance: Completes quantization support

10. **fake_quantize** - Quantization-aware training
    - Simulates quantization during training
    - Improves quantized model accuracy
    - Importance: Production deployment prep

### Priority 5: Data Augmentation (2)
11. **cutmix** - CutMix augmentation
    - Modern data augmentation technique
    - Mixes regions between images
    - Importance: State-of-the-art training

12. **elastic_transform** - Elastic deformation
    - Medical imaging augmentation
    - Simulates tissue deformation
    - Importance: Medical imaging standard

### Priority 6: Learning Rate & Metrics (3)
13. **cyclical_lr** - Cyclical learning rates
    - Learning rate scheduling strategy
    - Improves convergence
    - Importance: Training optimization

14. **cosine_embedding_loss** - Metric learning loss
    - Learns embedding spaces
    - Used in face recognition, retrieval
    - Importance: Metric learning essential

15. **cross_product** - Vector cross product
    - 3D vector operations
    - Used in graphics, physics
    - Importance: 3D geometry ops

## Implementation Strategy

### Week 4 Sprint Plan
1. **Day 1-2**: Linear algebra (determinant, diag)
2. **Day 2-3**: CNN features (circular_pad2d, dilated_conv2d, fractional_max_pool2d)
3. **Day 3-4**: Attention & loss (flash_attention, dice_loss, earth_mover_distance)
4. **Day 4-5**: Quantization (dequantize, fake_quantize)
5. **Day 5-6**: Augmentation (cutmix, elastic_transform)
6. **Day 6-7**: LR & metrics (cyclical_lr, cosine_embedding_loss, cross_product)

### Expected Outcomes
- **Coverage**: 184 → 199 operations (58.6% → 63.4%)
- **Capabilities Added**:
  - ✅ Advanced CNN architectures (DeepLab, MobileNet)
  - ✅ Medical imaging complete (dice loss, elastic transform)
  - ✅ Quantization pipeline complete (quantize + dequantize + fake_quantize)
  - ✅ Flash attention (memory-efficient transformers)
  - ✅ Modern augmentation (cutmix)
  - ✅ Metric learning (cosine embedding loss)
  - ✅ 3D geometry operations (cross product)

### Quality Targets
- All operations follow canonical WGSL pattern
- Comprehensive tests (5+ per operation)
- Clean compilation (0 errors)
- Documentation for each operation

## Operations Replaced
- ❌ `focal_loss` - Already has WGSL
- ❌ `depthwise_conv2d` - Already has WGSL  
- ❌ `causal_mask`, `cosine_similarity`, `cosine_annealing` - Files don't exist
- ✅ Replaced with: `flash_attention`, `fake_quantize`

## Validation Strategy
After implementation:
1. Run full test suite
2. Verify all 15 operations passing
3. Update coverage metrics
4. Document sprint completion

## Next Session Prep
- Week 5 target: 213 operations (67.8% coverage)
- Focus: Graph operations, advanced losses, 3D ops
- Continue 15 ops/week velocity

---

*Sprint: Week 4*  
*Target: 199/314 operations (63.4%)*  
*Operations: 15 high-value additions*  
*Focus: CNN advanced, flash attention, quantization, medical imaging*
