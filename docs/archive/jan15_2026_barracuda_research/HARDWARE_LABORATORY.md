# Hardware Laboratory Inventory
## Complete Research Testbed Available

**Date**: January 15, 2026  
**Status**: ✅ **FULL MULTI-VENDOR LABORATORY DETECTED**

---

## 🖥️ DETECTED HARDWARE

### **GPU 1: AMD Radeon RX 6950 XT** 🔴
- **Architecture**: RDNA 2
- **Vendor**: AMD/ATI
- **Device ID**: 0x73a5
- **Model**: 0x6950
- **SKU**: 69KB6SHF1
- **Driver**: ROCm 6.12.10
- **Memory**: ~16GB GDDR6 (estimated)
- **Compute Units**: 80 (estimated)
- **Wavefront Size**: 64 (AMD characteristic)

**Research Value**: HIGH - Different architecture from NVIDIA!

---

### **GPU 2: NVIDIA GeForce RTX 3090** 🟢
- **Architecture**: Ampere (GA102)
- **Vendor**: NVIDIA
- **CUDA Cores**: 10496
- **Tensor Cores**: 328
- **RT Cores**: 82
- **Driver**: 570.153.02
- **Memory**: 24GB GDDR6X
- **Memory Bandwidth**: 936.2 GB/s
- **Warp Size**: 32 (NVIDIA characteristic)

**Research Value**: HIGH - Our baseline, high performance!

---

### **CPU: Dual AMD EPYC 7452** ⚡
- **Model**: AMD EPYC 7452 32-Core Processor
- **Total CPUs**: 128 (dual socket, hyperthreading)
- **Cores per Socket**: 32
- **Threads per Core**: 2
- **Total Physical Cores**: 64
- **Total Threads**: 128

**Research Value**: VERY HIGH - Massive parallelism for CPU fallback testing!

---

## 🔬 RESEARCH OPPORTUNITIES

### **Cross-Vendor Validation** (Critical!)

**Question**: Are our NVIDIA findings universal or vendor-specific?

**Test Plan**:
1. Run Experiment 001 (MatMul) on AMD RX 6950 XT
2. Run Experiment 002 (LayerNorm) on AMD RX 6950 XT
3. Compare patterns with NVIDIA RTX 3090 results
4. Document vendor-specific behaviors

**Expected Insights**:
- Are optimal workgroup sizes the same?
- Does AMD's wavefront size (64) affect patterns?
- Are memory-bound operations equally chaotic on AMD?

---

### **Architecture Comparison**

| Feature | AMD RX 6950 XT | NVIDIA RTX 3090 |
|---------|----------------|-----------------|
| **Architecture** | RDNA 2 | Ampere |
| **Wavefront/Warp** | 64 | 32 |
| **Memory** | 16GB GDDR6 | 24GB GDDR6X |
| **Compute Units/SMs** | 80 CUs | 82 SMs |
| **API** | Vulkan, OpenCL | Vulkan, CUDA |

**Key Difference**: **Wavefront size 64 vs 32!**
- AMD processes 64 threads simultaneously
- NVIDIA processes 32 threads simultaneously
- May prefer different workgroup sizes!

---

### **CPU Baseline Testing**

**128 CPU Threads Available!**

**Test Plan**:
1. Run operations on CPU (WebGPU CPU backend)
2. Compare CPU vs GPU performance
3. Identify which operations benefit from GPU
4. Document CPU-GPU crossover points

**Value**: Understanding when GPU is worth it!

---

## 🎯 IMMEDIATE RESEARCH PLAN

### **Phase 1: AMD GPU Profiling** (This Session!)

1. ⏳ **Setup**: Modify experiments to select GPU
2. ⏳ **Run Experiment 001 on AMD**: MatMul workgroup sweep
3. ⏳ **Run Experiment 002 on AMD**: LayerNorm workgroup sweep
4. ⏳ **Compare Results**: AMD vs NVIDIA side-by-side
5. ⏳ **Document Findings**: Vendor-specific behaviors

**Expected Duration**: ~30 minutes (both experiments)

---

### **Phase 2: CPU Profiling** (Next)

1. ⏳ Run Experiments 001-002 on CPU backend
2. ⏳ Measure CPU vs GPU performance
3. ⏳ Identify crossover points
4. ⏳ Document when to use CPU vs GPU

---

### **Phase 3: Parallel Multi-GPU** (Future)

1. ⏳ Test workload distribution across both GPUs
2. ⏳ Measure scaling efficiency
3. ⏳ Document multi-GPU strategies

---

## 📊 EXPECTED FINDINGS

### **Hypothesis 1: Wavefront Size Matters**

**AMD** (64 threads):
- May prefer workgroup sizes that are multiples of 64
- Possible optima: 64, 128, 256, 512, 1024

**NVIDIA** (32 threads):
- May prefer multiples of 32
- Our current optima: 128, 256, 512

**Prediction**: AMD may show different optimal workgroup sizes!

---

### **Hypothesis 2: Memory Patterns Differ**

**AMD RDNA 2**:
- Different cache hierarchy
- Different memory bandwidth characteristics
- May have different memory-bound patterns

**Prediction**: LayerNorm (memory-bound) may behave differently on AMD!

---

### **Hypothesis 3: Compute Patterns May Be Similar**

**MatMul** (compute-bound):
- Less dependent on hardware specifics
- May show similar patterns across vendors

**Prediction**: AMD MatMul patterns may match NVIDIA (but verify!)

---

## 🔬 RESEARCH VALUE

### **Why This Matters**

1. **Generalizability**: Are our findings universal or NVIDIA-specific?
2. **Vendor Strategies**: Do we need vendor-specific optimizations?
3. **Hardware Adaptation**: How to select optimal configs per GPU?
4. **Real-World Impact**: Most users don't have same GPU as us!

### **Scientific Rigor**

✅ **Cross-vendor validation** (AMD + NVIDIA)  
✅ **Multiple architectures** (RDNA 2 + Ampere)  
✅ **CPU baseline** (128 threads!)  
✅ **Reproducibility** (test on multiple hardware)

**This is how real research is done!**

---

## 💯 BOTTOM LINE

**We Have**:
- ✅ High-end AMD GPU (RX 6950 XT)
- ✅ High-end NVIDIA GPU (RTX 3090)
- ✅ Massive CPU resources (128 threads!)
- ✅ Complete WebGPU research testbed

**We Should**:
1. ⏳ Run experiments on BOTH GPUs
2. ⏳ Compare vendor-specific patterns
3. ⏳ Validate findings across architectures
4. ⏳ Build multi-vendor knowledge base

**Impact**:
- Validate our NVIDIA findings
- Discover vendor-specific behaviors
- Build robust, portable optimizations
- Achieve true cross-vendor mastery

---

**Status**: ✅ Laboratory Detected | ⏳ Multi-Vendor Research Ready!

---

🔬 **"From single-GPU to full laboratory! Let's utilize EVERYTHING!"** 🔬
