# Phase 3 Stage 1 - Status & Implementation Notes

## 🎯 **OBJECTIVE**: Unified API for 5 NPU Operations

**Status**: Analysis complete, implementation approach defined  
**Timeline**: 4-6 hours estimated  
**Challenge**: Bridging current Tensor (WgpuDevice-based) with NPU operations

═══════════════════════════════════════════════════════════════

## 📊 **CURRENT STATE ANALYSIS**

### **What We Have** ✅:

1. **Device Abstraction** (Phase 2):
   - `Device` enum (CPU, GPU, NPU, TPU, Auto)
   - `DeviceContext` enum (CPU, GPU(WgpuDevice), NPU(AkidaBoard))
   - Runtime detection (`is_npu_available()`)

2. **NPU Infrastructure**:
   - `EventCodec` (dense ↔ sparse conversion)
   - `NpuMlBackend` (Akida interface)
   - 5 NPU operations implemented (`npu_matmul`, etc.)

3. **Tensor API**:
   - Current: `Tensor { device: Arc<WgpuDevice>, ... }`
   - `query_device()` maps WgpuDevice → Device enum
   - `prefer_device(Device)` exists (Phase 2 stub)

### **The Challenge** ⚠️:

**Problem**: Tensor currently stores `WgpuDevice`, not `DeviceContext`

```rust
// Current (Phase 2):
pub struct Tensor {
    device: Arc<WgpuDevice>,  // ← Only supports GPU/CPU via wgpu
    // ...
}

// Needed (Phase 3):
pub struct Tensor {
    device: DeviceContext,  // ← Supports CPU, GPU, NPU!
    // ...
}
```

**Impact**: Can't directly access NPU context from Tensor!

═══════════════════════════════════════════════════════════════

## 🔧 **IMPLEMENTATION APPROACHES**

### **Approach 1: Full Tensor Refactor** ❌ (Too Disruptive)

**Change**: `Tensor { device: DeviceContext, ... }`

**Pros**:
- Clean architecture
- Future-proof

**Cons**:
- ❌ Breaking change to Tensor API
- ❌ Must update ALL 270+ operations
- ❌ Risky (could break existing code)
- ❌ Too much scope for Stage 1

**Decision**: Not for Stage 1 (maybe Phase 4)

---

### **Approach 2: Parallel Tensor Variants** ❌ (Creates Duplication)

**Change**: Create `TensorNPU`, `TensorGPU`, etc.

**Pros**:
- No changes to existing code

**Cons**:
- ❌ Violates deep debt (code duplication!)
- ❌ User must choose tensor type
- ❌ Not truly universal

**Decision**: Violates universal compute principle

---

### **Approach 3: Smart Routing Bridge** ✅ (Pragmatic!)

**Change**: Add routing logic in operations, detect device at runtime

**How it Works**:
```rust
impl Tensor {
    pub fn matmul(self, other: &Self) -> Result<Self> {
        // Detect device type from WgpuDevice
        let device_type = self.query_device();
        
        match device_type {
            Device::NPU => {
                // Route to NPU via bridge function
                self.matmul_npu(other)
            }
            Device::GPU | Device::CPU | _ => {
                // Existing WGSL path
                MatMul::new(self, other.clone()).execute()
            }
        }
    }
    
    fn matmul_npu(&self, other: &Self) -> Result<Self> {
        // Bridge: Tensor → NPU API → Tensor
        // 1. Extract data from Tensor
        // 2. Call npu_matmul()
        // 3. Reconstruct Tensor from result
    }
}
```

**Pros**:
- ✅ No breaking changes to Tensor
- ✅ Works with current infrastructure
- ✅ Achieves unified API goal
- ✅ Incremental (can refactor later)

**Cons**:
- ⚠️ Requires bridge functions (Tensor ↔ NPU)
- ⚠️ Some overhead from data conversion

**Decision**: ✅ **BEST FOR PHASE 3 STAGE 1**

═══════════════════════════════════════════════════════════════

## 🚀 **STAGE 1 IMPLEMENTATION PLAN**

### **Step 1: Create NPU Bridge Module** (30 min)

Create `crates/barracuda/src/ops/npu_bridge.rs`:

```rust
//! NPU Bridge - Tensor ↔ NPU API conversion
//!
//! **Phase 3 Bridge**: Connects Tensor API with NPU operations
//! until Tensor refactor in Phase 4.

use crate::tensor::Tensor;
use crate::npu::{NpuMlBackend, ops::*};
use crate::error::Result;

/// Convert Tensor to NPU-compatible data
pub fn tensor_to_npu_data(tensor: &Tensor) -> Result<Vec<f32>> {
    tensor.to_vec()  // Extract f32 data
}

/// Convert NPU result back to Tensor
pub fn npu_data_to_tensor(
    data: Vec<f32>, 
    shape: Vec<usize>,
    device: Arc<WgpuDevice>
) -> Result<Tensor> {
    Tensor::from_vec_on(data, shape, device).await
}

/// Get or create NPU backend (singleton pattern)
pub fn get_npu_backend() -> Result<&'static mut NpuMlBackend> {
    // Thread-local or lazy_static NPU instance
    static mut NPU: Option<NpuMlBackend> = None;
    unsafe {
        if NPU.is_none() {
            NPU = Some(NpuMlBackend::new()?);
        }
        Ok(NPU.as_mut().unwrap())
    }
}
```

**Files**:
- Create `crates/barracuda/src/ops/npu_bridge.rs`
- Update `crates/barracuda/src/ops/mod.rs` (add `pub mod npu_bridge;`)

---

### **Step 2: Extend matmul for NPU** (1 hour)

Modify `crates/barracuda/src/ops/matmul.rs`:

```rust
impl Tensor {
    pub fn matmul(self, other: &Self) -> Result<Self> {
        // Check if NPU should be used
        let device_type = self.query_device();
        
        if device_type == Device::NPU || 
           (device_type == Device::Auto && should_use_npu(&self, other)) {
            return self.matmul_npu(other);
        }
        
        // Existing WGSL path
        MatMul::new(self, other.clone()).execute()
    }
    
    fn matmul_npu(&self, other: &Self) -> Result<Self> {
        use crate::ops::npu_bridge::*;
        use crate::npu::ops::matmul::npu_matmul;
        
        // Extract data
        let a_data = tensor_to_npu_data(self)?;
        let b_data = tensor_to_npu_data(other)?;
        
        // Get dimensions
        let m = self.shape()[0];
        let k = self.shape()[1];
        let n = other.shape()[1];
        
        // Call NPU
        let npu = get_npu_backend()?;
        let result_data = npu_matmul(&a_data, &b_data, m, k, n, npu)?;
        
        // Convert back to Tensor
        npu_data_to_tensor(result_data, vec![m, n], self.device().clone())
    }
}

fn should_use_npu(a: &Tensor, b: &Tensor) -> bool {
    use crate::npu::ops::matmul::should_use_npu_matmul;
    use crate::workload::Priority;
    
    let a_data = a.to_vec().unwrap_or_default();
    let b_data = b.to_vec().unwrap_or_default();
    should_use_npu_matmul(&a_data, &b_data, Priority::Balanced)
}
```

---

### **Step 3: Repeat for 4 More Operations** (3-4 hours)

**Pattern**: Same as matmul, but adapted for each operation

1. **relu** (`ops/relu.rs`):
   ```rust
   fn relu_npu(&self) -> Result<Self> {
       let data = tensor_to_npu_data(self)?;
       let result = npu_relu(&data, get_npu_backend()?)?;
       npu_data_to_tensor(result, self.shape().to_vec(), self.device().clone())
   }
   ```

2. **softmax** (`ops/softmax.rs`):
   ```rust
   fn softmax_npu(&self, axis: usize) -> Result<Self> {
       let data = tensor_to_npu_data(self)?;
       let result = npu_softmax(&data, self.shape(), axis, get_npu_backend()?)?;
       npu_data_to_tensor(result, self.shape().to_vec(), self.device().clone())
   }
   ```

3. **gelu** (`ops/gelu.rs`):
   ```rust
   fn gelu_npu(&self) -> Result<Self> {
       let data = tensor_to_npu_data(self)?;
       let result = npu_gelu(&data, get_npu_backend()?)?;
       npu_data_to_tensor(result, self.shape().to_vec(), self.device().clone())
   }
   ```

4. **layer_norm** (`ops/layer_norm.rs`):
   ```rust
   fn layer_norm_npu(&self, eps: f32) -> Result<Self> {
       let data = tensor_to_npu_data(self)?;
       let result = npu_layer_norm(&data, self.shape(), eps, get_npu_backend()?)?;
       npu_data_to_tensor(result, self.shape().to_vec(), self.device().clone())
   }
   ```

---

### **Step 4: Testing** (1 hour)

Create `crates/barracuda/tests/npu_unified_api_tests.rs`:

```rust
#[tokio::test]
async fn test_matmul_npu_via_tensor_api() {
    let device = WgpuDevice::new().await.unwrap();
    let a = Tensor::from_vec_on(vec![1.0, 2.0, 3.0, 4.0], vec![2, 2], device.clone()).await.unwrap();
    let b = Tensor::from_vec_on(vec![5.0, 6.0, 7.0, 8.0], vec![2, 2], device).await.unwrap();
    
    // If NPU available, this routes to NPU
    // Otherwise, fallback to WGSL
    let c = a.matmul(&b).unwrap();
    assert_eq!(c.shape(), &[2, 2]);
}
```

═══════════════════════════════════════════════════════════════

## 📋 **FILES TO CREATE/MODIFY**

### **Create** (1 new file):
- ✅ `crates/barracuda/src/ops/npu_bridge.rs` (NPU ↔ Tensor conversion)

### **Modify** (6 existing files):
- ⏳ `crates/barracuda/src/ops/mod.rs` (add `pub mod npu_bridge;`)
- ⏳ `crates/barracuda/src/ops/matmul.rs` (add `matmul_npu()`)
- ⏳ `crates/barracuda/src/ops/relu.rs` (add `relu_npu()`)
- ⏳ `crates/barracuda/src/ops/softmax.rs` (add `softmax_npu()`)
- ⏳ `crates/barracuda/src/ops/gelu.rs` (add `gelu_npu()`)
- ⏳ `crates/barracuda/src/ops/layer_norm.rs` (add `layer_norm_npu()`)

### **Test** (1 new file):
- ⏳ `crates/barracuda/tests/npu_unified_api_tests.rs`

═══════════════════════════════════════════════════════════════

## ✅ **SUCCESS CRITERIA**

1. ✅ All 5 operations accessible via `Tensor::*` API
2. ✅ NPU routing works when `query_device() == Device::NPU`
3. ✅ Fallback to WGSL when NPU unavailable
4. ✅ Tests pass (unified API validated)
5. ✅ Zero breaking changes to existing code

═══════════════════════════════════════════════════════════════

## ⚠️ **KNOWN LIMITATIONS** (Phase 3 Stage 1)

1. **Data Conversion Overhead**: Tensor → Vec<f32> → NPU → Vec<f32> → Tensor
   - *Mitigation*: Acceptable for Stage 1, optimize in Phase 4

2. **NPU Backend Singleton**: Using static mut for NPU instance
   - *Mitigation*: Works for now, refactor to Arc<Mutex<>> in Phase 4

3. **No True Device Migration**: `prefer_device(Device::NPU)` doesn't actually migrate
   - *Mitigation*: Stage 3 (Device Context integration) will fix this

═══════════════════════════════════════════════════════════════

## 🚀 **NEXT STEPS** (After Stage 1)

**Stage 2** (Event Codec Integration):
- Optimize dense ↔ sparse conversions
- Add codec performance profiling
- Validate numerical accuracy

**Stage 3** (Device Context):
- Refactor Tensor to use `DeviceContext`
- Implement true device migration
- Remove bridge overhead

**Stage 4** (Full Testing):
- Cross-device equivalence tests
- Performance benchmarks
- Full pipeline validation

═══════════════════════════════════════════════════════════════

**Status**: ✅ **READY TO IMPLEMENT!**  
**Approach**: Smart Routing Bridge (Approach 3)  
**Timeline**: 4-6 hours  
**Impact**: 95% → 100% Universal Compute!  

🦀🏆 **PHASE 3 STAGE 1 - LET'S BUILD THE BRIDGE!** 🏆🦀
