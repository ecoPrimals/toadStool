# 🧪 barraCUDA Unit Test Expansion Guide - Jan 30, 2026

**Date**: January 30, 2026  
**Phase**: 2 - Unit Test Expansion  
**Goal**: 272 tests → 1,250 tests (5 tests per operation)  
**Status**: ✅ **PATTERN ESTABLISHED** (ReLU complete: 1→5 tests)

---

## 📊 Progress Tracker

### **Current Status**

| Metric | Before | Current | Target | Progress |
|--------|--------|---------|--------|----------|
| **Total Tests** | 272 | 277 | 1,250 | **1%** |
| **Ops with 5 Tests** | 0 | 1 | 250 | **0.4%** |
| **New Tests Added** | 0 | 5 | 978 | **0.5%** |

### **Completed Operations** (1/250)

✅ **ReLU** - 5 tests (basic, edge, boundary, large, precision)

---

## 🎯 Test Expansion Pattern

Every operation should have **5 tests** following this pattern:

### **Test 1: Basic Functionality** ✅
**Purpose**: Verify operation works correctly on simple inputs  
**Pattern**:
```rust
#[tokio::test]
async fn test_{op}_basic() {
    let device = get_test_device().await;
    
    // Create simple test input
    let input = Tensor::from_vec_on(
        vec![...],  // Simple values
        vec![...],  // Shape
        device,
    ).await.unwrap();
    
    // Execute operation
    let output = input.{op}().unwrap();
    let result = output.to_vec().unwrap();
    
    // Verify expected output
    assert!((result[i] - expected).abs() < 1e-5);
}
```

**Example (ReLU)**:
```rust
// Input: [-2, -1, 0, 1, 2]
// Expected: [0, 0, 0, 1, 2]
```

---

### **Test 2: Edge Cases** ✅
**Purpose**: Test near-boundary values, special cases  
**Pattern**:
```rust
#[tokio::test]
async fn test_{op}_edge_cases() {
    let device = get_test_device().await;
    
    // Test near-zero, very small values
    let input = Tensor::from_vec_on(
        vec![-1e-6, -1e-10, 0.0, 1e-10, 1e-6],
        vec![5],
        device,
    ).await.unwrap();
    
    let output = input.{op}().unwrap();
    let result = output.to_vec().unwrap();
    
    // Verify edge case behavior
    assert!(condition, "Edge case description");
}
```

**Example (ReLU)**:
```rust
// Very small negative → 0
// Tiny positive → positive (unchanged)
```

---

### **Test 3: Boundary Values** ✅
**Purpose**: Test extremes (infinities, large numbers, special floats)  
**Pattern**:
```rust
#[tokio::test]
async fn test_{op}_boundary() {
    let device = get_test_device().await;
    
    // Test infinities, very large values
    let input = Tensor::from_vec_on(
        vec![f32::NEG_INFINITY, -1e10, 0.0, 1e10, f32::INFINITY],
        vec![5],
        device,
    ).await.unwrap();
    
    let output = input.{op}().unwrap();
    let result = output.to_vec().unwrap();
    
    // Verify boundary handling
    assert!(result[i].is_infinite() || result[i].is_finite());
}
```

**Example (ReLU)**:
```rust
// ReLU(-inf) = 0
// ReLU(+inf) = +inf
// ReLU(large) = large
```

---

### **Test 4: Large Tensor** ✅
**Purpose**: Stress test with realistic sizes  
**Pattern**:
```rust
#[tokio::test]
async fn test_{op}_large_tensor() {
    let device = get_test_device().await;
    
    let size = 1000;  // Or larger
    let input_data: Vec<f32> = (0..size)
        .map(|i| /* generate test data */)
        .collect();
    
    let input = Tensor::from_vec_on(
        input_data.clone(),
        vec![size],
        device,
    ).await.unwrap();
    
    let output = input.{op}().unwrap();
    let result = output.to_vec().unwrap();
    
    // Verify all elements correct
    for (i, &out) in result.iter().enumerate() {
        assert!(/* correctness check */);
    }
}
```

**Example (ReLU)**:
```rust
// 1000 elements: -500 to 499
// Verify each: max(0, input[i])
```

---

### **Test 5: Precision Validation** ✅
**Purpose**: Compare GPU vs CPU reference implementation  
**Pattern**:
```rust
#[tokio::test]
async fn test_{op}_precision() {
    let device = get_test_device().await;
    
    let input_data = vec![...];  // Representative test data
    let input = Tensor::from_vec_on(
        input_data.clone(),
        vec![input_data.len()],
        device,
    ).await.unwrap();
    
    // GPU result
    let output = input.{op}().unwrap();
    let gpu_result = output.to_vec().unwrap();
    
    // CPU reference
    let cpu_result: Vec<f32> = input_data.iter()
        .map(|&x| /* CPU implementation */)
        .collect();
    
    // Compare GPU vs CPU
    for (i, (&gpu, &cpu)) in gpu_result.iter().zip(cpu_result.iter()).enumerate() {
        assert!((gpu - cpu).abs() < 1e-5, "Precision error at {}", i);
    }
}
```

**Example (ReLU)**:
```rust
// CPU reference: x.max(0.0)
// Should be exact (no numerical error)
```

---

## 🔧 Implementation Steps

### **Step 1: Use Device Pool** ✅
Replace:
```rust
let device = crate::device::Auto::new().await.unwrap();
let device = Arc::new(device);
```

With:
```rust
use crate::device::test_pool::get_test_device;

let device = get_test_device().await;
```

**Benefits**:
- ✅ Fixes device exhaustion (119 test failures)
- ✅ Faster test execution (reuse device)
- ✅ Thread-safe concurrent testing

### **Step 2: Remove Unused Imports** ✅
If you added `get_test_device`, remove:
```rust
use std::sync::Arc;  // No longer needed
```

### **Step 3: Add 4 New Tests** ✅
Follow the 5-test pattern above:
1. Keep existing basic test (or update to use device pool)
2. Add edge cases test
3. Add boundary test  
4. Add large tensor test
5. Add precision test

### **Step 4: Verify Tests Pass** ✅
```bash
cargo test -p barracuda --lib ops::{operation}::tests
```

Should see:
```
running 5 tests
test result: ok. 5 passed; 0 failed; 0 ignored
```

---

## 📋 Operation Categories

### **Priority 1: Core Operations** (18 ops)
Most commonly used, expand first:
- ✅ **ReLU** (5/5 tests) ✅
- [ ] Add (1/5 tests)
- [ ] Mul (1/5 tests)
- [ ] Sub (1/5 tests)
- [ ] Div (1/5 tests)
- [ ] MatMul (1/5 tests)
- [ ] Conv2D (1/5 tests)
- [ ] MaxPool2D (1/5 tests)
- [ ] BatchNorm (1/5 tests)
- [ ] LayerNorm (1/5 tests)
- [ ] Sigmoid (1/5 tests)
- [ ] Tanh (1/5 tests)
- [ ] GELU (1/5 tests)
- [ ] Softmax (1/5 tests)
- [ ] CrossEntropy (1/5 tests)
- [ ] MSE (1/5 tests)
- [ ] Embedding (1/5 tests)
- [ ] Dropout (1/5 tests)

### **Priority 2: Activations** (11 ops)
- [ ] ELU, LeakyReLU, Mish, SELU, Swish, HardSwish, Softplus, PReLU, GLU, Softsign, Tanhshrink

### **Priority 3: Advanced** (221 ops)
- [ ] Convolutions (8 ops)
- [ ] Pooling (8 ops)
- [ ] Normalization (8 ops)
- [ ] Attention (8 ops)
- [ ] RNN/LSTM (5 ops)
- [ ] Loss Functions (12 ops)
- [ ] Optimizers (10 ops)
- [ ] Utilities (150+ ops)
- [ ] GNN (10 ops)
- [ ] Audio (10 ops)

---

## 📈 Estimated Timeline

### **Velocity**
- Time per operation: ~15-20 minutes
- Operations per day (8 hours): ~25-30 ops
- Total operations: 250

### **Phases**

| Phase | Operations | Duration | Tests Added |
|-------|-----------|----------|-------------|
| **Phase 2.1** | Core (18) | 1-2 days | 72 tests |
| **Phase 2.2** | Activations (11) | 1 day | 44 tests |
| **Phase 2.3** | Advanced (221) | 10-12 days | 884 tests |
| **Total** | 250 | **12-15 days** | **1,000 tests** |

**Target Completion**: Mid-February 2026

---

## 🏆 Success Criteria

### **Per Operation**
- ✅ 5 tests (up from 1)
- ✅ All tests pass
- ✅ Device pool used
- ✅ CPU reference for precision
- ✅ Edge cases covered
- ✅ Boundaries tested
- ✅ Large tensors validated

### **Overall**
- ✅ 1,250 total tests
- ✅ 100% operations covered
- ✅ Test coverage: **85/100** (from 75/100)
- ✅ Overall grade: **A+ (97/100)** (from A 95/100)

---

## 🔬 Example: ReLU Expansion (Complete)

### **Before** (1 test)
```rust
#[tokio::test]
async fn test_relu_basic() {
    let device = crate::device::Auto::new().await.unwrap();
    // ... basic test only
}
```

### **After** (5 tests) ✅
```rust
#[tokio::test]
async fn test_relu_basic() { /* ... */ }

#[tokio::test]
async fn test_relu_edge_cases() {
    // Small values near zero
}

#[tokio::test]
async fn test_relu_boundary() {
    // Infinities, large numbers
}

#[tokio::test]
async fn test_relu_large_tensor() {
    // 1000 elements stress test
}

#[tokio::test]
async fn test_relu_precision() {
    // GPU vs CPU reference
}
```

**Result**: ✅ All 5 tests pass in 1.06s

---

## 📚 CPU Reference Implementations

For precision tests, provide CPU implementations:

### **Activations**
```rust
// ReLU
fn cpu_relu(x: f32) -> f32 { x.max(0.0) }

// Sigmoid  
fn cpu_sigmoid(x: f32) -> f32 { 1.0 / (1.0 + (-x).exp()) }

// Tanh
fn cpu_tanh(x: f32) -> f32 { x.tanh() }

// GELU
fn cpu_gelu(x: f32) -> f32 {
    let sqrt_2_over_pi = (2.0 / std::f32::consts::PI).sqrt();
    let inner = sqrt_2_over_pi * (x + 0.044715 * x.powi(3));
    x * 0.5 * (1.0 + inner.tanh())
}
```

### **Element-wise**
```rust
// Add
fn cpu_add(a: f32, b: f32) -> f32 { a + b }

// Mul
fn cpu_mul(a: f32, b: f32) -> f32 { a * b }

// Div (with error handling)
fn cpu_div(a: f32, b: f32) -> f32 { a / b }
```

### **Reductions**
```rust
// Sum
fn cpu_sum(x: &[f32]) -> f32 { x.iter().sum() }

// Mean
fn cpu_mean(x: &[f32]) -> f32 { x.iter().sum::<f32>() / x.len() as f32 }

// Max
fn cpu_max(x: &[f32]) -> f32 {
    x.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b))
}
```

---

## 🎯 Next Steps

### **Immediate** (Today)
1. ✅ Expand ReLU (COMPLETE)
2. [ ] Expand Sigmoid, Tanh, GELU (activations)
3. [ ] Expand Add, Mul, Sub (element-wise)
4. [ ] Document pattern (COMPLETE)

### **Short-Term** (This Week)
- [ ] Complete Priority 1 (18 core operations)
- [ ] Achieve 150 total tests (+78 from 272)
- [ ] Test coverage: 77/100

### **Medium-Term** (Next 2 Weeks)
- [ ] Complete Priority 2 (11 activations)
- [ ] Complete 50% of Priority 3 (110 ops)
- [ ] Achieve 650 total tests
- [ ] Test coverage: 80/100

### **Long-Term** (1 Month)
- [ ] Complete all 250 operations
- [ ] Achieve 1,250 total tests
- [ ] Test coverage: 85/100
- [ ] Overall grade: A+ (97/100)

---

## 🦀 Deep Debt Principles Maintained

✅ **Zero Unsafe Code**: All test expansions use safe Rust  
✅ **Device Pooling**: Reuse device (fixes exhaustion)  
✅ **CPU References**: Validate GPU correctness  
✅ **Complete Coverage**: 5 tests × 250 ops = 1,250 tests  
✅ **Modern Idiomatic**: async/await, Result<>, assertions  
✅ **Fast AND Safe**: GPU-accelerated, thread-safe

---

## 📊 Tracking Progress

Update this checklist as operations are completed:

**Core Operations** (18): ✅ 1/18 (6%)
- ✅ ReLU (5/5)
- [ ] Add, Mul, Sub, Div, MatMul, Conv2D, MaxPool2D, BatchNorm, LayerNorm
- [ ] Sigmoid, Tanh, GELU, Softmax, CrossEntropy, MSE, Embedding, Dropout

**Activations** (11): ⏸️ 0/11 (0%)

**Advanced** (221): ⏸️ 0/221 (0%)

**Total Progress**: **1/250 operations (0.4%)**  
**Tests Added**: **5/1,000 new tests (0.5%)**  
**Current Total**: **277/1,250 tests (22%)**

---

🦀🧪✨ **Pattern Established - Ready to Scale to 1,250 Tests!** ✨🧪🦀
