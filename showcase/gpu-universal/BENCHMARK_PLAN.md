# 🏁 ToadStool vs ZLUDA/SCALE Benchmark Plan

**Date**: January 7, 2026  
**Goal**: Learn from each other through comprehensive benchmarking  
**Status**: Planning & Infrastructure Setup

---

## 🎯 Objectives

### 1. Performance Comparison
- Measure raw compute performance across implementations
- Identify optimization opportunities
- Share learnings with ZLUDA and SCALE teams

### 2. Compatibility Testing
- Test same workloads across all three systems
- Identify gaps in API coverage
- Document translation approaches

### 3. Developer Experience
- Compare ease of implementation
- Measure compile times, setup complexity
- Document user friction points

### 4. Architectural Learning
- Understand different approaches to vendor abstraction
- Share best practices
- Contribute to ecosystem evolution

---

## 📊 Benchmark Categories

### Category 1: Basic Operations ✅ (We Have Matrix Multiply)

**Operations**:
- ✅ Matrix multiplication (GEMM) - ToadStool: 121,788 img/sec
- 🚧 Vector addition
- 🚧 Vector dot product
- 🚧 Element-wise operations (add, mul, div)
- 🚧 Reductions (sum, max, min)

**Metrics**:
- Throughput (GFLOPS)
- Latency (μs)
- Memory bandwidth utilization (GB/s)
- Energy efficiency (GFLOPS/W)

**Systems to Benchmark**:
1. ToadStool (OpenCL on NVIDIA)
2. ToadStool (Vulkan on AMD) - after 5-6h implementation
3. ZLUDA (CUDA on AMD)
4. SCALE (CUDA on AMD)
5. Native CUDA (NVIDIA baseline)
6. Native ROCm (AMD baseline)

### Category 2: Neural Network Operations

**Operations**:
- ✅ Matrix multiply - DONE
- ✅ ReLU activation - DONE
- ✅ Softmax - DONE
- 🚧 Convolution (Conv2D, Conv3D)
- 🚧 Pooling (MaxPool, AvgPool)
- 🚧 Batch normalization
- 🚧 Dropout

**Workloads**:
- ✅ MNIST inference (simple network) - 121,788 img/sec
- 🚧 ResNet-50 inference
- 🚧 BERT transformer
- 🚧 GPT-style attention

**Metrics**:
- Images/second (inference)
- Samples/second (training)
- Memory usage (MB)
- Accuracy (verify correctness)

### Category 3: Scientific Computing

**Operations**:
- FFT (Fast Fourier Transform)
- Linear algebra (BLAS Level 1, 2, 3)
- Sparse matrix operations
- N-body simulations
- Monte Carlo simulations

**Metrics**:
- Throughput (operations/sec)
- Numerical accuracy
- Memory efficiency

### Category 4: Image Processing

**Operations**:
- Gaussian blur
- Edge detection (Sobel, Canny)
- Histogram equalization
- Color space conversions
- Morphological operations

**Metrics**:
- Images/second
- Latency per operation
- Quality (PSNR, SSIM)

### Category 5: Real-World Applications

**Applications**:
- Blender rendering (if ZLUDA/SCALE support)
- Hashcat password cracking
- Video encoding/decoding
- Molecular dynamics
- Computational fluid dynamics

**Metrics**:
- Render time (seconds)
- Hashes/second
- Frames/second
- Simulation steps/second

---

## 🏗️ Benchmark Infrastructure

### Phase 1: Setup (NOW - 1 day)

**Task 1.1: Install ZLUDA**
```bash
# ZLUDA installation
git clone https://github.com/vosen/ZLUDA.git
cd ZLUDA
cargo build --release

# Test on AMD GPU
export LD_LIBRARY_PATH=/path/to/ZLUDA:$LD_LIBRARY_PATH
# Run CUDA apps on AMD
```

**Task 1.2: Install SCALE**
```bash
# SCALE installation (if publicly available)
# Check: https://spectralcompute.co.uk/scale/
# Note: May require registration/license
```

**Task 1.3: Setup Common Benchmark Harness**
```rust
// benchmark_harness/src/main.rs
pub trait GpuBackend {
    fn name(&self) -> &str;
    fn initialize(&mut self) -> Result<()>;
    fn execute_workload(&self, workload: &Workload) -> Result<BenchmarkResult>;
}

pub struct ToadStoolBackend { /* ... */ }
pub struct ZLUDABackend { /* ... */ }
pub struct SCALEBackend { /* ... */ }
pub struct NativeCUDABackend { /* ... */ }
pub struct NativeROCmBackend { /* ... */ }
```

### Phase 2: Basic Operation Benchmarks (1-2 days)

**Workload**: Vector Addition
```rust
// Simplest possible workload
pub struct VectorAdd {
    size: usize,
    a: Vec<f32>,
    b: Vec<f32>,
}

impl Workload for VectorAdd {
    fn to_toadstool(&self) -> ToadStoolWorkload;
    fn to_cuda_source(&self) -> String;  // For ZLUDA/SCALE
    fn to_rocm_source(&self) -> String;  // For native ROCm
}
```

**Benchmark Script**:
```bash
#!/bin/bash
# Run on all backends
for size in 1024 4096 16384 65536 262144 1048576; do
    echo "Size: $size"
    ./bench vectoradd --size $size --backend toadstool-opencl
    ./bench vectoradd --size $size --backend toadstool-vulkan
    ./bench vectoradd --size $size --backend zluda
    ./bench vectoradd --size $size --backend scale
    ./bench vectoradd --size $size --backend cuda-native
    ./bench vectoradd --size $size --backend rocm-native
done
```

### Phase 3: Neural Network Benchmarks (3-5 days)

**Workload**: MNIST Inference
- ✅ ToadStool implementation ready (121,788 img/sec)
- 🚧 Port to CUDA source for ZLUDA/SCALE
- 🚧 Implement native ROCm version
- 🚧 Run comparisons

**Expected Results**:
```
System                    | Throughput      | Memory  | Power
--------------------------|-----------------|---------|-------
ToadStool (OpenCL/NVIDIA) | 121,788 img/sec | 100 MB  | ?W
ToadStool (Vulkan/AMD)    | ~85,000 img/sec | 100 MB  | ?W
ZLUDA (AMD)               | ? img/sec       | ?MB     | ?W
SCALE (AMD)               | ? img/sec       | ?MB     | ?W
Native CUDA (NVIDIA)      | ? img/sec       | ?MB     | ?W
Native ROCm (AMD)         | ? img/sec       | ?MB     | ?W
```

### Phase 4: Comprehensive Report (1-2 days)

**Deliverable**: Comparative Analysis Report
- Performance comparison tables
- Architectural analysis
- Lessons learned
- Optimization recommendations
- Contribution opportunities

---

## 🔬 Specific Benchmarks

### Benchmark 1: Vector Addition (Baseline)

**Purpose**: Simplest possible workload, tests raw overhead

**CUDA Source** (for ZLUDA/SCALE):
```cuda
__global__ void vectorAdd(float *a, float *b, float *c, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) c[i] = a[i] + b[i];
}
```

**ToadStool OpenCL** (vendor-agnostic):
```opencl
__kernel void vectorAdd(__global float* a, __global float* b, 
                        __global float* c, int n) {
    int i = get_global_id(0);
    if (i < n) c[i] = a[i] + b[i];
}
```

**Test Sizes**: 1K, 4K, 16K, 64K, 256K, 1M, 4M, 16M elements

**Metrics**:
- Kernel launch overhead
- Memory transfer time
- Compute time
- Total throughput (GB/s)

### Benchmark 2: Matrix Multiplication

**Purpose**: Core operation, tests compute performance

**Status**: ✅ ToadStool implementation ready (121,788 img/sec)

**Sizes to Test**:
- Small: 128x128
- Medium: 512x512
- Large: 2048x2048
- Huge: 4096x4096

**Metrics**:
- GFLOPS (Giga Floating-Point Operations Per Second)
- Memory bandwidth utilization
- Percentage of peak performance

### Benchmark 3: MNIST Neural Network

**Purpose**: Real ML workload, end-to-end

**Status**: ✅ ToadStool implementation ready

**Network Architecture**:
- Input: 784 (28x28 image)
- Hidden: 128 (ReLU)
- Output: 10 (Softmax)
- Total params: ~100K

**Batch Sizes**: 1, 8, 16, 32, 64, 128, 256

**Metrics**:
- Images/second
- Latency/image
- Accuracy (verify correctness)
- Memory usage

### Benchmark 4: Convolution

**Purpose**: Core CNN operation

**Status**: 🚧 To implement

**Test Cases**:
- Conv2D: 224x224x3 → 224x224x64 (kernel=3x3, stride=1)
- Conv2D: 112x112x64 → 112x112x128 (kernel=3x3, stride=1)
- Conv2D: 56x56x128 → 28x28x256 (kernel=3x3, stride=2)

**Metrics**:
- GFLOPS
- Latency
- Memory bandwidth

---

## 🤝 Collaboration Opportunities

### With ZLUDA Team

**What We Can Learn**:
- CUDA → ROCm translation strategies
- Binary compatibility approaches
- Handling CUDA-specific extensions

**What We Can Share**:
- OpenCL optimization techniques
- Vulkan compute best practices
- Cross-platform abstraction patterns

**Collaboration Ideas**:
- Share benchmark results
- Coordinate on workload coverage
- Joint optimization efforts
- Cross-reference documentation

### With SCALE Team

**What We Can Learn**:
- Compiler-based translation
- PTX → LLVM → AMD translation
- Enterprise deployment strategies

**What We Can Share**:
- Open-source implementation insights
- Runtime discovery patterns
- Multi-vendor testing results

**Collaboration Ideas**:
- Benchmark result comparison
- API design discussions
- Performance optimization sharing
- Community building

### With Broader Community

**Open Benchmarks**:
- Publish all benchmark code
- Share raw results
- Document methodologies
- Accept contributions

**Community Benefits**:
- Breaks vendor lock-in narrative
- Demonstrates multiple viable paths
- Encourages innovation
- Improves all implementations

---

## 📋 Implementation Plan

### Week 1: Infrastructure (NOW)

**Day 1-2**: Setup
- ✅ ToadStool ready (121,788 img/sec verified)
- 🚧 Install ZLUDA on AMD GPU
- 🚧 Install SCALE (if available)
- 🚧 Setup benchmark harness

**Day 3-4**: Vector Addition
- 🚧 Implement in ToadStool
- 🚧 Run on ZLUDA/SCALE
- 🚧 Compare results
- 🚧 Document differences

**Day 5-7**: Matrix Multiply
- ✅ ToadStool ready
- 🚧 Port to CUDA for ZLUDA/SCALE
- 🚧 Run comparisons
- 🚧 Analysis

### Week 2: Neural Networks

**Day 8-10**: MNIST Inference
- ✅ ToadStool ready (121,788 img/sec)
- 🚧 Complete Vulkan compute (AMD at full speed)
- 🚧 Port to CUDA for ZLUDA/SCALE
- 🚧 Comprehensive comparison

**Day 11-14**: Convolution Operations
- 🚧 Implement Conv2D in ToadStool
- 🚧 Test on ZLUDA/SCALE
- 🚧 Performance analysis
- 🚧 Optimization iteration

### Week 3: Analysis & Report

**Day 15-17**: Data Analysis
- Aggregate all results
- Statistical analysis
- Performance profiling
- Bottleneck identification

**Day 18-21**: Report & Sharing
- Write comprehensive report
- Create visualizations
- Share with ZLUDA/SCALE teams
- Publish findings

---

## 📊 Expected Outcomes

### Performance Insights

**Hypothesis**:
- ToadStool will be competitive on vendor-native APIs (OpenCL/Vulkan)
- ZLUDA/SCALE may have overhead from CUDA translation
- All will be significantly faster than CPU
- Specific workloads will favor specific approaches

**Metrics to Compare**:
1. **Raw Speed**: GFLOPS, throughput
2. **Memory Efficiency**: Bandwidth utilization
3. **Latency**: Single operation time
4. **Scalability**: Performance across batch sizes
5. **Energy**: GFLOPS/Watt (if measurable)

### Learning Opportunities

**For ToadStool**:
- Learn CUDA translation techniques
- Identify optimization gaps
- Improve API design
- Find missing features

**For ZLUDA/SCALE**:
- Learn OpenCL/Vulkan patterns
- Cross-vendor insights
- Alternative architectures
- Community feedback

**For Community**:
- Multiple viable paths proven
- Vendor lock-in defeated
- Open collaboration model
- Shared innovation

---

## 🎯 Success Criteria

### Minimum Success
- ✅ Run same workload on all 3 systems
- ✅ Document performance differences
- ✅ Share results openly

### Target Success
- ✅ Comprehensive benchmark suite
- ✅ Detailed performance analysis
- ✅ Collaboration with ZLUDA/SCALE teams
- ✅ Optimization insights

### Stretch Success
- ✅ Joint optimization efforts
- ✅ Shared benchmark infrastructure
- ✅ Community benchmark standard
- ✅ Ongoing collaboration

---

## 🚀 Quick Start

### For ToadStool Benchmarking

```bash
# Already working!
cd showcase/gpu-universal/ml-inference
cargo build --release --features opencl,vulkan
./target/release/dual-gpu-demo

# Output: 121,788 img/sec (17.3x speedup)
```

### For ZLUDA Installation

```bash
# Install ZLUDA
git clone https://github.com/vosen/ZLUDA.git
cd ZLUDA
cargo build --release

# Set library path
export LD_LIBRARY_PATH=/path/to/ZLUDA/target/release:$LD_LIBRARY_PATH

# Run CUDA app on AMD
./cuda_app  # Will use ZLUDA automatically
```

### For SCALE Installation

```bash
# Check availability
# https://spectralcompute.co.uk/scale/

# Install (if available)
# Follow SCALE documentation

# Run CUDA app on AMD
nvcc -o app app.cu  # Uses SCALE compiler
./app               # Runs on AMD GPU
```

---

## 📞 Contact & Collaboration

### ToadStool Team
- **GitHub**: ecoPrimals/toadStool
- **Docs**: showcase/gpu-universal/
- **Status**: Production-ready, 121,788 img/sec verified

### ZLUDA Project
- **GitHub**: vosen/ZLUDA
- **Status**: Open-source, active development
- **Community**: Growing rapidly

### SCALE Toolkit
- **Website**: spectralcompute.co.uk/scale
- **Status**: Commercial offering
- **Contact**: Via website

---

## 💡 Next Steps

### Immediate (Today)
1. ✅ Document current ToadStool performance
2. 🚧 Setup ZLUDA on AMD GPU
3. 🚧 Test simple workload (vectorAdd)

### Short-Term (This Week)
1. 🚧 Implement vectorAdd in ToadStool
2. 🚧 Complete Vulkan compute (5-6h)
3. 🚧 Run MNIST on ZLUDA/SCALE

### Medium-Term (2-3 Weeks)
1. 🚧 Comprehensive benchmark suite
2. 🚧 Performance analysis
3. 🚧 Share results with teams

---

**ToadStool Team - January 7, 2026**

*"Collaboration over competition."*  
*"Learn from each other, improve together."*  
*"Break vendor lock-in through open innovation."*

