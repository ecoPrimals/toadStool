# 🧠 → 🦈 Neuromorphic to barraCUDA Migration Plan

**Date**: January 31, 2026  
**Status**: Architecture Evolution  
**Goal**: Absorb Akida-specific operations into universal barraCUDA framework

---

## 🎯 **VISION: TRUE UNIVERSAL COMPUTE**

### **The Problem**

Current architecture:
```
Akida-specific code → Akida hardware only
GPU code (barraCUDA) → GPU hardware only
CPU code → CPU only
```

### **The Solution**

Universal architecture:
```
barraCUDA Operations → ANY hardware (NPU, TPU, GPU, CPU)
    ├─ Akida backend    → Dispatches to NPU when available
    ├─ CUDA/Vulkan      → Dispatches to GPU
    ├─ Metal            → Dispatches to Apple Silicon
    └─ CPU fallback     → Always available
```

**Key Insight**: Operations are hardware-agnostic. Backends handle hardware-specific dispatch.

---

## 📊 **CURRENT AKIDA CODE ANALYSIS**

### **Akida-Specific Implementations**

| Component | Location | Purpose | Migration Target |
|-----------|----------|---------|------------------|
| **K-mer Filtering** | `showcase/neuromorphic/02-akida-bioinformatics/src/akida_filter.rs` | Pattern matching | barraCUDA: Pattern ops |
| **SNN Inference** | `crates/neuromorphic/akida-driver/src/inference.rs` | Spike-based compute | barraCUDA: Neuromorphic ops |
| **Reservoir Computing** | `crates/neuromorphic/akida-reservoir-research/` | Echo state networks | barraCUDA: Recurrent ops |
| **Model Loading** | `crates/neuromorphic/akida-models/src/` | NPU model format | Backend: Model adapter |
| **Device Management** | `crates/neuromorphic/akida-driver/src/discovery.rs` | Hardware enumeration | Backend: Device pool |

### **Operations to Extract**

**1. Pattern Matching Operations** (K-mer Filtering):
- One-hot encoding
- Spike train generation
- Pattern classification
- GC content filtering
- Low complexity detection
- Adapter sequence matching

**2. Neuromorphic Operations** (SNN):
- Spike encoding/decoding
- Temporal integration
- Leaky integrate-and-fire neurons
- Spike timing dependent plasticity (STDP)
- Rate coding
- Temporal coding

**3. Reservoir Computing Operations**:
- Random reservoir generation
- Echo state property enforcement
- Spectral radius control
- Sparse connectivity
- State extraction
- Readout training (ridge regression)

**4. Model Operations**:
- Quantized inference
- Low-bit arithmetic (1-4 bit)
- Event-driven computation
- Sparse activations

---

## 🦈 **BARRACUDA OPERATIONS TO ADD**

### **Phase 1: Neuromorphic Primitives** (5 operations)

1. **`spike_encode`** - Convert continuous values to spike trains
   ```rust
   pub async fn spike_encode(
       device: &Device,
       queue: &Queue,
       input: &[f32],      // Input values
       dt: f32,            // Time step
       duration: f32,      // Spike train duration
   ) -> Result<Vec<u8>> {  // Spike times
   ```

2. **`spike_decode`** - Convert spike trains to continuous values
   ```rust
   pub async fn spike_decode(
       device: &Device,
       queue: &Queue,
       spikes: &[u8],      // Spike times
       dt: f32,            // Time step
   ) -> Result<Vec<f32>> {  // Decoded values
   ```

3. **`lif_neuron`** - Leaky integrate-and-fire neuron
   ```rust
   pub async fn lif_neuron(
       device: &Device,
       queue: &Queue,
       input_current: &[f32],  // Input current
       tau: f32,               // Time constant
       threshold: f32,         // Spike threshold
       reset: f32,             // Reset potential
   ) -> Result<Vec<u8>> {      // Output spikes
   ```

4. **`temporal_pool`** - Temporal pooling for spike trains
   ```rust
   pub async fn temporal_pool(
       device: &Device,
       queue: &Queue,
       spikes: &[u8],      // Input spikes
       window: usize,      // Pooling window
   ) -> Result<Vec<f32>> {  // Pooled rates
   ```

5. **`sparse_matmul_quantized`** - Sparse matrix multiply with quantization
   ```rust
   pub async fn sparse_matmul_quantized(
       device: &Device,
       queue: &Queue,
       sparse_matrix: &SparseMatrix,  // Sparse weights
       input: &[i8],                  // Quantized input
       scale: f32,                    // Quantization scale
   ) -> Result<Vec<i8>> {             // Quantized output
   ```

### **Phase 2: Pattern Matching** (3 operations)

6. **`pattern_match`** - Fast pattern matching (K-mer style)
   ```rust
   pub async fn pattern_match(
       device: &Device,
       queue: &Queue,
       sequences: &[u8],    // Input sequences
       patterns: &[u8],     // Pattern library
       k: usize,            // Pattern length
   ) -> Result<Vec<u32>> {  // Match indices
   ```

7. **`gc_content`** - GC content calculation (vectorized)
   ```rust
   pub async fn gc_content(
       device: &Device,
       queue: &Queue,
       sequences: &[u8],    // DNA sequences (ACGT)
       window_size: usize,  // Sliding window
   ) -> Result<Vec<f32>> {  // GC ratios
   ```

8. **`complexity_filter`** - Low complexity region detection
   ```rust
   pub async fn complexity_filter(
       device: &Device,
       queue: &Queue,
       sequences: &[u8],    // Input sequences
       threshold: f32,      // Entropy threshold
   ) -> Result<Vec<bool>> {  // Pass/fail mask
   ```

### **Phase 3: Reservoir Computing** (4 operations)

9. **`reservoir_init`** - Initialize random reservoir
   ```rust
   pub async fn reservoir_init(
       device: &Device,
       queue: &Queue,
       input_size: usize,
       reservoir_size: usize,
       spectral_radius: f32,  // < 1.0 for echo state
       sparsity: f32,         // Fraction of zeros
       seed: u64,
   ) -> Result<(Vec<f32>, Vec<f32>)> {  // (W_in, W_res)
   ```

10. **`reservoir_update`** - Echo state network forward pass
    ```rust
    pub async fn reservoir_update(
        device: &Device,
        queue: &Queue,
        state: &[f32],        // Current reservoir state
        input: &[f32],        // Input vector
        w_in: &[f32],         // Input weights
        w_res: &[f32],        // Reservoir weights
        leak_rate: f32,       // Leaky integrator rate
    ) -> Result<Vec<f32>> {    // New state
    ```

11. **`spectral_radius`** - Compute spectral radius of matrix
    ```rust
    pub async fn spectral_radius(
        device: &Device,
        queue: &Queue,
        matrix: &[f32],       // Square matrix
        n: usize,             // Matrix dimension
    ) -> Result<f32> {         // Largest eigenvalue magnitude
    ```

12. **`ridge_regression`** - Train readout layer
    ```rust
    pub async fn ridge_regression(
        device: &Device,
        queue: &Queue,
        states: &[f32],       // Collected reservoir states
        targets: &[f32],      // Target outputs
        alpha: f32,           // L2 regularization
    ) -> Result<Vec<f32>> {    // Readout weights
    ```

---

## 🏗️ **BACKEND ARCHITECTURE**

### **Current: Hardware-Specific**

```rust
// Akida-specific
impl AkidaDevice {
    pub fn infer(&mut self, input: &[u8]) -> Result<Vec<u8>> {
        // Direct hardware access
        self.io.write(input)?;
        self.io.read(&mut output)?;
    }
}
```

### **Future: Universal Dispatch**

```rust
// Universal operation
pub async fn spike_encode(
    device: &Device,      // wgpu::Device (agnostic)
    queue: &Queue,
    input: &[f32],
    dt: f32,
) -> Result<Vec<u8>> {
    // barraCUDA dispatches to best backend:
    // - NPU (Akida) if available + optimal
    // - GPU (Vulkan/Metal/DX12) if available
    // - CPU fallback
}
```

### **Backend Selection Logic**

```rust
pub enum ComputeBackend {
    Neuromorphic(NeuromorphicDevice),  // Akida, Loihi, etc.
    GPU(GpuDevice),                     // NVIDIA, AMD, Apple
    CPU(CpuDevice),                     // Fallback
}

impl ComputeBackend {
    pub fn select_for_operation(op: &str, available: &[Backend]) -> Backend {
        match op {
            "spike_encode" if has_npu => Backend::Neuromorphic,  // NPU optimal
            "matmul" if has_gpu => Backend::GPU,                 // GPU optimal
            _ => Backend::CPU,                                   // Fallback
        }
    }
}
```

---

## 🔄 **MIGRATION STRATEGY**

### **Step 1: Extract Operations** ✅ (This Plan)

- [x] Analyze Akida-specific code
- [x] Identify reusable operations
- [x] Define 12 new barraCUDA operations
- [x] Document API signatures

### **Step 2: Implement barraCUDA Operations** 🔜

**For each operation**:
1. Create `crates/barracuda/src/ops/[operation].rs`
2. Implement WGSL shader (GPU path)
3. Write 5-test pattern (basic, edge, boundary, large, precision)
4. Document with examples

**Priority Order**:
1. `sparse_matmul_quantized` (foundational)
2. `spike_encode` / `spike_decode` (neuromorphic core)
3. `lif_neuron` (SNN primitive)
4. `pattern_match` / `gc_content` / `complexity_filter` (bioinformatics)
5. `reservoir_*` operations (echo state networks)

### **Step 3: Create Backend Abstraction** 🔜

**New crate**: `crates/barracuda/src/backends/`

```rust
pub mod backends {
    pub mod gpu;           // wgpu (existing)
    pub mod neuromorphic;  // NPU dispatch
    pub mod cpu;           // Fallback
}

pub trait Backend {
    fn supports_operation(&self, op: &str) -> bool;
    fn execute_operation(&self, op: Operation) -> Result<Tensor>;
    fn cost_estimate(&self, op: Operation) -> f64;  // Power/latency
}
```

### **Step 4: Neuromorphic Backend Implementation** 🔜

**New file**: `crates/barracuda/src/backends/neuromorphic.rs`

```rust
pub struct NeuromorphicBackend {
    devices: Vec<NeuromorphicDevice>,
}

pub enum NeuromorphicDevice {
    Akida(AkidaDevice),      // Existing driver
    Loihi(LoihiDevice),      // Future
    TrueNorth(TrueNorthDevice),  // Future
}

impl Backend for NeuromorphicBackend {
    fn supports_operation(&self, op: &str) -> bool {
        matches!(op, 
            "spike_encode" | "spike_decode" | "lif_neuron" | 
            "pattern_match" | "sparse_matmul_quantized"
        )
    }
    
    fn execute_operation(&self, op: Operation) -> Result<Tensor> {
        match &self.devices[0] {
            NeuromorphicDevice::Akida(dev) => {
                // Use existing Akida driver
                akida_execute(dev, op)
            }
            // ... other NPU types
        }
    }
}
```

### **Step 5: Update Showcases** 🔜

**K-mer Filtering Showcase** → Use barraCUDA operations:

```rust
// Before (Akida-specific):
let akida_filter = AkidaFilter::new().await?;
let stats = akida_filter.filter_kmers(&sequences, &config)?;

// After (Universal):
let device = WgpuDevice::new().await?;
let gc = barracuda::gc_content(&device.device, &device.queue, &sequences, k).await?;
let mask = barracuda::complexity_filter(&device.device, &device.queue, &sequences, 1.5).await?;
let filtered = apply_mask(&sequences, &mask);
```

### **Step 6: Workload Orchestrator Integration** 🔜

Update `crates/toadstool/src/orchestrator/mod.rs`:

```rust
pub struct WorkloadOrchestrator {
    backends: Vec<Box<dyn Backend>>,  // GPU, NPU, CPU
}

impl WorkloadOrchestrator {
    pub fn execute(&self, workload: Workload) -> Result<Output> {
        // Select best backend for this workload
        let backend = self.select_backend(&workload);
        backend.execute_operation(workload.operation)
    }
    
    fn select_backend(&self, workload: &Workload) -> &dyn Backend {
        // Cost-based selection:
        // - NPU: Best for sparse, low-bit, spike-based
        // - GPU: Best for dense, FP32, matrix-heavy
        // - CPU: Fallback
        
        let costs: Vec<_> = self.backends.iter()
            .map(|b| (b, b.cost_estimate(workload.operation)))
            .collect();
        
        costs.iter().min_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).unwrap().0
    }
}
```

---

## 📈 **BENEFITS OF MIGRATION**

### **1. True Hardware Agnosticism** ✅

**Before**:
```rust
// Code locked to Akida
let akida = AkidaDevice::open(0)?;
let result = akida.infer(&input)?;
```

**After**:
```rust
// Works on ANY hardware
let device = Device::detect_best()?;  // Akida, NVIDIA, AMD, Apple, CPU
let result = barracuda::spike_encode(&device, &input).await?;
```

### **2. Cross-Hardware Optimization** ✅

Same workload automatically uses best hardware:
- **Akida NPU**: Pattern matching (50-100x power efficient)
- **NVIDIA GPU**: Dense matmul (1000x faster than CPU)
- **AMD GPU**: Same ops via Vulkan
- **Apple M-series**: Same ops via Metal
- **EPYC CPU**: Fallback for anything

### **3. Composability** ✅

Mix backends in same pipeline:
```rust
// K-mer filtering pipeline (automatic backend selection)
let sequences = load_sequences("genome.fasta")?;

// Step 1: GC content (GPU optimal)
let gc = barracuda::gc_content(&gpu_device, &sequences, 31).await?;

// Step 2: Pattern match (NPU optimal if available)
let matches = barracuda::pattern_match(&best_device, &sequences, &patterns, 31).await?;

// Step 3: Complexity filter (GPU optimal)
let mask = barracuda::complexity_filter(&gpu_device, &sequences, 1.5).await?;
```

### **4. Future-Proof** ✅

New hardware? Just add backend:
```rust
// Add Intel Loihi support
impl Backend for LoihiBackend {
    fn supports_operation(&self, op: &str) -> bool {
        matches!(op, "spike_encode" | "lif_neuron")
    }
}

// All existing code works immediately!
```

### **5. Testing & Development** ✅

Develop without hardware:
```rust
// GPU simulation of NPU operations
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_spike_encode() {
        // Tests run on GPU even without NPU
        let device = WgpuDevice::new().await.unwrap();
        let spikes = spike_encode(&device.device, &device.queue, &[0.5], 0.001).await.unwrap();
        assert!(!spikes.is_empty());
    }
}
```

---

## 🎯 **MIGRATION MILESTONES**

### **Milestone 1: Foundation** (3-4 batches)

- [ ] Implement 5 neuromorphic primitives
- [ ] Write 25 tests (5 per operation)
- [ ] Document APIs
- [ ] **Outcome**: `spike_encode`, `spike_decode`, `lif_neuron`, `temporal_pool`, `sparse_matmul_quantized` in barraCUDA

### **Milestone 2: Pattern Matching** (2 batches)

- [ ] Implement 3 pattern operations
- [ ] Write 15 tests
- [ ] Benchmark against Akida-specific code
- [ ] **Outcome**: K-mer filtering works via barraCUDA

### **Milestone 3: Reservoir Computing** (2-3 batches)

- [ ] Implement 4 reservoir operations
- [ ] Write 20 tests
- [ ] Port reservoir research crate
- [ ] **Outcome**: Echo state networks in barraCUDA

### **Milestone 4: Backend Abstraction** (1 large batch)

- [ ] Create `backends/` module
- [ ] Implement trait
- [ ] Add neuromorphic backend (Akida adapter)
- [ ] **Outcome**: Multi-backend dispatch working

### **Milestone 5: Integration** (1-2 batches)

- [ ] Update workload orchestrator
- [ ] Port showcases to universal ops
- [ ] Update documentation
- [ ] **Outcome**: Full universal compute stack

---

## 🔬 **TECHNICAL CHALLENGES**

### **Challenge 1: Spike Encoding on GPU**

**Problem**: GPUs are optimized for dense FP32, not sparse events  
**Solution**: Use rate coding + temporal binning (efficient on GPU)

```wgsl
// spike_encode.wgsl
@compute @workgroup_size(256)
fn spike_encode(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @group(0) @binding(0) var<storage, read> input: array<f32>,
    @group(0) @binding(1) var<storage, read_write> spikes: array<u32>,
    @group(0) @binding(2) var<uniform> params: Params,
) {
    let idx = gid.x;
    if (idx >= params.n) { return; }
    
    // Rate coding: value → spike probability
    let rate = input[idx];
    let spike_count = u32(rate * f32(params.time_steps));
    spikes[idx] = spike_count;
}
```

### **Challenge 2: Quantized Operations**

**Problem**: WGSL doesn't have native int4/int8 SIMD  
**Solution**: Pack into u32, use bit manipulation

```wgsl
// Pack 4x int8 into u32
fn pack_int8x4(a: i32, b: i32, c: i32, d: i32) -> u32 {
    return (u32(a & 0xFF) << 0) |
           (u32(b & 0xFF) << 8) |
           (u32(c & 0xFF) << 16) |
           (u32(d & 0xFF) << 24);
}
```

### **Challenge 3: Spectral Radius Calculation**

**Problem**: Eigenvalue computation is complex on GPU  
**Solution**: Power iteration method (GPU-friendly)

```rust
// Approximate largest eigenvalue
pub async fn spectral_radius(
    device: &Device,
    queue: &Queue,
    matrix: &[f32],
    n: usize,
) -> Result<f32> {
    // Power iteration: A^k * v
    let mut v = vec![1.0; n];  // Random vector
    for _ in 0..100 {          // Iterate
        v = matmul(device, queue, matrix, &v, n, n).await?;
        let norm = vector_norm(device, queue, &v).await?;
        v = scale(device, queue, &v, 1.0 / norm).await?;
    }
    Ok(norm)  // Converged to largest eigenvalue
}
```

---

## 📝 **NEXT STEPS**

### **Immediate** (This Session)

1. ✅ Create this migration plan
2. ✅ Document 12 new operations
3. 🔜 Decide: Start neuromorphic ops OR continue barraCUDA marathon?

### **Near-Term** (Next 7-10 Batches)

1. Implement Milestone 1 (neuromorphic primitives)
2. Implement Milestone 2 (pattern matching)
3. Implement Milestone 3 (reservoir computing)
4. **Result**: +12 operations, +60 tests, universal compute ready

### **Medium-Term** (Backend Integration)

1. Create backend abstraction
2. Implement neuromorphic backend
3. Update orchestrator
4. Port showcases

### **Long-Term** (Ecosystem)

1. Add more NPU backends (Loihi, TrueNorth)
2. Add TPU backend (Google Coral)
3. Optimize backend selection (cost models)
4. Power profiling integration

---

## 🎊 **SUCCESS CRITERIA**

**Migration is complete when**:

1. ✅ All 12 neuromorphic operations in barraCUDA
2. ✅ 100% test pass rate (60 new tests)
3. ✅ Backend abstraction implemented
4. ✅ Akida hardware works through barraCUDA
5. ✅ K-mer showcase uses universal ops
6. ✅ Workload orchestrator dispatches to best backend
7. ✅ Same code runs on NPU/GPU/CPU without changes

**Outcome**: **TRUE UNIVERSAL COMPUTE!** 🌟

---

*"One operation, infinite hardware. That's the toadStool way!"* 🍄🦈🧠✨
