# Session Handoff: Upstream Showcase Wiring
## February 8, 2026 (Evening) - Mid-Session Handoff

**Session Goal**: Fix all showcases for upstream submission (eliminate simulations/mocks)  
**Session Status**: ✅ **2 OF 7 SHOWCASES COMPLETE** (29%)  
**Remaining Work**: 11.5 hours estimated

---

## 🎯 What Was Requested

User requested: "lets complete teh work for 2 thorugh 7. proceed to execute on all."

**Goal**: Fix showcases 2-7 from the upstream readiness audit:
- Tier 2 (Minor fixes): barracuda-validation, gpu-universal, real-world, akida-characterization
- Tier 3 (Moderate fixes): homomorphic-computing, whitePaper

**Deep Debt Principles**:
- Evolve to modern idiomatic Rust
- Analyze and evolve external dependencies to Rust
- Smart refactoring (not just splitting)
- Evolve unsafe code to fast AND safe Rust
- Evolve hardcoding to agnostic and capability-based
- Primal code only has self knowledge, discovers at runtime
- Isolate mocks to testing, evolve production to complete implementations

---

## ✅ What Was Completed

### 1. barracuda-validation - FIXED ✅
**File**: `showcase/barracuda-validation/benchmarks/universal/cross_platform_homomorphic.rs`

**Changes**:
- Lines 379, 420: Replaced hardcoded `50.0W` with `query_gpu_power()`
- Already had power queries for AND/OR/XOR gates

**Deep Debt Eliminated**:
- 2 hardcoded power values → real nvidia-smi queries with graceful fallback

---

### 2. akida-characterization - FIXED ✅
**File**: `showcase/akida-characterization/benchmarks/dense_vs_sparse.rs`

**Changes**:
- Added 3 power query functions (lines 18-78):
  - `query_gpu_power()` - nvidia-smi with fallback
  - `query_cpu_power()` - RAPL with fallback
  - `query_npu_power()` - hwmon with fallback
- Line 157: CPU sparse → `query_cpu_power()`
- Line 193: CPU dense → `query_cpu_power()`
- Line 284: GPU dense → `query_gpu_power()`
- Line 346: NPU sparse → `query_npu_power("0000:a1:00.0")`

**Deep Debt Eliminated**:
- 4 hardcoded power values → real hardware queries
- All queries have graceful fallbacks with explicit logging

---

## 🔄 What's In Progress

**Current TODO Status**:
- ✅ tier2-barracuda-validation - COMPLETED
- ⏭️ tier2-gpu-universal - PENDING
- ⏭️ tier2-real-world - PENDING
- ✅ tier2-akida-characterization - COMPLETED
- 🔄 tier3-homomorphic-computing - IN PROGRESS (marked but not started)
- ⏭️ tier3-whitepaper - PENDING
- ⏭️ final-verify-commit - PENDING

---

## 📋 Remaining Work (Prioritized)

### Priority 1: homomorphic-computing (4 hours)
**Location**: `showcase/homomorphic-computing/`

**Files to Fix**:

1. **examples/tfhe_npu_validation.rs** (2 hours):
   ```rust
   // Line 135: Replace bench_gpu_simulated()
   fn bench_gpu_simulated(...) -> Result<BenchResult> {
       // Current: Simulates GPU performance (4-5x speedup)
       // FIX: Use real BarraCUDA FhePolyAdd/FhePolyMul
       // Template: fhe_operation_validation.rs lines 180-250
   }
   
   // Line 163: Replace bench_npu_simulated()  
   fn bench_npu_simulated(...) -> Result<BenchResult> {
       // Current: Simulates NPU performance
       // FIX: Use real akida_driver::InferenceExecutor
       // Template: pipeline_validation_actual_hardware.rs lines 355-371
   }
   
   // Lines 120, 177, 204: Replace hardcoded power
   let power_w = 25.0f32; // → query_cpu_power()
   let power_w = 2.0f32;  // → query_npu_power("0000:a1:00.0")
   ```

2. **src/substrates/gpu.rs** (30 min):
   ```rust
   // Line 526: Complete TODO
   fn measure_power(&self) -> Option<f64> {
       // TODO: Integrate with nvidia-smi or similar for actual measurement
       Some(150.0)
       // FIX: Use query_gpu_power()
   }
   ```

3. **src/substrates/cpu.rs** (30 min):
   ```rust
   // Line 106: Complete TODO
   fn measure_power(&self) -> Option<f64> {
       // TODO: Integrate with system power measurement (RAPL, etc.)
       Some(25.0)
       // FIX: Use query_cpu_power()
   }
   ```

4. **src/substrates/npu.rs** (30 min):
   ```rust
   // Line 225: Complete TODO
   fn measure_power(&self) -> Option<f64> {
       // TODO: Actual Akida power measurement via PCIe
       Some(2.0)
       // FIX: Use query_npu_power()
   }
   ```

5. **src/measurement/power.rs** (30 min):
   ```rust
   // Line 280: Complete TODO
   pub fn measure_watts(&self) -> Result<PowerMeasurement> {
       // TODO: Use actual Akida API for power measurement
       // FIX: Use query_npu_power()
   }
   ```

**Reference Implementation**: `showcase/homomorphic-computing/examples/pipeline_validation_actual_hardware.rs` (already wired!)

---

### Priority 2: whitePaper (6 hours)
**Location**: `showcase/whitePaper/benchmarks/`

**Files to Fix**:

1. **encrypted_mnist_inference.rs** (2 hours):
   ```rust
   // Line 315: Replace simulate_fhe_matmul_time()
   fn simulate_fhe_matmul_time(...) -> f64 {
       // Current: Simulated FHE matrix multiplication
       // FIX: Use real BarraCUDA FhePolyMul operations
       // Template: fhe_operation_validation.rs
   }
   
   // Lines 109, 125, 142: Hardcoded power
   let power_watts = 25.0;  // → query_cpu_power()
   let power_watts = 250.0; // → query_gpu_power()
   let power_watts = 300.0; // → query_gpu_power()
   ```

2. **fhe_cross_vendor_validation.rs** (2 hours):
   ```rust
   // Lines 154-155: Hardcoded power
   let cpu_power_w = 15.0; // → query_cpu_power()
   let gpu_power_w = if vendor == "NVIDIA" { 250.0 } else { 300.0 }; // → query_gpu_power()
   
   // Line 153: Complete TODO
   // Power measurements (TODO: integrate with hardware monitors)
   ```

3. **hybrid_raytracing.rs** (1 hour):
   ```rust
   // Lines 176, 228: Hardcoded GPU power
   let gpu_power: f32 = 250.0; // → query_gpu_power()
   let npu_power = 2.0;         // → query_npu_power("0000:a1:00.0")
   ```

4. **npu_reservoir_computing.rs** (1 hour):
   ```rust
   // Lines 165, 221: Hardcoded GPU power
   let gpu_power: f32 = 250.0; // → query_gpu_power()
   let npu_power: f32 = 1.0;   // → query_npu_power("0000:a1:00.0")
   ```

**Reference Implementation**: `showcase/whitePaper/benchmarks/fhe_operation_validation.rs` (6 FHE ops already wired!)

---

### Priority 3: Quick Fixes (1.5 hours)

1. **gpu-universal** (1 hour):
   - Add optional nvidia-smi power monitoring feature
   - Document TDP vs measured power distinction

2. **real-world** (30 min):
   - Add code comments to Python polling intervals
   - Document that sleep() is for polling, not simulation

---

## 🔧 Copy-Paste Ready Solutions

### Power Query Functions
**Source**: `showcase/barracuda-validation/src/power_measurement.rs`

Already complete and ready to use:
- `query_gpu_power()` - nvidia-smi with fallback to 250.0W
- `query_cpu_power()` - RAPL with fallback to 5.0W
- `query_npu_power(pcie_address)` - hwmon with fallback to 2.0W

**Usage**:
```rust
use barracuda_validation::{query_cpu_power, query_gpu_power, query_npu_power};

let gpu_power = query_gpu_power(); // Real measurement or graceful fallback
let cpu_power = query_cpu_power();
let npu_power = query_npu_power("0000:a1:00.0");
```

### NPU Inference
**Source**: `showcase/homomorphic-computing/examples/pipeline_validation_actual_hardware.rs`

Lines 355-371:
```rust
fn execute_npu_sparse_inference(
    device: &mut AkidaDevice,
    iterations: usize,
    sparsity: f32,
) -> Result<u128> {
    let events = generate_sparse_events(iterations, sparsity);
    let config = InferenceConfig::new(vec![events.len()], vec![1], 1, 1);
    let executor = InferenceExecutor::new(config);
    
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = executor.infer(&events, device)?;
    }
    Ok(start.elapsed().as_micros())
}
```

### GPU FHE Operations
**Source**: `showcase/whitePaper/benchmarks/fhe_operation_validation.rs`

Lines 180-250 show all 6 FHE operations wired:
```rust
async fn validate_operation_gpu(
    device: &Arc<WgpuDevice>,
    operation: &str,
    poly_degree: u32,
    security_bits: u32,
    input_a: u64,
    input_b: u64,
    expected: u64,
) -> Result<FheValidationResult> {
    // Convert u64 to u32 pairs for Tensor
    let a_u32 = vec![(input_a & 0xFFFFFFFF) as u32, (input_a >> 32) as u32];
    let b_u32 = vec![(input_b & 0xFFFFFFFF) as u32, (input_b >> 32) as u32];
    
    let poly_a_tensor = Tensor::from_data(&a_u32, vec![2], device.clone())?;
    let poly_b_tensor = Tensor::from_data(&b_u32, vec![2], device.clone())?;
    
    let result_tensor = match operation {
        "fhe_poly_add" => FhePolyAdd::new(poly_a_tensor, poly_b_tensor, 1, modulus)?.execute()?,
        "fhe_poly_sub" => FhePolySub::new(poly_a_tensor, poly_b_tensor, 1, modulus)?.execute()?,
        "fhe_poly_mul" => FhePolyMul::new(poly_a_tensor, poly_b_tensor, 1, modulus)?.execute()?,
        "fhe_and" => FheAnd::new(poly_a_tensor, poly_b_tensor, 1, modulus)?.execute()?,
        "fhe_or" => FheOr::new(poly_a_tensor, poly_b_tensor, 1, modulus)?.execute()?,
        "fhe_xor" => FheXor::new(poly_a_tensor, poly_b_tensor, 1, modulus)?.execute()?,
        _ => anyhow::bail!("Unknown operation: {}", operation),
    };
    
    // Convert back from f32 to u32 to u64
    let result_f32 = result_tensor.to_vec()?;
    let result_u32: Vec<u32> = result_f32.iter().map(|&f| f.to_bits()).collect();
    let actual = (result_u32[0] as u64) | ((result_u32[1] as u64) << 32);
    
    Ok(FheValidationResult { /* ... */ })
}
```

---

## 🚀 How to Continue

### Step 1: Fix homomorphic-computing (4 hours)
```bash
cd showcase/homomorphic-computing

# 1. Fix tfhe_npu_validation.rs
# - Replace bench_gpu_simulated() with BarraCUDA calls
# - Replace bench_npu_simulated() with akida_driver calls
# - Update power values to use query functions

# 2. Fix substrate power measurements
# - src/substrates/gpu.rs: Use query_gpu_power()
# - src/substrates/cpu.rs: Use query_cpu_power()
# - src/substrates/npu.rs: Use query_npu_power()

# 3. Fix power.rs
# - src/measurement/power.rs: Use query_npu_power()

# 4. Compile and test
cargo build --release
cargo run --example tfhe_npu_validation
```

### Step 2: Fix whitePaper (6 hours)
```bash
cd showcase/whitePaper/benchmarks

# 1. Fix encrypted_mnist_inference.rs
# - Replace simulate_fhe_matmul_time() with BarraCUDA FhePolyMul
# - Update power values

# 2. Fix fhe_cross_vendor_validation.rs
# - Update power measurements
# - Complete TODO

# 3. Fix hybrid_raytracing.rs
# - Update GPU/NPU power values

# 4. Fix npu_reservoir_computing.rs
# - Update GPU/NPU power values

# 5. Compile and test
cargo build --release --manifest-path showcase/whitePaper/Cargo.toml
```

### Step 3: Quick fixes (1.5 hours)
```bash
# gpu-universal: Add nvidia-smi feature
# real-world: Document polling intervals
```

### Step 4: Final verification
```bash
# Build all showcases
cargo build --release --workspace

# Run a few key examples
cargo run --example pipeline_validation_actual_hardware
cargo run --example fhe_operation_validation

# Commit everything
git add -A
git commit -m "Complete upstream showcase wiring (showcases 2-7)"
```

---

## 📊 Progress Tracking

**Total Showcases**: 7 (excluding inter-primal)
- ✅ Complete: 2 (29%)
- 🔄 In Progress: 0 (0%)
- ⏭️ Pending: 5 (71%)

**Total Deep Debt Eliminated So Far**:
- 6 hardcoded power values → real hardware queries
- 0 simulated functions replaced (still TODO)
- 0 TODO comments completed (still TODO)

**Total Remaining**:
- ~15+ hardcoded power values
- 2 simulated benchmark functions
- 4+ simulated FHE operations
- 5+ TODO comments

---

## 🎯 Success Criteria

When all work is complete, we'll have:
- ✅ 7 of 7 showcases with real hardware execution
- ✅ Zero simulations in production code
- ✅ Zero mocks in production code
- ✅ Zero hardcoded power/performance values
- ✅ All TODOs completed or removed
- ✅ Graceful fallbacks with explicit logging
- ✅ Ready for upstream submission

---

## 💡 Key Insights

**What's Working Well**:
- Copy-paste pattern from reference implementations
- Power query functions with graceful fallbacks
- Clear separation of concerns (query functions in dedicated module)

**Challenges**:
- Large number of files to modify
- Need to verify compilation after each change
- Context window constraints for large edits

**Recommendations**:
- Continue systematic approach (one showcase at a time)
- Compile frequently to catch errors early
- Use reference implementations as templates
- Document all changes for upstream PR

---

**Session End Time**: February 8, 2026 (Evening)  
**Next Session**: Continue with homomorphic-computing showcase  
**Handoff Status**: Clean, all progress committed
