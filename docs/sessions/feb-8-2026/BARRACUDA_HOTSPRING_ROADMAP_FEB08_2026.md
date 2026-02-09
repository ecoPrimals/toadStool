# BarraCUDA Evolution Roadmap: hotSpring Physics Integration
## February 8, 2026 - Upstream Math Analysis

**From**: ToadStool Team  
**Re**: hotSpring physics shader handoff  
**Status**: Analysis complete, implementation roadmap ready

---

## Executive Summary

**Good News**: BarraCUDA already has ~75% of the required infrastructure.  
**Gap**: 4 new shaders + 1 composition layer closes the loop completely.  
**Timeline**: 5-10 weeks for full surrogate + MD pipeline.  
**Deep Debt Status**: All implementations will follow existing patterns (zero debt).

---

## 📊 Capability Matrix

### ✅ **Already Implemented (Ancestors)**

| Capability | Location | Status | Notes |
|------------|----------|--------|-------|
| **Pairwise Distance** | `ops/cdist_wgsl.rs` | ✅ Complete | Euclidean, Manhattan, Cosine |
| **Matrix Inverse** | `ops/inverse_wgsl.rs` | ✅ Complete | Gauss-Jordan, N≤16 optimized |
| **Exponential** | `ops/exp_wgsl.rs` | ✅ Complete | Element-wise exp |
| **Matrix Multiply** | `ops/matmul.rs` | ✅ Complete | Tiled, batched variants |
| **Sum/Mean** | `ops/sum.rs`, `ops/mean.rs` | ✅ Complete | Reduction ops |
| **MD Forces** | `ops/md/forces/*.wgsl` | ✅ Complete | Coulomb, Yukawa, LJ, Morse, Born-Mayer |
| **Velocity-Verlet** | `ops/md/integrators/velocity_verlet.rs` | ✅ Complete | Symplectic integrator |
| **PBC** | `ops/md/pbc.rs` | ✅ Complete | Periodic boundaries |
| **FFT 1D/2D/3D** | `ops/fft/*.rs` | ✅ Complete | For PPPM validation |
| **NTT/INTT** | `ops/fhe_ntt.wgsl`, `ops/fhe_intt.wgsl` | ✅ Complete | For FHE (bonus: FFT validation) |

**Coverage**: ~75% of surrogate pipeline, ~85% of MD pipeline

---

## 🎯 **Priority 1: Surrogate Learning (Missing 4)**

### 1.1 Cholesky Decomposition ❌ **NEEDS IMPLEMENTATION**

**File**: `crates/barracuda/src/ops/linalg/cholesky.wgsl` + `cholesky.rs`

**Ancestor**: `inverse_wgsl.rs` (same matrix traversal, similar structure)

**Algorithm**:
```rust
// Already have: Gauss-Jordan elimination (inverse.wgsl)
// Need: Cholesky decomposition (very similar structure)
//
// inverse.wgsl does:
//   for each row: pivot, normalize, eliminate
//
// cholesky.wgsl does:
//   for each column: compute L[j,j], update L[i>j,j]
//   Similar dependency structure (sequential in one dim, parallel in other)
```

**Implementation Strategy**:
```
Week 1-2: Cholesky
├─ Day 1-2:  Copy inverse_wgsl.rs → cholesky.rs
├─ Day 3-5:  Adapt WGSL shader (column-wise instead of row-wise)
├─ Day 6-7:  Blocked variant for large N (32×32 tiles)
├─ Day 8-9:  Test with known SPD matrices
└─ Day 10:   Integration tests (A = LLᵀ reconstruction)
```

**Size Validation**:
- hotSpring: N ≤ 30,000 (training set size)
- BarraCUDA inverse: Optimized for N ≤ 16
- **Action**: Implement blocked Cholesky for N up to 30,000
- **Fallback**: CPU via Rayon for N > 30,000 (scientific workloads often have small N anyway)

**Deep Debt Compliance**:
- ✅ Pure WGSL (follows inverse.wgsl pattern)
- ✅ Safe Rust wrapper (follows existing ops pattern)
- ✅ Hardware-agnostic (WGPU)
- ✅ No hardcoding (runtime size)

---

### 1.2 Triangular Solve ❌ **NEEDS IMPLEMENTATION**

**File**: `crates/barracuda/src/ops/linalg/triangular_solve.wgsl` + `triangular_solve.rs`

**Ancestor**: `inverse_wgsl.rs` (column operations)

**Algorithm**:
```rust
// Forward substitution: Lx = b
// x[0] = b[0] / L[0,0]
// x[i] = (b[i] - Σ L[i,j]*x[j]) / L[i,i]  for j < i
//
// This is simpler than inverse! Just accumulation + division.
```

**Implementation Strategy**:
```
Week 2-3: Triangular Solve
├─ Day 1-2:  Create triangular_solve.rs (simpler than Cholesky)
├─ Day 3-4:  WGSL shader (forward + backward sub)
├─ Day 5:    Batch mode (solve multiple RHS in parallel)
└─ Day 6-7:  Integration tests (L·solve(L,b) == b)
```

**Deep Debt Compliance**:
- ✅ Pure WGSL
- ✅ Safe Rust wrapper
- ✅ Hardware-agnostic
- ✅ Batched (solve multiple RHS efficiently)

---

### 1.3 RBF Kernel Evaluation ❌ **NEEDS IMPLEMENTATION**

**File**: `crates/barracuda/src/ops/interpolation/rbf_kernel.wgsl` + `rbf_kernel.rs`

**Ancestor**: `cdist_wgsl.rs` (pairwise distance) + `exp_wgsl.rs` + `pow_wgsl.rs`

**Algorithm**:
```rust
// Compose existing ops:
// 1. Compute pairwise distances: cdist(X, Y) → D[N×M]
// 2. Apply kernel function: φ(D) → K[N×M]
//
// Kernels:
//   thin_plate_spline: φ(r) = r² · log(r)
//   gaussian: φ(r) = exp(-ε²r²)
//   multiquadric: φ(r) = sqrt(1 + ε²r²)
//   etc.
```

**Implementation Strategy**:
```
Week 3: RBF Kernel
├─ Day 1:  Create rbf_kernel.rs
├─ Day 2:  Fused WGSL shader (distance + kernel in one pass)
├─ Day 3:  Support 5 kernel types (thin_plate_spline, gaussian, etc.)
└─ Day 4-5: Integration tests with known analytic solutions
```

**Optimization**: Fuse distance + kernel computation (avoid N×M intermediate buffer)

**Deep Debt Compliance**:
- ✅ Pure WGSL
- ✅ Safe Rust wrapper
- ✅ Hardware-agnostic
- ✅ Kernel type as enum (runtime selection)

---

### 1.4 RBF Interpolator ❌ **NEEDS IMPLEMENTATION**

**File**: `crates/barracuda/src/ops/interpolation/rbf.rs`

**Ancestor**: Composition of 1.1 + 1.2 + 1.3

**Implementation**:
```rust
pub struct RbfInterpolator {
    training_points: Tensor,  // [N×d]
    weights: Tensor,          // [N]
    kernel: RbfKernelType,
    epsilon: f32,
}

impl RbfInterpolator {
    /// Train RBF surrogate on GPU
    pub fn fit(X: &Tensor, y: &Tensor, kernel: RbfKernelType, epsilon: f32) -> Result<Self> {
        // 1. Compute kernel matrix: K[i,j] = φ(‖xᵢ - xⱼ‖)
        let K = RbfKernel::new(X, X, kernel, epsilon)?.execute()?;
        
        // 2. Cholesky decomposition: K = LLᵀ
        let L = Cholesky::new(K)?.execute()?;
        
        // 3. Solve: Lz = y, then Lᵀw = z
        let z = TriangularSolve::forward(L.clone(), y.clone())?.execute()?;
        let weights = TriangularSolve::backward(L, z)?.execute()?;
        
        Ok(Self {
            training_points: X.clone(),
            weights,
            kernel,
            epsilon,
        })
    }
    
    /// Evaluate surrogate at new points
    pub fn predict(&self, X_new: &Tensor) -> Result<Tensor> {
        // Compute kernel matrix between new points and training points
        let K = RbfKernel::new(X_new, &self.training_points, self.kernel, self.epsilon)?.execute()?;
        
        // Prediction = K·weights (existing matmul)
        K.matmul(&self.weights)
    }
}
```

**Implementation Strategy**:
```
Week 4: RBF Interpolator
├─ Day 1-2:  Implement RbfInterpolator struct
├─ Day 3:    Fit method (compose Cholesky + TriangularSolve + RbfKernel)
├─ Day 4:    Predict method (RbfKernel + matmul)
└─ Day 5:    Validation: train on sin(x), predict, error < 1e-4
```

**Validation**: 
- Train on sin(x) → predict at intermediate points → error < 1e-4
- **Critical**: Same training data as Python `scipy.interpolate.RBFInterpolator` → same predictions (< 1e-6 diff)

**Deep Debt Compliance**:
- ✅ Pure composition (no new WGSL)
- ✅ Safe Rust
- ✅ Hardware-agnostic
- ✅ Python `scipy` compatible (API parity)

---

## 🎯 **Priority 2: MD Force Pipeline (Already ~85% Complete)**

### What We Already Have ✅

| Component | Status | File |
|-----------|--------|------|
| **Coulomb Force** | ✅ | `ops/md/forces/coulomb.rs` |
| **Yukawa Force** | ✅ | `ops/md/forces/yukawa.wgsl` |
| **Lennard-Jones** | ✅ | `ops/md/forces/lennard_jones.rs` |
| **Morse** | ✅ | `ops/md/forces/morse.wgsl` |
| **Born-Mayer** | ✅ | `ops/md/forces/born_mayer.wgsl` |
| **Velocity-Verlet** | ✅ | `ops/md/integrators/velocity_verlet.rs` |
| **PBC** | ✅ | `ops/md/pbc.rs` |
| **RK4 Integrator** | ✅ | `ops/md/integrators/rk4.rs` |

### What's Missing ❌

#### 2.1 Neighbor List Construction ❌

**File**: `crates/barracuda/src/ops/md/neighbor_list.rs` + `.wgsl`

**Ancestor**: 
- `ops/histc.rs` (binning)
- `ops/argsort.wgsl` (sorting)
- `ops/searchsorted.rs` (binary search)

**Algorithm**: Cell-list (spatial hashing)
```
1. Hash particles into cells (3D grid)
2. Sort by cell index (existing argsort)
3. Build cell boundaries (existing searchsorted)
4. Each particle: check 27 neighbor cells
```

**Implementation**: 2 weeks (reuse existing primitives)

**Deep Debt**: ✅ Compose existing ops, pure WGSL

---

#### 2.2 Force Kernel Integration Test ❌

**What**: Validate all 5 force kernels match Sarkas reference data

**File**: `tests/md_forces_validation.rs`

**Data**: `sarkas/simulations/dsf-study/results/all_observables_validation.json`

**Test**:
```rust
#[test]
fn test_yukawa_matches_sarkas() {
    let positions = load_sarkas_positions("yukawa_case");
    let forces = YukawaForce::new(positions, params).execute()?;
    
    let reference = load_sarkas_forces("yukawa_case");
    assert_close(forces, reference, 1e-6);
}
```

**Timeline**: 1 week (test all 5 kernels + integrators)

---

## 🎯 **Priority 3: NPU Inference Path**

### 3.1 RBF Model Export to Akida ❌

**What**: Convert trained RBF → Akida-compatible format

**Strategy**:
```
RBF prediction = pairwise_distance(X_new, X_train) → kernel(φ) → weighted_sum(w)
                 ↓
            Equivalent to 1-hidden-layer network:
                 input → RBF_units (kernel activations) → linear (weights)
                       ↓
            Quantize to Akida format (INT8/INT4)
```

**Implementation**:
```rust
pub struct RbfAkidaExporter {
    interpolator: RbfInterpolator,
}

impl RbfAkidaExporter {
    pub fn export(&self) -> AkidaModel {
        // 1. Quantize training_points (RBF centers)
        let centers_quantized = quantize_f32_to_int8(&self.interpolator.training_points);
        
        // 2. Quantize weights
        let weights_quantized = quantize_f32_to_int8(&self.interpolator.weights);
        
        // 3. Build Akida model
        AkidaModel::new()
            .add_rbf_layer(centers_quantized, self.interpolator.kernel)
            .add_linear_layer(weights_quantized)
            .build()
    }
}
```

**Timeline**: 2 weeks

**Validation**: NPU prediction matches GPU prediction within quantization tolerance (< 1e-3)

---

### 3.2 Cross-Hardware Benchmark ❌

**File**: `showcase/whitePaper/benchmarks/cross_hardware_rbf_surrogate.rs`

**Test**:
```rust
pub fn benchmark_rbf_cross_hardware() -> Result<BenchmarkReport> {
    // Train on GPU
    let rbf = RbfInterpolator::fit(&X_train, &y_train, ThinPlateSpline, 1.0)?;
    
    // Predict on CPU
    let y_cpu = rbf.predict_on_cpu(&X_test)?;
    
    // Predict on GPU
    let y_gpu = rbf.predict_on_gpu(&X_test)?;
    
    // Predict on NPU
    let akida_model = RbfAkidaExporter::new(&rbf).export()?;
    let y_npu = akida_model.predict(&X_test)?;
    
    // Validate identical results
    assert_close(y_cpu, y_gpu, 1e-6);
    assert_close(y_gpu, y_npu, 1e-3);  // Quantization tolerance
    
    BenchmarkReport {
        cpu: { predictions: y_cpu, time_us: ..., power_w: ... },
        gpu: { predictions: y_gpu, time_us: ..., power_w: ... },
        npu: { predictions: y_npu, time_us: ..., power_w: ... },
        max_diff: ...,
    }
}
```

**Timeline**: 1 week

---

## 📈 **Implementation Timeline**

### Phase 1: Surrogate Pipeline (5 weeks)
```
Week 1-2:  Cholesky Decomposition
Week 2-3:  Triangular Solve
Week 3:    RBF Kernel
Week 4:    RBF Interpolator
Week 5:    Integration + Validation
```

### Phase 2: NPU Path (2 weeks)
```
Week 6-7:  RBF Akida Export + Cross-Hardware Benchmark
```

### Phase 3: MD Completion (3 weeks)
```
Week 8-9:  Neighbor List
Week 10:   Force Kernel Integration Tests
```

**Total**: 10 weeks for complete hotSpring integration

**Critical Path**: Cholesky → Tri Solve → RBF Kernel → RBF Interpolator (4 weeks)

---

## 🎯 **Deep Debt Guarantee**

All implementations will follow existing patterns:

✅ **Modern Idiomatic Rust**
- Follows existing op patterns (`cdist_wgsl.rs`, `inverse_wgsl.rs`)
- Safe wrappers, no unsafe blocks
- Result types, proper error handling

✅ **Pure WGSL Shaders**
- Hardware-agnostic (WGPU)
- No hardcoded values
- Runtime-configured parameters

✅ **Composable**
- RBF Interpolator = Cholesky + TriangularSolve + RBFKernel
- Neighbor List = histc + argsort + searchsorted

✅ **Tested**
- Unit tests (each shader)
- Integration tests (full pipelines)
- Validation tests (vs. Python reference)

✅ **Documented**
- Algorithm explanations
- Physics context
- Example usage

---

## 📊 **Capability After Implementation**

### Surrogate Learning ✅
- ✅ Train RBF surrogates on GPU
- ✅ Evaluate on CPU/GPU/NPU
- ✅ Identical results across hardware
- ✅ Python `scipy` compatible

### MD Simulation ✅
- ✅ 5 force kernels (Coulomb, Yukawa, LJ, Morse, Born-Mayer)
- ✅ Velocity-Verlet integrator
- ✅ Neighbor lists (O(N) scaling)
- ✅ PBC + minimum image
- ✅ FFT 3D (for PPPM validation)

### Cross-Hardware ✅
- ✅ Train on GPU → Infer on NPU
- ✅ Benchmark CPU/GPU/NPU
- ✅ Measure speedup + power
- ✅ Identical predictions (within tolerance)

---

## 🚀 **Next Steps**

1. **Approve roadmap** - Confirm timeline + priorities
2. **Start with Cholesky** - Week 1-2, closes biggest gap
3. **Reference data access** - Need hotSpring control results for validation
4. **Validation framework** - Set up tests with Sarkas/scipy reference data

---

## 📝 **Questions for hotSpring Team**

1. **Reference Data**: Can we get read-only access to hotSpring control results?
   - Need: `surrogate/results/*.json` for RBF validation
   - Need: `sarkas/simulations/dsf-study/results/*.json` for MD validation

2. **Validation Tolerance**: What's acceptable for "identical results"?
   - RBF: < 1e-6 vs. scipy?
   - MD forces: < 1e-6 vs. Sarkas?
   - NPU quantized: < 1e-3 vs. GPU?

3. **Priority Order**: Confirm surrogate pipeline first, then MD?

4. **Timeline**: 10 weeks acceptable for complete integration?

---

**Status**: Ready to proceed  
**Contact**: ToadStool Team  
**Date**: February 8, 2026

**🍄 BarraCUDA: Universal Math for Universal Hardware**
