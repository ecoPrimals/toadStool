# 🔬 Benchmark Execution Plan

**Date**: January 7, 2026 (Evening)  
**Goal**: Comprehensive GPU benchmarking across vendors and frameworks  
**Status**: IN PROGRESS

---

## 🎯 Benchmark Objectives

### 1. RTX 3090 vs RX 6950 XT
**Compare performance across vendors**:
- NVIDIA RTX 3090 (10496 CUDA cores, 24GB)
- AMD RX 6950 XT (5120 stream processors, 16GB)

**Backends to test**:
- OpenCL (both GPUs)
- Vulkan (both GPUs)
- CUDA (NVIDIA only, for baseline)

### 2. Cross-GPU Vendor Workloads
**Distribute workload across both GPUs**:
- Parallel execution (NVIDIA + AMD simultaneously)
- Work splitting strategies
- Load balancing
- Aggregate throughput

### 3. ZLUDA Comparison
**AMD GPU running CUDA code via ZLUDA**:
- ZLUDA layer translating CUDA → ROCm
- Compare against native OpenCL
- Compare against Vulkan
- Identify translation overhead

### 4. SCALE Comparison
**AMD GPU running CUDA code via SCALE**:
- SCALE toolkit translation
- Performance vs ZLUDA
- Performance vs native
- Commercial vs open-source approach

---

## 📊 Benchmark Workloads

### Primary Workload: MNIST Inference
**Why**: Already implemented, verified, production-ready
- LeNet-5 CNN (Conv→ReLU→MaxPool→FC→Softmax)
- vectorAdd (simple baseline)
- Conv2D operations (compute-intensive)

**Metrics**:
- Throughput (images/sec or elements/sec)
- Latency (ms per operation)
- GPU utilization (%)
- Memory usage (MB)
- Power consumption (watts, if available)

### Secondary Workloads
- Matrix multiplication (various sizes)
- Reduction operations
- Memory bandwidth tests
- Kernel launch overhead

---

## 🔧 Implementation Strategy

### Phase 1: Single-GPU Benchmarks (1-2 hours)
**Establish baselines for each GPU**:

1. **NVIDIA RTX 3090**:
   - ✅ OpenCL (verified: 121,788 img/sec)
   - → Vulkan (infrastructure ready)
   - → CUDA (for reference)

2. **AMD RX 6950 XT**:
   - → OpenCL (needs device selection fix)
   - → Vulkan (infrastructure ready)
   - → ROCm (if available)

3. **Benchmark Suite**:
   - vectorAdd (1M, 10M, 100M elements)
   - Conv2D (various sizes)
   - MNIST inference (batch sizes: 16, 64, 256)
   - Memory bandwidth

### Phase 2: Cross-GPU Workloads (1-2 hours)
**Parallel execution across vendors**:

1. **Work Distribution**:
   - Split MNIST batches across GPUs
   - Measure aggregate throughput
   - Identify optimal split ratio

2. **Async Execution**:
   - Tokio async/await
   - Concurrent kernel launches
   - Result aggregation

3. **Load Balancing**:
   - Static split (50/50)
   - Dynamic (performance-based)
   - Capability-based

### Phase 3: ZLUDA Benchmarking (2-3 hours)
**AMD GPU via CUDA translation**:

1. **Setup**:
   - Build ZLUDA from source (already cloned)
   - Set up LD_LIBRARY_PATH
   - Verify CUDA detection

2. **Benchmarks**:
   - Run same MNIST workload
   - Compare vs native OpenCL
   - Compare vs Vulkan
   - Measure translation overhead

3. **Analysis**:
   - Identify bottlenecks
   - Feature support gaps
   - Performance characteristics

### Phase 4: SCALE Comparison (2-3 hours)
**Commercial CUDA translation**:

1. **Setup**:
   - Obtain SCALE toolkit (if available)
   - Configure for RX 6950 XT
   - Verify installation

2. **Benchmarks**:
   - Same workloads as ZLUDA
   - Direct performance comparison
   - Feature compatibility

3. **Analysis**:
   - ZLUDA vs SCALE
   - Commercial vs open-source
   - Cost-benefit analysis

---

## 📈 Success Metrics

### Performance Targets

**Single-GPU** (vs CPU baseline):
- vectorAdd: >2x speedup
- Conv2D: >4x speedup
- MNIST: >15x speedup

**Cross-GPU** (vs single GPU):
- Linear scaling: 1.8-2.0x
- Acceptable scaling: >1.5x

**ZLUDA/SCALE** (vs native):
- Translation overhead: <20%
- Feature support: >90%

### Quality Targets
- All benchmarks reproducible
- Statistical significance (10+ runs)
- Clear documentation
- Automated benchmark suite

---

## 🛠️ Implementation Details

### Benchmark Framework

```rust
pub struct BenchmarkResult {
    pub name: String,
    pub gpu: GpuInfo,
    pub backend: GpuBackend,
    pub throughput: f64,       // operations/sec
    pub latency_ms: f64,       // milliseconds
    pub memory_mb: f64,        // megabytes
    pub runs: usize,
    pub std_dev: f64,
}

pub struct BenchmarkSuite {
    pub workloads: Vec<Workload>,
    pub gpus: Vec<GpuInfo>,
    pub backends: Vec<GpuBackend>,
}

impl BenchmarkSuite {
    pub async fn run_all(&self) -> Vec<BenchmarkResult> {
        // Run all combinations
        // Collect statistics
        // Generate report
    }
}
```

### Cross-GPU Executor

```rust
pub struct CrossGpuExecutor {
    pub gpus: Vec<(GpuInfo, Box<dyn Executor>)>,
}

impl CrossGpuExecutor {
    pub async fn execute_distributed(
        &self,
        workload: &Workload,
        split_strategy: SplitStrategy,
    ) -> Result<AggregateResult> {
        // Split workload
        // Launch on all GPUs in parallel
        // Aggregate results
    }
}
```

---

## 📋 Current Status

### Available Now
- ✅ NVIDIA RTX 3090 (OpenCL verified: 121,788 img/sec)
- ✅ AMD RX 6950 XT (Vulkan discovered, infrastructure ready)
- ✅ MNIST inference workload (production-ready)
- ✅ vectorAdd workload (2.27x speedup verified)
- ✅ Conv2D workload (4.37x speedup verified)
- ✅ ZLUDA source code (cloned, ready to build)

### Needs Setup
- → OpenCL on AMD (device selection issue)
- → CUDA baseline on NVIDIA
- → Vulkan compute execution (infrastructure ready)
- → ZLUDA build and configuration
- → SCALE toolkit (availability TBD)

---

## 🎯 Execution Order

### Immediate (Now)
1. Create benchmark framework
2. Implement automated benchmark runner
3. Run NVIDIA baselines (OpenCL, Vulkan, CUDA)
4. Document results

### Short-Term (Next 2-4 hours)
1. Fix AMD OpenCL (if possible)
2. Implement Vulkan compute execution
3. Run AMD benchmarks (Vulkan)
4. Run cross-GPU parallel execution

### Medium-Term (Next 4-8 hours)
1. Build and configure ZLUDA
2. Run ZLUDA benchmarks on AMD
3. Compare ZLUDA vs native
4. Document findings

### Long-Term (If Available)
1. Obtain SCALE toolkit
2. Run SCALE benchmarks
3. Compare all approaches
4. Final comprehensive report

---

## 📊 Expected Results

### Single-GPU Performance
**NVIDIA RTX 3090**:
- OpenCL: ~120,000 img/sec (verified)
- Vulkan: ~110,000 img/sec (estimated)
- CUDA: ~130,000 img/sec (estimated, native)

**AMD RX 6950 XT**:
- OpenCL: ~60,000 img/sec (estimated)
- Vulkan: ~70,000 img/sec (estimated)
- ROCm: ~75,000 img/sec (estimated, native)

### Cross-GPU Aggregate
- NVIDIA + AMD: ~180,000-200,000 img/sec
- Scaling efficiency: 1.5-1.7x

### Translation Layers
**ZLUDA** (AMD via CUDA):
- Performance: 50,000-60,000 img/sec
- Overhead: 10-20% vs native
- Status: Open-source, evolving

**SCALE** (AMD via CUDA):
- Performance: 60,000-70,000 img/sec
- Overhead: 5-10% vs native
- Status: Commercial, optimized

---

## 🎓 Learning Objectives

### Technical Insights
1. Real-world GPU performance across vendors
2. Translation layer overhead quantification
3. Cross-GPU workload distribution strategies
4. Backend selection heuristics

### Strategic Insights
1. Vendor lock-in cost assessment
2. Open-source (ZLUDA) vs commercial (SCALE)
3. Native vs translated performance
4. ToadStool positioning vs alternatives

### Documentation
1. Comprehensive benchmark results
2. Reproducible methodology
3. Clear recommendations
4. Future optimization paths

---

## 🏆 Success Criteria

### Must Have
- ✅ Benchmark framework implemented
- ✅ NVIDIA baselines established
- ✅ AMD benchmarks (at least one backend)
- ✅ Cross-GPU execution working
- ✅ Results documented

### Should Have
- ✅ Multiple backends per GPU
- ✅ ZLUDA comparison
- ✅ Statistical significance
- ✅ Performance analysis

### Nice to Have
- SCALE comparison
- Power consumption metrics
- Automated CI integration
- Public benchmark suite

---

**ToadStool Team - January 7, 2026**

*"Comprehensive benchmarking: RTX vs RX, ZLUDA, SCALE"*  
*"Measure everything. Compare everything. Learn everything."*

