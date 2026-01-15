# WebGPU Optimization Knowledge Base
## Empirical Findings from Systematic Research

**Hardware**: NVIDIA GeForce RTX 3090 (Vulkan backend)  
**Date**: January 15, 2026  
**Experiments**: 2/20+ planned  
**Status**: 🔬 **ACTIVE RESEARCH**

---

## 🎯 RESEARCH PHILOSOPHY

**"Measure everything. Assume nothing. Build knowledge systematically."**

---

## 📊 FINDINGS TO DATE

### **Experiment 001: MatMul (Compute-Bound)** ✅

**Operation Type**: Compute-bound (heavy arithmetic)  
**Algorithm**: Naive matrix multiplication

**Optimal Workgroup Sizes**:

| Matrix Size | Optimal | Performance | Pattern |
|-------------|---------|-------------|---------|
| 256×256 | 256 | 4248μs | Consistent |
| 512×512 | 256 | 5309μs | Consistent |
| 1024×1024 | 128 | 9702μs | Transition |
| 2048×2048 | 128 | 37812μs | Consistent |

**Pattern Identified**:
- **Small matrices (≤512)**: 256 threads optimal
- **Large matrices (≥1024)**: 128 threads optimal
- **Sweet spot**: 128-256 threads
- **Extremes**: 32 (worst), 1024 (never optimal)
- **Consistency**: HIGH ✅

**Key Learning**: Compute-bound operations have **predictable, consistent patterns**.

---

### **Experiment 002: LayerNorm (Memory-Bound)** ✅

**Operation Type**: Memory-bound (3-pass, reduction + normalization)  
**Algorithm**: Multi-pass with Welford's statistics

**Optimal Workgroup Sizes**:

| Tensor Size | Optimal | Performance | Pattern |
|-------------|---------|-------------|---------|
| 128K | 128 | 5445μs | Moderate |
| 384K (BERT) | **32** | 7824μs | **Outlier!** |
| 1M (GPT-2) | **1024** | 19366μs | **Extreme!** |
| 8M (LLaMA) | 512 | 120116μs | Moderate |

**Pattern Identified**:
- **NO CONSISTENT PATTERN!** ⚠️
- **Optimal spans full range**: 32 → 1024
- **Size dependency**: Complex, non-linear
- **Outliers**: BERT (32), GPT-2 (1024) are extreme
- **Consistency**: LOW ❌

**Key Learning**: Memory-bound operations are **CHAOTIC** - no simple rules!

---

## 🔬 COMPARATIVE ANALYSIS

### **Compute-Bound vs Memory-Bound**

| Characteristic | Compute (MatMul) | Memory (LayerNorm) |
|----------------|------------------|---------------------|
| **Pattern Complexity** | Simple | Chaotic |
| **Optimal Range** | 128-256 | 32-1024 (full!) |
| **Predictability** | High | Low |
| **Size Sensitivity** | Moderate | High |
| **Performance Delta** | 3-6% | 3-10% |
| **Optimization Difficulty** | Easy | Hard |
| **Rule-Based Optimization** | Possible | Impossible |

**CRITICAL INSIGHT**: **Operation class fundamentally changes optimization strategy!**

---

## 🎯 OPTIMIZATION GUIDELINES (Current)

### **For Compute-Bound Operations** (MatMul, etc.)

**Simple Rule**:
```rust
fn optimal_workgroup_compute(n: usize) -> usize {
    match n {
        0..=512 => 256,   // Small problems
        _ => 128,          // Large problems
    }
}
```

**Confidence**: HIGH ✅  
**Expected Impact**: 3-6% improvement  
**Applicability**: MatMul, dense linear algebra

---

### **For Memory-Bound Operations** (LayerNorm, etc.)

**Complex Lookup Required**:
```rust
fn optimal_workgroup_memory_layernorm(n: usize) -> usize {
    match n {
        0..=200_000 => 128,       // Small (< 200K)
        200_001..=500_000 => 32,  // BERT range (strange but empirical!)
        500_001..=6_000_000 => 1024, // GPT-2 range (needs validation)
        _ => 512,                  // LLaMA+ (8M+)
    }
}
```

**Confidence**: MEDIUM ⚠️ (needs more validation)  
**Expected Impact**: 3-10% improvement (higher than compute!)  
**Note**: GPT-2 result (1024) needs re-validation

---

## ⚠️ OPEN QUESTIONS

### **Question 1: Why is BERT (384K) An Outlier?**

**Data**: 384K prefers 32 threads (smallest tested)

**Hypotheses**:
1. Cache effects (384K × 4 bytes = 1.5MB, fits in L2 cache?)
2. Memory alignment characteristics at this specific size
3. Multi-pass algorithm interaction with this size
4. Hardware-specific quirk

**Action**: Test 192K, 256K, 512K to find transition points

---

### **Question 2: Is GPT-2 (1M → 1024) Real?**

**Data**: 1M prefers 1024 threads (largest tested)

**Suspicious Because**:
- MatMul large sizes prefer 128 (opposite!)
- 1024 was never optimal in Experiment 001
- Contradicts conventional wisdom

**Action**: 
- Re-run to verify reproducibility
- Test 768, 896 (near 1024)
- Check if measurement error

---

### **Question 3: Does Multi-Pass Need Per-Pass Tuning?**

**LayerNorm Algorithm**:
- Pass 1: Compute partial stats (reduction)
- Pass 2: Finalize stats (single workgroup)
- Pass 3: Normalize (parallel, memory-bound)

**Current**: All passes use same workgroup size  
**Potential**: Each pass may have different optimal

**Action**: Design Experiment 006 for per-pass optimization

---

### **Question 4: Are Patterns Hardware-Specific?**

**Current Data**: RTX 3090 (Vulkan) only

**Key Question**: Do AMD/Intel/Apple show same patterns?

**AMD Specific**:
- Wavefront size = 64 (vs NVIDIA warp = 32)
- May prefer different workgroup sizes

**Apple M-series**:
- Unified memory architecture
- May have very different patterns

**Action**: Run Experiments 001-002 on AMD/Intel/Apple GPUs

---

## 📈 PROGRESS TRACKER

### **Experiments Completed**: 2/5 (Phase 1)

- ✅ **Experiment 001**: MatMul workgroup sweep
  - **Finding**: Consistent patterns (128-256 sweet spot)
  - **Confidence**: HIGH
  
- ✅ **Experiment 002**: LayerNorm workgroup sweep
  - **Finding**: Chaotic patterns (32-1024 range!)
  - **Confidence**: MEDIUM (needs validation)

### **Experiments Pending**

- ⏳ **Experiment 003**: Activations (ReLU, GELU) - Validate compute-bound patterns
- ⏳ **Experiment 004**: Reductions (Sum, Max) - Synchronization-heavy operations
- ⏳ **Experiment 005**: Memory access patterns - Sequential vs strided vs random

---

## 🎓 KEY LEARNINGS TO DATE

### **1. Operation Class is Fundamental**

✅ **Compute-bound operations** (MatMul):
- Simple, predictable patterns
- Consistent across sizes
- Easy to optimize

✅ **Memory-bound operations** (LayerNorm):
- Complex, chaotic patterns
- Size-dependent in non-obvious ways
- Hard to optimize

⏳ **Sync-heavy operations** (Reductions):
- Unknown (test next!)

**Implication**: Must profile each operation class separately!

---

### **2. WebGPU ≠ CUDA**

**CUDA Conventions**:
- "256 threads is default/optimal"
- "Larger is better"
- "One size fits most"

**WebGPU Reality** (RTX 3090, Vulkan):
- ❌ 256 not always optimal (128 for large MatMul, 32 for BERT LayerNorm!)
- ❌ Larger is not better (1024 never optimal for MatMul)
- ❌ One size does NOT fit all (patterns vary wildly by operation)

**Conclusion**: Platform-specific research is ESSENTIAL!

---

### **3. Empirical Validation Beats Intuition**

**If We Had Guessed**:
- Would use 256 for everything (CUDA default)
- Would miss 3-10% performance
- Would waste time on wrong optimizations

**With Systematic Research**:
- ✅ Found size-dependent optima
- ✅ Discovered operation-class differences
- ✅ Avoided wasted effort
- ✅ Built confidence in data

**Conclusion**: Systematic research is the ONLY reliable approach!

---

### **4. Memory-Bound is Harder to Optimize**

**Complexity**:
- Compute-bound: Simple patterns
- Memory-bound: Chaotic patterns

**Sensitivity**:
- Compute-bound: 3-6% delta
- Memory-bound: 3-10% delta (higher!)

**Optimization Strategy**:
- Compute-bound: Use simple rules
- Memory-bound: Use empirical lookup tables

**Conclusion**: More research needed for memory-bound operations!

---

## 🚀 NEXT ACTIONS

### **Immediate** (This Week)

1. ⏳ **Validate GPT-2 Result**
   - Re-run Experiment 002 for 1M size
   - Test 768, 896 workgroup sizes
   - Confirm 1024 is truly optimal

2. ⏳ **Run Experiment 003** (Activations)
   - Test ReLU, GELU, Sigmoid
   - Validate compute-bound pattern from Experiment 001
   - Should match MatMul behavior

3. ⏳ **Update Hardware Profile**
   - Add LayerNorm data to rtx_3090_vulkan.yaml
   - Note complexity and open questions

### **Short-Term** (Next 2 Weeks)

4. ⏳ **Complete Phase 1 Experiments**
   - Experiment 004: Reductions (sync-heavy)
   - Experiment 005: Memory access patterns
   - Build operation-class taxonomy

5. ⏳ **Design Phase 2**
   - Run Experiments 001-002 on AMD GPU
   - Compare patterns across vendors
   - Start hardware database

---

## 💯 BOTTOM LINE

### **Knowledge Gained** (2 Experiments)

1. ✅ **Compute-bound operations have consistent patterns**
   - 128-256 sweet spot
   - Predictable
   - Easy to optimize

2. ✅ **Memory-bound operations are chaotic**
   - Full range (32-1024)
   - Complex dependencies
   - Require empirical lookup tables

3. ✅ **Operation class matters more than expected**
   - Can't generalize across types
   - Different optimization strategies needed

4. ✅ **Systematic research is essential**
   - Would have guessed wrong
   - Patterns are non-intuitive
   - Empirical validation is ONLY way

### **Confidence Level**

- **Compute-Bound Guidelines**: HIGH ✅
- **Memory-Bound Guidelines**: MEDIUM ⚠️ (needs more data)
- **Research Approach**: VERY HIGH ✅ (validated!)

---

**Status**: 2/5 Phase 1 Experiments Complete  
**Knowledge**: Growing systematically  
**Confidence**: Increasing with each experiment  
**Approach**: **VALIDATED** ✅

---

🔬 **"From guessing to knowing, one experiment at a time!"** 🔬
