# 🦈 barraCUDA Phase 2 - Operations Expansion Plan

**Date**: January 30, 2026  
**Current Status**: 25 operations (1.25% CUDA parity)  
**Phase 2 Goal**: 35 operations (1.75% CUDA parity)  
**Target**: +10 operations (+40% growth)

---

## 🎯 Mission: Expand Core Tensor Operations

### Current State
- ✅ Phase 1: 25 operations (neuromorphic ready)
- ✅ Week 1: Safety First complete (A+ grade)
- ✅ Core infrastructure: Error handling, testing patterns

### Phase 2 Goal
- 🎯 Add 10 essential tensor operations
- 🎯 Reach 35 total operations (1.75% CUDA parity)
- 🎯 Maintain A+ quality (Pure Rust, zero unsafe, proper errors)
- 🎯 Focus on ML/neuromorphic workflows

---

## 📋 Phase 2 Operations (10 ops)

### Category 1: Shape Manipulation (4 ops)

#### 1. **Transpose** 🔄
**Purpose**: Swap tensor dimensions

**Use Cases**:
- Matrix transpose (rows ↔ columns)
- NCHW ↔ NHWC conversion
- Batched matrix operations
- Neuromorphic data format conversion

**API**:
```rust
pub fn transpose(data: &[f32], shape: &[usize], dim0: usize, dim1: usize) -> Result<Vec<f32>>
```

**Example**:
```rust
// Transpose 2D: [[1,2,3], [4,5,6]] → [[1,4], [2,5], [3,6]]
Transpose::execute(&data, &[2, 3], 0, 1)?;
```

**Priority**: High (essential for ML)

---

#### 2. **Squeeze** 🗜️
**Purpose**: Remove dimensions of size 1

**Use Cases**:
- Simplify tensor shapes
- Remove batch dimension (1, H, W) → (H, W)
- Model output cleanup
- Neuromorphic output formatting

**API**:
```rust
pub fn squeeze(data: &[f32], shape: &[usize], dim: Option<usize>) -> Result<Vec<f32>>
// dim=None: remove all size-1 dims
// dim=Some(i): remove specific dim if size is 1
```

**Example**:
```rust
// [1, 3, 1, 4] → [3, 4]
Squeeze::execute(&data, &[1, 3, 1, 4], None)?;
```

**Priority**: High (common operation)

---

#### 3. **Unsqueeze** 📦
**Purpose**: Add dimensions of size 1

**Use Cases**:
- Add batch dimension
- Prepare for broadcasting
- Shape alignment
- Neuromorphic input formatting

**API**:
```rust
pub fn unsqueeze(data: &[f32], shape: &[usize], dim: usize) -> Result<Vec<f32>>
```

**Example**:
```rust
// [3, 4] → [1, 3, 4] (add batch dim)
Unsqueeze::execute(&data, &[3, 4], 0)?;
```

**Priority**: High (pairs with Squeeze)

---

#### 4. **Expand** 📏
**Purpose**: Broadcast tensor to larger shape

**Use Cases**:
- Broadcasting for element-wise ops
- Tile/repeat data
- Batch processing
- Feature replication

**API**:
```rust
pub fn expand(data: &[f32], shape: &[usize], target_shape: &[usize]) -> Result<Vec<f32>>
```

**Example**:
```rust
// [1, 3] → [4, 3] (broadcast batch)
Expand::execute(&data, &[1, 3], &[4, 3])?;
```

**Priority**: Medium (useful for efficiency)

---

### Category 2: Element-wise Operations (3 ops)

#### 5. **Where (Conditional Select)** ❓
**Purpose**: Select elements based on condition

**Use Cases**:
- Conditional masking
- ReLU variants (max(0, x))
- Thresholding
- Neuromorphic decision logic

**API**:
```rust
pub fn where_op(
    condition: &[bool],
    true_vals: &[f32],
    false_vals: &[f32]
) -> Result<Vec<f32>>
```

**Example**:
```rust
// condition ? x : y
Where::execute(&[true, false, true], &[1.0, 2.0, 3.0], &[0.0, 0.0, 0.0])?;
// → [1.0, 0.0, 3.0]
```

**Priority**: High (very common)

---

#### 6. **Clamp (Clip)** ✂️
**Purpose**: Constrain values to range

**Use Cases**:
- Gradient clipping
- Value normalization
- Quantization bounds
- Neuromorphic range control

**API**:
```rust
pub fn clamp(data: &[f32], min: f32, max: f32) -> Result<Vec<f32>>
```

**Example**:
```rust
// Clamp [-1, 5, 3] to [0, 3] → [0, 3, 3]
Clamp::execute(&data, 0.0, 3.0)?;
```

**Priority**: High (gradient clipping essential)

---

#### 7. **Abs (Absolute Value)** |x|
**Purpose**: Compute absolute value

**Use Cases**:
- Distance calculations
- Loss functions (L1)
- Feature normalization
- Signal processing

**API**:
```rust
pub fn abs(data: &[f32]) -> Vec<f32>
```

**Example**:
```rust
// [-1, 2, -3] → [1, 2, 3]
Abs::execute(&data);
```

**Priority**: High (simple, essential)

---

### Category 3: Mathematical Operations (3 ops)

#### 8. **Sqrt (Square Root)** √x
**Purpose**: Element-wise square root

**Use Cases**:
- Standard deviation
- Distance metrics (Euclidean)
- Normalization (LayerNorm)
- Gradient operations

**API**:
```rust
pub fn sqrt(data: &[f32]) -> Result<Vec<f32>>
```

**Example**:
```rust
// [1, 4, 9] → [1, 2, 3]
Sqrt::execute(&data)?;
```

**Error Handling**: Negative inputs → NaN or error

**Priority**: High (normalization operations)

---

#### 9. **Pow (Exponentiation)** x^n
**Purpose**: Raise to power

**Use Cases**:
- Polynomial operations
- Variance calculation (x²)
- Custom activations
- Loss functions

**API**:
```rust
pub fn pow(data: &[f32], exponent: f32) -> Vec<f32>
```

**Example**:
```rust
// [2, 3, 4] ^ 2 → [4, 9, 16]
Pow::execute(&data, 2.0);
```

**Priority**: Medium (variance, MSE)

---

#### 10. **Exp (Exponential)** e^x
**Purpose**: Element-wise exponential

**Use Cases**:
- Softmax activation
- Gaussian functions
- Probability calculations
- Neuromorphic activation

**API**:
```rust
pub fn exp(data: &[f32]) -> Vec<f32>
```

**Example**:
```rust
// [0, 1, 2] → [1.0, 2.718, 7.389]
Exp::execute(&data);
```

**Priority**: High (softmax essential)

---

## 📊 Operation Priorities

### High Priority (7 ops) - Implement First
1. ✅ Transpose (shape manipulation)
2. ✅ Squeeze (shape cleanup)
3. ✅ Unsqueeze (shape preparation)
4. ✅ Where (conditional logic)
5. ✅ Clamp (gradient clipping)
6. ✅ Abs (basic math)
7. ✅ Sqrt (normalization)
8. ✅ Exp (softmax)

### Medium Priority (2 ops) - Implement Second
9. ⏳ Expand (broadcasting)
10. ⏳ Pow (variance)

---

## 🏗️ Implementation Strategy

### Approach

**1. Shape Operations First**
- Transpose, Squeeze, Unsqueeze
- Core infrastructure for shape manipulation
- Used by many other operations

**2. Element-wise Next**
- Where, Clamp, Abs
- Simple, independent operations
- Quick wins for operation count

**3. Math Operations Last**
- Sqrt, Exp, Pow
- Enable advanced ML features
- Complete core toolkit

### Quality Standards (A+ Required)

**Code Quality**:
- ✅ Pure Rust (zero unsafe)
- ✅ Comprehensive error handling (BarracudaError)
- ✅ Input validation (self-knowledge)
- ✅ Rich error context
- ✅ No panics possible

**Testing**:
- ✅ Unit tests per operation
- ✅ Edge case coverage
- ✅ Shape validation tests
- ✅ Error condition tests

**Documentation**:
- ✅ Clear purpose
- ✅ Use case examples
- ✅ API documentation
- ✅ Neuromorphic alignment

---

## 🧠 Neuromorphic Alignment

### Why These Operations?

**Preprocessing Pipeline**:
- Transpose: Format conversion (NHWC ↔ NCHW)
- Squeeze/Unsqueeze: Dimension management
- Clamp: Quantization bounds

**Processing**:
- Where: Conditional routing
- Abs: Signal normalization
- Sqrt: Feature scaling

**Postprocessing**:
- Exp: Softmax (classification)
- Pow: Confidence scoring
- Expand: Batch replication

**Result**: Complete ML preprocessing + inference + postprocessing stack

---

## 📈 Progress Tracking

### Current Status
- **Operations**: 25
- **CUDA Parity**: 1.25%
- **Grade**: A+

### Phase 2 Target
- **Operations**: 35 (+10)
- **CUDA Parity**: 1.75% (+0.5%)
- **Grade**: A+ (maintained)
- **Growth**: +40%

### Future Phases
- **Phase 3**: 50 operations (2.5% parity)
- **Phase 4**: 100 operations (5% parity)
- **Phase 5**: 400 operations (20% parity)

---

## 🎯 Success Criteria

### Code Metrics
- [ ] 10 new operations implemented
- [ ] ~600-800 LOC production code
- [ ] ~300-400 LOC tests
- [ ] Zero compilation errors
- [ ] A+ quality maintained

### Quality Metrics
- [ ] 100% Pure Rust (zero unsafe)
- [ ] BarracudaError throughout
- [ ] Comprehensive tests (30+ test functions)
- [ ] Full documentation
- [ ] Zero production panics possible

### Integration
- [ ] tensor_ops.rs extended or new module
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
| **Phase 2** | **35** | **1.75%** | **+40%** |
| Phase 3 | 50 | 2.5% | +43% |
| Target | 400 | 20% | +1,500% |

### Code Volume

| Component | Current | Phase 2 | Total |
|-----------|---------|---------|-------|
| tensor_ops.rs | 770 | +700 | 1,470 |
| Tests | ~200 | +350 | ~550 |
| Docs | ~220 | +150 | ~370 |
| **Total** | **1,190** | **+1,200** | **2,390** |

---

## 🚀 Implementation Timeline

### Estimated Effort
- **Shape ops (4)**: 3-4 hours
- **Element-wise (3)**: 2-3 hours
- **Math ops (3)**: 2-3 hours
- **Testing**: 2 hours
- **Documentation**: 1 hour
- **Total**: ~10-12 hours (1-2 days)

### Milestones
1. ✅ Plan complete
2. ⏳ Shape operations (4 ops)
3. ⏳ Element-wise operations (3 ops)
4. ⏳ Math operations (3 ops)
5. ⏳ All tests passing
6. ⏳ Documentation complete

---

## 💡 Design Patterns

### Pattern 1: Zero-Copy When Possible
```rust
// Operations that don't change data, only metadata
pub fn squeeze(data: &[f32], shape: &[usize]) -> Result<Vec<f32>> {
    validate_shape(shape)?;
    Ok(data.to_vec())  // Zero-copy in simple case
}
```

### Pattern 2: SIMD-Friendly
```rust
// Element-wise operations optimize easily
pub fn abs(data: &[f32]) -> Vec<f32> {
    data.iter().map(|&x| x.abs()).collect()  // Auto-vectorizes
}
```

### Pattern 3: Rich Validation
```rust
// Always validate before processing
fn validate_transpose(shape: &[usize], dim0: usize, dim1: usize) -> Result<()> {
    if dim0 >= shape.len() || dim1 >= shape.len() {
        return Err(BarracudaError::invalid_params(
            "Transpose",
            format!("Dims {} and {} invalid for shape {:?}", dim0, dim1, shape)
        ));
    }
    Ok(())
}
```

---

## 📝 Module Structure

### Option A: Extend tensor_ops.rs
```rust
// Add to existing file
pub struct Transpose;
pub struct Squeeze;
// ... etc
```

**Pros**: Keeps related ops together  
**Cons**: File gets large (1,470 LOC)

### Option B: Create tensor_ops2.rs
```rust
// New file for Phase 2
pub struct Transpose;
pub struct Squeeze;
// ... etc
```

**Pros**: Clean separation  
**Cons**: Arbitrary split

### Option C: Split by Category
```rust
// shape_ops.rs - Transpose, Squeeze, Unsqueeze, Expand
// element_ops.rs - Where, Clamp, Abs
// math_ops.rs - Sqrt, Pow, Exp
```

**Pros**: Logical grouping  
**Cons**: More files to manage

**Decision**: Start with **Option A** (extend tensor_ops.rs), refactor if > 2000 LOC

---

## 🎊 Summary

### Phase 2 Plan

**Goal**: Add 10 essential tensor operations (25 → 35)

**Operations**:
1. Transpose (shape)
2. Squeeze (shape)
3. Unsqueeze (shape)
4. Expand (shape)
5. Where (conditional)
6. Clamp (element-wise)
7. Abs (element-wise)
8. Sqrt (math)
9. Pow (math)
10. Exp (math)

**Standards**:
- Pure Rust, zero unsafe
- BarracudaError throughout
- Comprehensive tests
- A+ quality maintained

**Impact**:
- +40% operation growth
- +0.5% CUDA parity
- Complete ML core operations
- Enhanced neuromorphic support

**Timeline**: 1-2 days focused work

---

**Date**: January 30, 2026  
**Status**: ✅ Plan Complete, Ready to Execute  
**Next**: Implement shape operations (Transpose, Squeeze, Unsqueeze, Expand)

🦈 **Phase 2: Expanding barraCUDA to 35 operations!** 📈
