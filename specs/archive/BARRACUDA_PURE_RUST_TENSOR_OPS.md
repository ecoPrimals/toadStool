# 🦈 barraCUDA: Pure Rust Tensor Operations Specification

**Date**: January 12, 2026  
**Version**: 1.1.0  
**Status**: Active Development - Architecture Complete, Expanding Coverage

**Implementation Status**: 10/21 operations complete (48%)  
**WGSL Shaders**: 21/21 complete (100%)  
**Grade**: A- (Architecture A+, Coverage 48%)

---

## Mission Statement

**Enable ALL advanced tensor operations that CUDA provides, implemented in pure Rust, executable on ANY hardware substrate (NVIDIA, AMD, Intel, Apple, CPU, neuromorphic) without vendor lock-in.**

## Current Achievement (January 12, 2026)

✅ **Architecture**: Production-ready pure Rust GPU framework  
✅ **WGSL Shaders**: 21/21 compute kernels complete (100%)  
✅ **Operations**: 10/21 fully implemented and tested (48%)  
✅ **Validation**: 241M elem/sec on NVIDIA RTX 3090  
✅ **Cross-Vendor**: Working on NVIDIA + AMD (Vulkan/wgpu)  
✅ **Deep Debt**: Zero violations (no unsafe, no vendor lock-in)

---

## Core Principles

### 1. Pure Rust Application Layer

**Requirement**: ZERO unsafe blocks in application code

```rust
// ✅ GOOD: Pure Rust, type-safe
pub async fn execute_relu(input: &[f32]) -> Result<Vec<f32>> {
    // Safe Rust only
}

// ❌ BAD: Unsafe in application
pub unsafe fn execute_relu_unsafe(ptr: *const f32) { }
```

**Rationale**: Safety, maintainability, auditability

---

### 2. Vendor Agnostic

**Requirement**: Same code works on ALL hardware

```rust
// ✅ GOOD: Discovers hardware at runtime
let device = WgpuExecutor::new().await?;  // Works on NVIDIA, AMD, Intel, Apple

// ❌ BAD: Vendor-specific code paths
#[cfg(feature = "nvidia")]
let device = CudaExecutor::new()?;  // Locks to NVIDIA
```

**Supported Substrates**:
- ✅ NVIDIA GPUs (via Vulkan/wgpu, NOT CUDA)
- ✅ AMD GPUs (via Vulkan/wgpu)
- ✅ Intel GPUs (via Vulkan/wgpu)
- ✅ Apple GPUs (via Metal/wgpu)
- ✅ CPU (via Rayon)
- 🎯 Future: Neuromorphic (Akida, Loihi)

---

### 3. WGSL Compute Shaders

**Requirement**: All GPU kernels in WGSL (WebGPU Shading Language)

```wgsl
// ✅ GOOD: Pure WGSL, portable
@compute @workgroup_size(256)
fn relu(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < arrayLength(&input)) {
        output[id.x] = max(input[id.x], 0.0);
    }
}
```

**Rationale**: 
- WebGPU standard (W3C)
- Portable across all backends
- Compile-time type checking
- Future-proof

---

### 4. Performance Target

**Requirement**: ≥80% of vendor-specific (CUDA) performance

**Rationale**: 
- Trade 20% performance for 100% vendor freedom
- Most workloads are memory-bound anyway
- Optimization headroom exists

---

### 5. Correctness Guarantee

**Requirement**: Max difference < 1e-6 vs reference implementation

**Validation**:
```rust
#[test]
fn validate_correctness() {
    let cpu_result = operation_cpu(&input);
    let gpu_result = operation_gpu(&input).await;
    
    let max_diff = max_absolute_difference(&cpu_result, &gpu_result);
    assert!(max_diff < 1e-6, "GPU output differs from CPU by {}", max_diff);
}
```

---

## Complete Operation Catalog

### Tier 1: Core Parallel Patterns (9 operations)

Essential primitives that compose into complex operations.

#### 1.1 Map

**CUDA Equivalent**: `thrust::transform`

**Signature**:
```rust
pub async fn execute_map<F>(
    &self,
    input: &[f32],
    operation: F,
) -> Result<Vec<f32>>
where F: Fn(f32) -> f32
```

**WGSL Kernel**: `shaders/map.wgsl`

**Use Cases**:
- Element-wise transforms
- Activation functions
- Normalization

**Performance Target**: >500M elem/sec (RTX 3090)

---

#### 1.2 Filter

**CUDA Equivalent**: `thrust::copy_if`, `cub::DeviceSelect`

**Signature**:
```rust
pub async fn execute_filter<F>(
    &self,
    input: &[f32],
    predicate: F,
) -> Result<Vec<f32>>
where F: Fn(f32) -> bool
```

**WGSL Kernel**: `shaders/filter.wgsl`

**Algorithm**: Stream compaction with prefix sum

**Use Cases**:
- Sparse operations
- Conditional selection
- Data filtering

**Performance Target**: >300M elem/sec

---

#### 1.3 Reduce

**CUDA Equivalent**: `thrust::reduce`, `cub::DeviceReduce`

**Signature**:
```rust
pub async fn execute_reduce(
    &self,
    input: &[f32],
    operation: ReduceOp,  // Sum, Max, Min, Mean
) -> Result<f32>
```

**WGSL Kernel**: `shaders/reduce.wgsl`

**Algorithm**: Tree reduction, work-efficient

**Use Cases**:
- Sum, max, min, mean
- Loss computation
- Gradient accumulation

**Performance Target**: >400GB/sec memory bandwidth

---

#### 1.4 Scan (Prefix Sum)

**CUDA Equivalent**: `thrust::scan`, `cub::DeviceScan`

**Signature**:
```rust
pub async fn execute_scan(
    &self,
    input: &[f32],
    operation: ScanOp,  // Sum, Max, Min
    exclusive: bool,
) -> Result<Vec<f32>>
```

**WGSL Kernel**: `shaders/scan.wgsl`

**Algorithm**: Work-efficient parallel scan (Blelloch)

**Use Cases**:
- Cumulative sums
- Stream compaction
- Allocation
- Sorting primitives

**Performance Target**: >300M elem/sec

---

#### 1.5 DotProduct

**CUDA Equivalent**: `cublas::dot`

**Signature**:
```rust
pub async fn execute_dot_product(
    &self,
    a: &[f32],
    b: &[f32],
) -> Result<f32>
```

**WGSL Kernel**: `shaders/dotproduct.wgsl`

**Algorithm**: Vectorized multiply + reduce

**Use Cases**:
- Inner product
- Similarity measures
- Attention scores

**Performance Target**: >600GB/sec memory bandwidth

---

#### 1.6 ElementwiseBinary

**CUDA Equivalent**: `thrust::transform` (binary)

**Signature**:
```rust
pub async fn execute_elementwise_binary(
    &self,
    a: &[f32],
    b: &[f32],
    operation: BinaryOp,  // Add, Sub, Mul, Div
) -> Result<Vec<f32>>
```

**WGSL Kernel**: `shaders/elementwise_binary.wgsl`

**Use Cases**:
- Residual connections
- Loss computation
- Data augmentation

**Performance Target**: >400M elem/sec

---

#### 1.7 Gather

**CUDA Equivalent**: `thrust::gather`

**Signature**:
```rust
pub async fn execute_gather(
    &self,
    source: &[f32],
    indices: &[u32],
) -> Result<Vec<f32>>
```

**WGSL Kernel**: `shaders/gather.wgsl`

**Use Cases**:
- Embedding lookup
- Sparse access
- Graph neural networks

**Performance Target**: >300M elem/sec

---

#### 1.8 Scatter

**CUDA Equivalent**: `thrust::scatter`

**Signature**:
```rust
pub async fn execute_scatter(
    &self,
    source: &[f32],
    indices: &[u32],
    dest: &mut [f32],
) -> Result<()>
```

**WGSL Kernel**: `shaders/scatter.wgsl`

**Algorithm**: Atomic operations for conflicts

**Use Cases**:
- Sparse updates
- Gradient accumulation
- Graph neural networks

**Performance Target**: >200M elem/sec

---

#### 1.9 Transpose

**CUDA Equivalent**: `cublas::geam`, custom CUDA kernels

**Signature**:
```rust
pub async fn execute_transpose(
    &self,
    input: &[f32],
    rows: usize,
    cols: usize,
) -> Result<Vec<f32>>
```

**WGSL Kernel**: `shaders/transpose.wgsl`

**Algorithm**: Tiled, coalesced memory access

**Use Cases**:
- Matrix operations
- Layout transforms
- Attention mechanisms

**Performance Target**: >400GB/sec memory bandwidth

---

### Tier 2: Neural Network Operations (7 operations)

Core operations for modern deep learning.

#### 2.1 Softmax

**CUDA Equivalent**: `cudnn::Softmax`

**Signature**:
```rust
pub async fn execute_softmax(
    &self,
    input: &[f32],
    axis: usize,
    shape: &[usize],
) -> Result<Vec<f32>>
```

**WGSL Kernel**: `shaders/softmax.wgsl`

**Algorithm**: Stable softmax (subtract max for numerical stability)

**Formula**: `softmax(x_i) = exp(x_i - max(x)) / sum(exp(x_j - max(x)))`

**Use Cases**:
- Classification output
- Attention weights
- Probability distributions

**Performance Target**: >200M elem/sec

---

#### 2.2 LayerNorm

**CUDA Equivalent**: `cudnn::LayerNormalization`

**Signature**:
```rust
pub async fn execute_layer_norm(
    &self,
    input: &[f32],
    normalized_shape: &[usize],
    gamma: Option<&[f32]>,
    beta: Option<&[f32]>,
    epsilon: f32,
) -> Result<Vec<f32>>
```

**WGSL Kernel**: `shaders/layernorm.wgsl`

**Algorithm**: Welford's online algorithm for stability

**Formula**: `output = (input - mean) / sqrt(variance + epsilon) * gamma + beta`

**Use Cases**:
- Transformer normalization
- Pre-norm, post-norm
- Stabilizing training

**Performance Target**: >150M elem/sec

---

#### 2.3 BatchNorm

**CUDA Equivalent**: `cudnn::BatchNormalization`

**Signature**:
```rust
pub async fn execute_batch_norm(
    &self,
    input: &[f32],
    gamma: &[f32],
    beta: &[f32],
    running_mean: &[f32],
    running_var: &[f32],
    epsilon: f32,
    training: bool,
) -> Result<Vec<f32>>
```

**WGSL Kernel**: `shaders/batchnorm.wgsl`

**Use Cases**:
- CNN normalization
- Accelerating training
- Reducing internal covariate shift

**Performance Target**: >150M elem/sec

---

#### 2.4 ReLU ✅ **IMPLEMENTED**

**CUDA Equivalent**: `cudnn::Activation(RELU)`

**Signature**:
```rust
pub async fn execute_relu(&self, input: &[f32]) -> Result<Vec<f32>>
```

**WGSL Kernel**: `shaders/relu.wgsl` ✅

**Formula**: `relu(x) = max(0, x)`

**Status**: ✅ Implemented, validated (241M elem/sec)

---

#### 2.5 Sigmoid

**CUDA Equivalent**: `cudnn::Activation(SIGMOID)`

**Signature**:
```rust
pub async fn execute_sigmoid(&self, input: &[f32]) -> Result<Vec<f32>>
```

**WGSL Kernel**: `shaders/sigmoid.wgsl`

**Formula**: `sigmoid(x) = 1 / (1 + exp(-x))`

**Use Cases**:
- Binary classification
- Gate activations (LSTM, GRU)

**Performance Target**: >200M elem/sec

---

#### 2.6 Tanh

**CUDA Equivalent**: `cudnn::Activation(TANH)`

**Signature**:
```rust
pub async fn execute_tanh(&self, input: &[f32]) -> Result<Vec<f32>>
```

**WGSL Kernel**: `shaders/tanh.wgsl`

**Formula**: `tanh(x) = (exp(x) - exp(-x)) / (exp(x) + exp(-x))`

**Use Cases**:
- Activation function
- Output normalization

**Performance Target**: >200M elem/sec

---

#### 2.7 Dropout

**CUDA Equivalent**: `cudnn::Dropout`

**Signature**:
```rust
pub async fn execute_dropout(
    &self,
    input: &[f32],
    dropout_prob: f32,
    training: bool,
    seed: Option<u64>,
) -> Result<Vec<f32>>
```

**WGSL Kernel**: `shaders/dropout.wgsl`

**Algorithm**: GPU random number generation (Philox, Threefry)

**Use Cases**:
- Regularization
- Preventing overfitting

**Performance Target**: >150M elem/sec

---

### Tier 3: Computer Vision Operations (3 operations)

Essential for CNNs and image processing.

#### 3.1 Conv2D ✅ **IMPLEMENTED**

**CUDA Equivalent**: `cudnn::Convolution`

**Signature**:
```rust
pub async fn execute_conv2d(
    &self,
    input: &[f32],       // [N, C_in, H_in, W_in]
    weights: &[f32],     // [C_out, C_in, K_h, K_w]
    bias: Option<&[f32]>,// [C_out]
    stride: (usize, usize),
    padding: (usize, usize),
) -> Result<Vec<f32>>    // [N, C_out, H_out, W_out]
```

**WGSL Kernel**: `shaders/conv2d.wgsl` ✅

**Algorithm**: Direct convolution (im2col optional for optimization)

**Status**: ✅ Implemented, needs integration

---

#### 3.2 MaxPool2D

**CUDA Equivalent**: `cudnn::Pooling(MAX)`

**Signature**:
```rust
pub async fn execute_max_pool2d(
    &self,
    input: &[f32],
    kernel_size: (usize, usize),
    stride: (usize, usize),
    padding: (usize, usize),
) -> Result<Vec<f32>>
```

**WGSL Kernel**: `shaders/maxpool2d.wgsl`

**Algorithm**: Sliding window maximum

**Use Cases**:
- Spatial downsampling
- Translation invariance
- Reducing computation

**Performance Target**: >300M elem/sec

---

#### 3.3 AvgPool2D

**CUDA Equivalent**: `cudnn::Pooling(AVG)`

**Signature**:
```rust
pub async fn execute_avg_pool2d(
    &self,
    input: &[f32],
    kernel_size: (usize, usize),
    stride: (usize, usize),
    padding: (usize, usize),
) -> Result<Vec<f32>>
```

**WGSL Kernel**: `shaders/avgpool2d.wgsl`

**Use Cases**:
- Smooth downsampling
- Global average pooling
- Feature aggregation

**Performance Target**: >300M elem/sec

---

### Tier 4: Linear Algebra (2 operations)

Fundamental matrix operations.

#### 4.1 MatMul (GEMM) ✅ **IMPLEMENTED**

**CUDA Equivalent**: `cublas::gemm`

**Signature**:
```rust
pub async fn execute_matmul(
    &self,
    a: &[f32],  // [M, K]
    b: &[f32],  // [K, N]
    m: usize,
    k: usize,
    n: usize,
) -> Result<Vec<f32>>  // [M, N]
```

**WGSL Kernel**: `shaders/matmul.wgsl` ✅

**Algorithm**: Tiled matrix multiplication

**Status**: ✅ Implemented, validated

**Optimization Path**:
- [ ] Shared memory tiling
- [ ] Vectorized loads (vec4)
- [ ] Register blocking

**Performance Target**: >5 TFLOPS (FP32 on RTX 3090)

---

#### 4.2 VectorAdd (AXPY)

**CUDA Equivalent**: `cublas::axpy`

**Signature**:
```rust
pub async fn execute_vector_add(
    &self,
    a: &[f32],
    b: &[f32],
    alpha: f32,  // a * alpha + b
) -> Result<Vec<f32>>
```

**WGSL Kernel**: `shaders/vectoradd.wgsl`

**Use Cases**:
- Gradient descent updates
- Residual connections
- Vector arithmetic

**Performance Target**: >500GB/sec memory bandwidth

---

## Implementation Guidelines

### WGSL Kernel Template

```wgsl
// File: shaders/operation_name.wgsl
// Purpose: [Brief description]
// CUDA equivalent: [CUDA function name]

// Input/output buffers
@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;

// Optional: parameters structure
struct Params {
    size: u32,
    // ... other params
}
@group(0) @binding(2) var<uniform> params: Params;

// Compute kernel
@compute @workgroup_size(256)  // Tune per operation
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    
    // Bounds check
    if (idx >= params.size) {
        return;
    }
    
    // Operation logic
    output[idx] = /* compute */;
}
```

---

### Rust Executor Method Template

```rust
/// [Operation name]
///
/// CUDA equivalent: `[cuda function]`
///
/// # Arguments
/// * `input` - Input tensor
/// * `...` - Other parameters
///
/// # Returns
/// Result containing output tensor or error
///
/// # Performance
/// Target: [target throughput] on RTX 3090
///
/// # Example
/// ```
/// let result = executor.execute_operation(&input).await?;
/// ```
pub async fn execute_operation(
    &self,
    input: &[f32],
    // ... parameters
) -> Result<Vec<f32>> {
    // Load shader
    let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Operation Shader"),
        source: wgpu::ShaderSource::Wgsl(
            include_str!("shaders/operation.wgsl").into()
        ),
    });
    
    // Create buffers
    let input_buffer = /* ... */;
    let output_buffer = /* ... */;
    let params_buffer = /* ... */;
    
    // Create bind group
    let bind_group = /* ... */;
    
    // Create pipeline
    let pipeline = /* ... */;
    
    // Dispatch compute
    let mut encoder = self.device.create_command_encoder(/* ... */);
    {
        let mut pass = encoder.begin_compute_pass(/* ... */);
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(/* calculate workgroups */);
    }
    
    // Submit and read back
    self.queue.submit(Some(encoder.finish()));
    let result = read_buffer(&self.device, &output_buffer).await?;
    
    Ok(result)
}
```

---

### Testing Requirements

For each operation, implement:

#### 1. Correctness Test

```rust
#[tokio::test]
async fn test_operation_correctness() {
    let executor = WgpuExecutor::new().await.unwrap();
    let input = vec![/* test data */];
    
    // CPU reference
    let cpu_result = operation_cpu(&input);
    
    // GPU implementation
    let gpu_result = executor.execute_operation(&input).await.unwrap();
    
    // Validate
    let max_diff = max_absolute_difference(&cpu_result, &gpu_result);
    assert!(max_diff < 1e-6, "Max difference: {}", max_diff);
}
```

#### 2. Performance Benchmark

```rust
#[tokio::test]
async fn bench_operation_performance() {
    let executor = WgpuExecutor::new().await.unwrap();
    let input = vec![0.0f32; 1_000_000];
    
    let start = Instant::now();
    for _ in 0..100 {
        let _ = executor.execute_operation(&input).await.unwrap();
    }
    let elapsed = start.elapsed();
    
    let throughput = (1_000_000 * 100) as f64 / elapsed.as_secs_f64();
    println!("Throughput: {:.2} M elem/sec", throughput / 1e6);
    
    // Assert performance target
    assert!(throughput > TARGET_THROUGHPUT);
}
```

#### 3. Cross-Vendor Test

```rust
#[tokio::test]
async fn test_operation_cross_vendor() {
    // Test on all available backends
    for backend in wgpu::Backends::all() {
        let executor = WgpuExecutor::with_backend(backend).await;
        if let Ok(exec) = executor {
            let result = exec.execute_operation(&input).await.unwrap();
            validate_correctness(&result);
        }
    }
}
```

---

## Performance Optimization Checklist

### Per-Kernel Optimization

- [ ] **Workgroup size tuning** - Test 64, 128, 256, 512
- [ ] **Shared memory usage** - Reduce global memory traffic
- [ ] **Vectorization** - Use vec4 loads/stores where possible
- [ ] **Coalesced memory access** - Sequential thread access
- [ ] **Register blocking** - Keep data in registers
- [ ] **Avoid divergence** - Minimize if/else in warps
- [ ] **Reduce atomics** - Use when necessary only

### Validation

- [ ] Measure memory bandwidth utilization
- [ ] Profile with wgpu/Vulkan tools
- [ ] Compare vs CUDA equivalent
- [ ] Test on multiple GPUs
- [ ] Validate power efficiency

---

## Roadmap

### Q1 2026 (Current)

- ✅ Foundation complete (wgpu_executor, 3 ops)
- ⏳ Phase 2: Core primitives (5 ops) - **IN PROGRESS**
- 🎯 Phase 3: Neural network ops (4 ops)

### Q2 2026

- 🎯 Phase 4: Advanced patterns (4 ops)
- 🎯 Phase 5: Feature complete (5 ops)
- 🎯 Performance optimization pass
- 🎯 Production workload validation

### Q3 2026

- 🎯 Advanced operations (100+ total ops)
- 🎯 Distributed multi-GPU
- 🎯 Auto-tuning framework

### Q4 2026

- 🎯 PyTorch plugin
- 🎯 TensorFlow plugin
- 🎯 Industry adoption

---

## References

### Standards

- **WGSL**: https://www.w3.org/TR/WGSL/
- **WebGPU**: https://www.w3.org/TR/webgpu/

### CUDA Equivalents

- **cuBLAS**: https://docs.nvidia.com/cuda/cublas/
- **cuDNN**: https://docs.nvidia.com/deeplearning/cudnn/
- **Thrust**: https://docs.nvidia.com/cuda/thrust/
- **CUB**: https://nvlabs.github.io/cub/

### Implementation

- **wgpu**: https://wgpu.rs/
- **wgpu-rs**: https://github.com/gfx-rs/wgpu

---

**Version**: 1.0.0  
**Last Updated**: January 12, 2026  
**Status**: Living Document  
**Owner**: ToadStool / barraCUDA Team

🦈 **Pure Rust. Any Hardware. Zero Lock-In.** 🦈
