# 🦈 barraCUDA Phase 3 - Reduction & Activation Operations Plan

**Date**: January 30, 2026  
**Current Status**: 35 operations (1.75% CUDA parity)  
**Phase 3 Goal**: 50 operations (2.5% CUDA parity)  
**Target**: +15 operations (+43% growth)

---

## 🎯 Mission: Complete Core ML Operations

### Current State
- ✅ Week 1: Safety First (A+ error handling)
- ✅ Phase 1: 7 neuromorphic operations (neuromorphic ready)
- ✅ Phase 2: 10 core ML operations (shape + math)
- ✅ 35 operations total (1.75% CUDA parity)

### Phase 3 Goal
- 🎯 Add 15 reduction & activation operations
- 🎯 Reach 50 total operations (2.5% CUDA parity)
- 🎯 Maintain A+ quality (Pure Rust, zero unsafe)
- 🎯 Complete core ML toolkit

---

## 📋 Phase 3 Operations (15 ops)

### Category 1: Reduction Operations (9 ops)

Reduction operations aggregate data along specified dimensions - essential for statistics, normalization, and loss functions.

#### 1. **Sum** ➕
**Purpose**: Sum elements along axis

**Use Cases**:
- Gradient aggregation
- Loss computation
- Feature pooling
- Statistics

**API**:
```rust
pub fn sum(data: &[f32], shape: &[usize], axis: Option<usize>) -> Result<Vec<f32>>
// axis=None: sum all elements → scalar
// axis=Some(i): sum along dimension i
```

**Example**:
```rust
// [[1,2,3], [4,5,6]] sum axis=0 → [5, 7, 9]
// [[1,2,3], [4,5,6]] sum axis=1 → [6, 15]
```

**Priority**: High (fundamental operation)

---

#### 2. **Mean** 📊
**Purpose**: Average elements along axis

**Use Cases**:
- Batch normalization
- Statistics
- Feature averaging
- Loss functions

**API**:
```rust
pub fn mean(data: &[f32], shape: &[usize], axis: Option<usize>) -> Result<Vec<f32>>
```

**Example**:
```rust
// [1,2,3,4,5,6] → 3.5 (mean all)
// [[1,2], [3,4], [5,6]] axis=0 → [3, 4] (column means)
```

**Priority**: High (normalization essential)

---

#### 3. **Max** ⬆️
**Purpose**: Maximum value along axis

**Use Cases**:
- Max pooling
- Feature selection
- Gradient clipping checks
- Value ranges

**API**:
```rust
pub fn max(data: &[f32], shape: &[usize], axis: Option<usize>) -> Result<Vec<f32>>
```

**Example**:
```rust
// [1, 5, 3, 2, 4] → 5
// [[1,2], [3,4]] axis=1 → [2, 4]
```

**Priority**: High (pooling operations)

---

#### 4. **Min** ⬇️
**Purpose**: Minimum value along axis

**Use Cases**:
- Min pooling
- Value ranges
- Outlier detection
- Bounds checking

**API**:
```rust
pub fn min(data: &[f32], shape: &[usize], axis: Option<usize>) -> Result<Vec<f32>>
```

**Priority**: High (pairs with Max)

---

#### 5. **Prod** ✖️
**Purpose**: Product of elements along axis

**Use Cases**:
- Probability multiplication
- Geometric mean
- Special math operations

**API**:
```rust
pub fn prod(data: &[f32], shape: &[usize], axis: Option<usize>) -> Result<Vec<f32>>
```

**Example**:
```rust
// [2, 3, 4] → 24
```

**Priority**: Medium (less common)

---

#### 6. **Var (Variance)** 📈
**Purpose**: Variance along axis

**Use Cases**:
- Statistics
- Normalization (LayerNorm, BatchNorm)
- Uncertainty quantification

**API**:
```rust
pub fn var(data: &[f32], shape: &[usize], axis: Option<usize>) -> Result<Vec<f32>>
```

**Formula**: Var = E[(X - μ)²]

**Priority**: High (normalization essential)

---

#### 7. **Std (Standard Deviation)** 📉
**Purpose**: Standard deviation along axis

**Use Cases**:
- Normalization
- Z-score computation
- Data scaling

**API**:
```rust
pub fn std(data: &[f32], shape: &[usize], axis: Option<usize>) -> Result<Vec<f32>>
```

**Formula**: Std = √Var

**Priority**: High (normalization essential)

---

#### 8. **Norm (L1/L2)** 📏
**Purpose**: Vector norms

**Use Cases**:
- Gradient clipping (L2 norm)
- Regularization
- Distance metrics
- Vector normalization

**API**:
```rust
pub fn norm(data: &[f32], p: f32) -> f32
// p=1: L1 norm (sum of abs)
// p=2: L2 norm (Euclidean)
```

**Priority**: High (gradient operations)

---

#### 9. **Cumsum** 🔢
**Purpose**: Cumulative sum

**Use Cases**:
- Prefix sums
- Cumulative statistics
- Integration approximation
- Sequence operations

**API**:
```rust
pub fn cumsum(data: &[f32], axis: usize) -> Result<Vec<f32>>
```

**Example**:
```rust
// [1, 2, 3, 4] → [1, 3, 6, 10]
```

**Priority**: Medium (sequence operations)

---

### Category 2: Activation Functions (4 ops)

#### 10. **Softmax** 🎲
**Purpose**: Normalize to probability distribution

**Use Cases**:
- Classification output
- Attention mechanisms
- Probability computation
- Neuromorphic decision

**API**:
```rust
pub fn softmax(data: &[f32], axis: usize) -> Result<Vec<f32>>
```

**Formula**: softmax(xi) = exp(xi) / Σ exp(xj)

**Priority**: HIGH (classification essential)

---

#### 11. **LogSoftmax** 📝
**Purpose**: Log of softmax (numerically stable)

**Use Cases**:
- Cross-entropy loss
- Log probability
- Stable softmax
- Training optimization

**API**:
```rust
pub fn log_softmax(data: &[f32], axis: usize) -> Result<Vec<f32>>
```

**Priority**: HIGH (training essential)

---

#### 12. **ReLU** ⚡
**Purpose**: Rectified Linear Unit (max(0, x))

**Use Cases**:
- Most common activation
- Hidden layer activation
- Non-linearity
- Neuromorphic processing

**API**:
```rust
pub fn relu(data: &[f32]) -> Vec<f32>
```

**Priority**: HIGH (ubiquitous in NNs)

---

#### 13. **GELU** 🌊
**Purpose**: Gaussian Error Linear Unit

**Use Cases**:
- Transformer models
- Modern architectures
- Smooth activation
- Better gradients

**API**:
```rust
pub fn gelu(data: &[f32]) -> Vec<f32>
```

**Formula**: GELU(x) = x * Φ(x) (Φ = Gaussian CDF)

**Priority**: HIGH (transformers)

---

### Category 3: Normalization (2 ops)

#### 14. **LayerNorm** 🔄
**Purpose**: Normalize across features

**Use Cases**:
- Transformer layers
- Modern architectures
- Training stability
- Feature normalization

**API**:
```rust
pub fn layer_norm(data: &[f32], shape: &[usize], eps: f32) -> Result<Vec<f32>>
```

**Formula**: (x - μ) / √(σ² + ε)

**Priority**: HIGH (transformers)

---

#### 15. **Sigmoid** 📉
**Purpose**: Logistic activation (0 to 1)

**Use Cases**:
- Binary classification
- Gate functions (LSTM, GRU)
- Probability output
- Legacy networks

**API**:
```rust
pub fn sigmoid(data: &[f32]) -> Vec<f32>
```

**Formula**: σ(x) = 1 / (1 + e^(-x))

**Priority**: HIGH (gates, classification)

---

## 📊 Operation Priorities

### Critical Priority (10 ops) - Implement First
1. ✅ Sum (reduction foundation)
2. ✅ Mean (normalization)
3. ✅ Max (pooling)
4. ✅ Min (pooling)
5. ✅ Var (normalization)
6. ✅ Std (normalization)
7. ✅ Softmax (classification)
8. ✅ ReLU (activation)
9. ✅ GELU (modern activation)
10. ✅ Sigmoid (gates)

### High Priority (3 ops) - Implement Second
11. ✅ LogSoftmax (training)
12. ✅ LayerNorm (transformers)
13. ✅ Norm (gradient clipping)

### Medium Priority (2 ops) - Implement Last
14. ⏳ Prod (less common)
15. ⏳ Cumsum (sequences)

---

## 🏗️ Implementation Strategy

### Approach

**1. Reduction Operations First (Sum, Mean, Max, Min)**
- Foundation for all statistics
- Used by other operations
- 4 operations, ~200 LOC

**2. Statistics (Var, Std, Norm)**
- Build on Sum/Mean
- Essential for normalization
- 3 operations, ~150 LOC

**3. Activations (ReLU, GELU, Sigmoid)**
- Simple element-wise
- Quick wins
- 3 operations, ~100 LOC

**4. Advanced (Softmax, LogSoftmax, LayerNorm)**
- Combine reductions + activations
- More complex
- 3 operations, ~200 LOC

**5. Sequences (Cumsum, Prod)**
- Lower priority
- 2 operations, ~100 LOC

### Quality Standards (A+ Required)

**Code Quality**:
- ✅ Pure Rust (zero unsafe)
- ✅ BarracudaError for all errors
- ✅ Axis validation (dimension checks)
- ✅ NaN/Inf handling
- ✅ No panics possible

**Testing**:
- ✅ Unit tests per operation
- ✅ Axis reduction tests
- ✅ Edge cases (empty, single element)
- ✅ Numerical stability tests

**Documentation**:
- ✅ Clear mathematical formulas
- ✅ Use case examples
- ✅ API documentation
- ✅ Performance notes

---

## 🧠 Neuromorphic & ML Alignment

### Why These Operations?

**Training Pipeline**:
- Mean/Var/Std: Normalization (BatchNorm, LayerNorm)
- Sum: Loss aggregation, gradient accumulation
- Max/Min: Pooling operations

**Inference Pipeline**:
- ReLU/GELU/Sigmoid: Activations
- Softmax: Classification output
- LayerNorm: Transformer layers

**Neuromorphic Processing**:
- Softmax: Decision probability
- ReLU: Sparse activation
- Sum/Mean: Feature aggregation
- Max: Winner-take-all circuits

**Result**: Complete training + inference + neuromorphic stack

---

## 📈 Progress Tracking

### Current Status
- **Operations**: 35
- **CUDA Parity**: 1.75%
- **Grade**: A+

### Phase 3 Target
- **Operations**: 50 (+15)
- **CUDA Parity**: 2.5% (+0.75%)
- **Grade**: A+ (maintained)
- **Growth**: +43%

### Future Phases
- **Phase 4**: 75 operations (3.75% parity)
- **Phase 5**: 100 operations (5% parity)
- **Goal**: 400 operations (20% parity)

---

## 🎯 Success Criteria

### Code Metrics
- [ ] 15 new operations implemented
- [ ] ~750-850 LOC production code
- [ ] ~350-400 LOC tests
- [ ] Zero compilation errors
- [ ] A+ quality maintained

### Quality Metrics
- [ ] 100% Pure Rust (zero unsafe)
- [ ] BarracudaError throughout
- [ ] Comprehensive tests (45+ test functions)
- [ ] Full documentation
- [ ] Numerical stability verified

### Integration
- [ ] New module or extend tensor_ops.rs
- [ ] Public API exported
- [ ] Examples added
- [ ] Documentation updated

---

## 📊 Expected Impact

### Operation Count Evolution

| Phase | Operations | CUDA Parity | Growth |
|-------|------------|-------------|--------|
| Baseline | 18 | 0.9% | - |
| Phase 1 | 25 | 1.25% | +39% |
| Phase 2 | 35 | 1.75% | +40% |
| **Phase 3** | **50** | **2.5%** | **+43%** |
| Target | 400 | 20% | +2,100% |

### Code Volume

| Component | Current | Phase 3 | Total |
|-----------|---------|---------|-------|
| tensor_ops.rs | 1,513 | +800 | 2,313 |
| Tests | ~520 | +380 | ~900 |
| Docs | ~290 | +120 | ~410 |
| **Total** | **2,323** | **+1,300** | **3,623** |

---

## 🚀 Implementation Timeline

### Estimated Effort
- **Reductions (9)**: 4-5 hours
- **Activations (4)**: 2 hours
- **Normalization (2)**: 2 hours
- **Testing**: 2 hours
- **Documentation**: 1 hour
- **Total**: ~11-12 hours (1.5-2 days)

### Milestones
1. ✅ Plan complete
2. ⏳ Basic reductions (Sum, Mean, Max, Min)
3. ⏳ Statistics (Var, Std, Norm)
4. ⏳ Activations (ReLU, GELU, Sigmoid)
5. ⏳ Advanced (Softmax, LogSoftmax, LayerNorm)
6. ⏳ Sequences (Cumsum, Prod)
7. ⏳ All tests passing
8. ⏳ Documentation complete

---

## 💡 Design Patterns

### Pattern 1: Axis Reduction Template
```rust
pub fn reduce_op(
    data: &[f32],
    shape: &[usize],
    axis: Option<usize>,
) -> Result<Vec<f32>> {
    // Validate
    validate_axis(shape, axis)?;
    
    // Reduce all if no axis specified
    if axis.is_none() {
        return Ok(vec![reduce_all(data)]);
    }
    
    // Reduce along axis
    let axis = axis.unwrap();
    let output_shape = compute_output_shape(shape, axis);
    let output = reduce_along_axis(data, shape, axis);
    
    Ok(output)
}
```

### Pattern 2: Numerical Stability
```rust
// Softmax with log-sum-exp trick
pub fn softmax(data: &[f32]) -> Vec<f32> {
    // Subtract max for numerical stability
    let max_val = data.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let exp_vals: Vec<f32> = data.iter().map(|&x| (x - max_val).exp()).collect();
    let sum: f32 = exp_vals.iter().sum();
    exp_vals.iter().map(|&x| x / sum).collect()
}
```

### Pattern 3: Activation Template
```rust
// Simple element-wise activation
pub fn activation(data: &[f32]) -> Vec<f32> {
    data.iter().map(|&x| activation_fn(x)).collect()
}

// Inline for performance
#[inline]
fn activation_fn(x: f32) -> f32 {
    // ... implementation
}
```

---

## 📝 Module Structure

### Option A: Extend tensor_ops.rs
```rust
// Add to existing file (currently 1,513 LOC)
// Reduction operations
pub struct Sum;
pub struct Mean;
// ... etc
```

**Pros**: All tensor ops together  
**Cons**: File gets large (2,313 LOC)

### Option B: New reduction_ops.rs
```rust
// New file for reductions
pub struct Sum;
pub struct Mean;
pub struct Var;
pub struct Std;
// ... etc
```

**Pros**: Logical separation  
**Cons**: More files

### Option C: Split by Category
```rust
// reduction_ops.rs - Sum, Mean, Max, Min, Var, Std, Norm, Cumsum, Prod
// activation_ops.rs - ReLU, GELU, Sigmoid, Softmax, LogSoftmax
// normalization_ops.rs - LayerNorm, BatchNorm
```

**Pros**: Clear organization  
**Cons**: Many files

**Decision**: **Option A** (extend tensor_ops.rs) - Keep cohesive, refactor if > 3000 LOC

---

## 🎊 Summary

### Phase 3 Plan

**Goal**: Add 15 reduction & activation operations (35 → 50)

**Operations** (by priority):
1. Sum, Mean, Max, Min (reductions)
2. Var, Std, Norm (statistics)
3. ReLU, GELU, Sigmoid (activations)
4. Softmax, LogSoftmax, LayerNorm (advanced)
5. Cumsum, Prod (sequences)

**Standards**:
- Pure Rust, zero unsafe
- BarracudaError throughout
- Comprehensive tests (45+ functions)
- A+ quality maintained

**Impact**:
- +43% operation growth
- +0.75% CUDA parity
- Complete core ML operations
- Full training + inference stack

**Timeline**: 1.5-2 days focused work

---

**Date**: January 30, 2026  
**Status**: ✅ Plan Complete, Ready to Execute  
**Next**: Implement reduction operations (Sum, Mean, Max, Min)

🦈 **Phase 3: Expanding barraCUDA to 50 operations with core ML stack!** 📈
