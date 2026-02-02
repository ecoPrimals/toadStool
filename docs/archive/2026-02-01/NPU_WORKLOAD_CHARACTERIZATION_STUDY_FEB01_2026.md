# 🔬 NPU WORKLOAD CHARACTERIZATION STUDY
## Understanding Akida's Strengths, Weaknesses, and Specializations

**Date**: February 1, 2026  
**Goal**: Determine WHY NPU dominates and identify its specialization domains  
**Status**: 🔄 Experimental Design Phase

═══════════════════════════════════════════════════════════════════════════════

## 🎯 RESEARCH QUESTIONS

### Primary Questions
1. **Is NPU dominance specific to homomorphic encryption?**
   - Or does it apply to other sparse/event-driven workloads?
   
2. **What computational patterns does NPU excel at?**
   - Sparse data processing?
   - Event-driven computation?
   - Sequential patterns?
   - Branching logic?

3. **What workloads does NPU struggle with?**
   - Dense matrix operations?
   - Memory-bound tasks?
   - High bandwidth requirements?
   - Complex dependencies?

4. **What makes Akida different from GPU/CPU?**
   - Architecture characteristics?
   - Memory hierarchy?
   - Execution model?
   - Parallelism strategy?

═══════════════════════════════════════════════════════════════════════════════

## 🧪 EXPERIMENTAL DESIGN - WORKLOAD DIVERSITY

### Category 1: SPARSE vs DENSE Operations

#### Test 1A: Sparse Matrix-Vector Multiplication
```
Sparsity levels: 99%, 95%, 90%, 75%, 50%, 25%, 10%, 5%
Matrix size: 1024×1024
Operations: y = A * x (where A is sparse)

Hypothesis: NPU advantage increases with sparsity
Expected: GPU dominates at <20% sparsity, NPU at >80%
```

#### Test 1B: Dense Matrix-Matrix Multiplication
```
Matrix sizes: 256×256, 512×512, 1024×1024
Operations: C = A * B (dense GEMM)

Hypothesis: GPU dominates dense linear algebra
Expected: GPU >> NPU for dense operations
```

---

### Category 2: EVENT-DRIVEN vs STREAMING

#### Test 2A: Spike-Time Processing (NPU Native)
```
Workload: Spiking Neural Network inference
- Input: Sparse spike trains (temporal events)
- Processing: Event-driven neuron updates
- Output: Output spike times

Hypothesis: NPU excels at temporal event processing
Expected: NPU >> GPU/CPU (native workload)
```

#### Test 2B: Continuous Streaming Data
```
Workload: Continuous signal processing
- Input: Dense audio/video streams
- Processing: FFT, convolution, filtering
- Output: Transformed streams

Hypothesis: GPU excels at streaming SIMD
Expected: GPU >> NPU (not event-driven)
```

---

### Category 3: MEMORY-BOUND vs COMPUTE-BOUND

#### Test 3A: Memory Bandwidth Test
```
Workload: Large array copy/scan
- Size: 100MB to 1GB arrays
- Operations: memcpy, reduce, scan
- Pattern: Sequential access

Hypothesis: GPU wins with high memory bandwidth
Expected: GPU > CPU > NPU (bandwidth limited)
```

#### Test 3B: Compute-Intensive Operations
```
Workload: Cryptographic operations
- AES encryption/decryption
- SHA256 hashing
- Integer arithmetic
- Modular exponentiation

Hypothesis: Depends on operation parallelism
Expected: GPU for parallel, NPU for sparse patterns
```

---

### Category 4: BRANCHING vs REGULAR

#### Test 4A: Branch-Heavy Algorithms
```
Workload: Tree traversal, graph search
- Binary tree search
- BFS/DFS on graphs
- Conditional logic chains

Hypothesis: CPU better at branching
Expected: CPU > GPU/NPU (dynamic branching)
```

#### Test 4B: Regular SIMD Operations
```
Workload: Element-wise operations
- Vector addition/multiplication
- Map/reduce operations
- Uniform computation

Hypothesis: GPU excels at regular patterns
Expected: GPU >> CPU/NPU (SIMD paradise)
```

---

### Category 5: WORKLOAD-SPECIFIC TESTS

#### Test 5A: Image Processing
```
Convolutions:
- 2D convolution (3×3, 5×5, 7×7 kernels)
- Separable convolutions
- Depthwise convolutions

Hypothesis: GPU dominates 2D convolutions
Expected: GPU >> NPU/CPU
```

#### Test 5B: Graph Analytics
```
Sparse Graph Operations:
- PageRank on sparse graphs
- Connected components
- Shortest path algorithms

Hypothesis: NPU handles sparse adjacency well
Expected: NPU competitive due to sparsity
```

#### Test 5C: Time-Series Analysis
```
Temporal Patterns:
- RNN/LSTM inference
- Autoregressive models
- Sequential dependencies

Hypothesis: Depends on sparsity of activations
Expected: Mixed results based on activation sparsity
```

#### Test 5D: Bioinformatics
```
Sequence Operations:
- DNA/protein sequence alignment
- K-mer counting
- Motif finding

Hypothesis: NPU handles sparse k-mer patterns
Expected: NPU advantage for sparse k-mers
```

═══════════════════════════════════════════════════════════════════════════════

## 📊 METRICS TO COLLECT

### Performance Metrics
- **Throughput** (ops/sec, samples/sec, GB/sec)
- **Latency** (ms per operation)
- **Scalability** (performance vs input size)
- **Utilization** (% of theoretical peak)

### Energy Metrics
- **Power** (W measured)
- **Energy** (J per operation)
- **Efficiency** (ops/J, samples/J)
- **Energy-Delay Product** (J·s)

### Architectural Insights
- **Memory bandwidth utilization**
- **Compute intensity** (ops/byte)
- **Parallelism achieved**
- **Bottleneck analysis** (compute, memory, I/O)

═══════════════════════════════════════════════════════════════════════════════

## 🔍 AKIDA ARCHITECTURE ANALYSIS

### Known Akida Characteristics (from docs)

**Spiking Neural Network (SNN) Architecture**:
```
- Event-driven processing (only compute on spikes)
- 80 NPUs per AKD1000 chip
- 10MB on-chip memory per chip
- PCIe Gen2 x1 interface (0.5 GB/s)
- ~2W power consumption
```

**Strengths (Hypothesized)**:
1. ✅ **Sparse Event Processing**: Only process non-zero activations
2. ✅ **Ultra-Low Power**: 2W vs 250W GPU
3. ✅ **Temporal Patterns**: Native support for spike timing
4. ✅ **Edge Deployment**: Low power enables battery operation

**Weaknesses (Hypothesized)**:
1. ❌ **Dense Operations**: Must process all elements (no sparsity gain)
2. ❌ **Memory Bandwidth**: 0.5 GB/s vs GPU's ~1000 GB/s
3. ❌ **Limited Memory**: 10MB vs GPU's 24GB
4. ❌ **PCIe Overhead**: Data transfer bottleneck

═══════════════════════════════════════════════════════════════════════════════

## 🎯 HYPOTHESIS TO TEST

### Hypothesis 1: Sparsity-Driven Advantage
**Claim**: NPU dominates ONLY when data is sparse (>70% zeros)

**Test**: Vary sparsity from 0% to 99.9% across workload types
**Expected**: NPU advantage correlates with sparsity level
**Current Data**: ❌ **CONTRADICTED** - NPU maintains efficiency even at 15% sparsity!

**New Hypothesis**: NPU advantage is NOT solely sparsity-dependent

---

### Hypothesis 2: Event-Driven Architecture Advantage
**Claim**: NPU excels at event/spike-based computation regardless of density

**Test**: Compare event-driven (SNN) vs continuous (DNN) workloads
**Expected**: NPU dominates event-driven, struggles with continuous
**Status**: 🔄 **NEEDS TESTING**

---

### Hypothesis 3: Memory-Bandwidth Bottleneck
**Claim**: NPU struggles when data exceeds 10MB or requires high bandwidth

**Test**: Vary data size from 1KB to 1GB
**Expected**: NPU competitive <10MB, degrades beyond
**Status**: 🔄 **NEEDS TESTING**

---

### Hypothesis 4: Compute Pattern Specialization
**Claim**: NPU specialized for specific compute patterns (convolution, accumulation)

**Test**: Compare different operation types (add, mul, conv, matmul)
**Expected**: NPU excels at specific ops, not general-purpose
**Status**: 🔄 **NEEDS TESTING**

═══════════════════════════════════════════════════════════════════════════════

## 📋 EXPERIMENTAL PROTOCOL

### Phase 1: Workload Diversity (Current - Homomorphic Only)
✅ **Complete**: Homomorphic polynomial addition
- Tested sparsity: 99.9%, 95%, 15%
- Finding: NPU dominates across all sparsity levels (UNEXPECTED!)

### Phase 2: Computational Pattern Analysis
🔄 **Next**: Test different computational patterns
- [ ] Dense matrix operations (GEMM)
- [ ] Sparse matrix operations (SpMV)
- [ ] Convolutions (2D, separable)
- [ ] Element-wise operations
- [ ] Reductions (sum, max, argmax)

### Phase 3: Memory Characterization
🔄 **Next**: Test memory hierarchy
- [ ] Vary data sizes (1KB → 1GB)
- [ ] Measure bandwidth utilization
- [ ] Test cache effects
- [ ] Measure transfer overhead

### Phase 4: Real-World Workloads
🔄 **Next**: Test actual application domains
- [ ] Image classification (ResNet, MobileNet)
- [ ] Object detection (YOLO, SSD)
- [ ] Natural language (BERT, GPT)
- [ ] Graph analytics (PageRank)
- [ ] Bioinformatics (k-mer counting)

═══════════════════════════════════════════════════════════════════════════════

## 🛠️ IMPLEMENTATION PLAN

### Benchmark Suite Structure
```
showcase/akida-characterization/
├── benchmarks/
│   ├── sparse_operations.rs      (SpMV, sparse patterns)
│   ├── dense_operations.rs       (GEMM, dense linear algebra)
│   ├── event_driven.rs           (SNN, spike processing)
│   ├── streaming.rs              (continuous data processing)
│   ├── memory_bound.rs           (bandwidth tests)
│   ├── compute_bound.rs          (arithmetic intensity)
│   ├── branching.rs              (control flow heavy)
│   └── real_world.rs             (image, NLP, graph)
├── analysis/
│   ├── roofline_analysis.rs      (compute vs bandwidth limits)
│   ├── scaling_analysis.rs       (performance vs size)
│   └── energy_analysis.rs        (power profiling)
└── reports/
    └── (generated analysis documents)
```

### Execution Timeline
```
Week 1: Dense/Sparse operations (Phase 2)
Week 2: Memory characterization (Phase 3)
Week 3: Real-world workloads (Phase 4)
Week 4: Analysis and white paper
```

═══════════════════════════════════════════════════════════════════════════════

## 💡 EXPECTED INSIGHTS

### What We'll Discover

1. **NPU Sweet Spot**:
   - Optimal sparsity range (if any)
   - Optimal data sizes (memory constraints)
   - Optimal operation types (native SNN ops)

2. **NPU Limitations**:
   - Where GPU dominates (dense ops, high bandwidth)
   - Where CPU dominates (branching, sequential)
   - Unsuitable workloads (large memory, dense compute)

3. **Architectural Understanding**:
   - Akida's execution model
   - Memory hierarchy behavior
   - PCIe transfer impact
   - Power efficiency sources

4. **Practical Recommendations**:
   - When to use NPU (edge, sparse, event-driven)
   - When to use GPU (dense, high-throughput, large models)
   - When to use CPU (control flow, sequential, small data)
   - Hybrid strategies (when to combine)

═══════════════════════════════════════════════════════════════════════════════

## 🎯 SUCCESS CRITERIA

### Scientific Rigor
- ✅ Test diverse workload types (not just HE)
- ✅ Vary all relevant parameters (sparsity, size, pattern)
- ✅ Measure on actual hardware (no simulations)
- ✅ Include negative results (where NPU fails)
- ✅ Explain mechanisms (WHY, not just WHAT)

### Practical Value
- ✅ Provide clear guidance on NPU use cases
- ✅ Identify unsuitable workloads
- ✅ Quantify trade-offs (speed vs power vs memory)
- ✅ Enable informed hardware selection

### Novel Contribution
- ✅ First comprehensive Akida characterization
- ✅ Compare against modern GPU (RTX 3090)
- ✅ Pure Rust ecosystem (not vendor SDK)
- ✅ Open empirical data for community

═══════════════════════════════════════════════════════════════════════════════

**Status**: 🔄 Ready to Execute Phase 2  
**Next**: Implement dense/sparse operation benchmarks  
**Goal**: Understand NPU specialization beyond homomorphic encryption

This is novel hardware, so all characterization data is NEW and PUBLISHABLE!

═══════════════════════════════════════════════════════════════════════════════
