# Phase 4: Attention Mechanisms - Implementation Plan

**Status**: ⏳ **READY TO START** (Feb 3, 2026)  
**Goal**: Enable transformer workloads on any chip  
**Target**: 6-7 operations → 40% universal coverage  
**Timeline**: 2-3 weeks  
**Priority**: 🔥 **CRITICAL** for modern AI

═══════════════════════════════════════════════════════════════

## 🎯 **CURRENT STATE**

### **Existing Attention Operations** (CPU-only):

All attention operations exist as CPU reference implementations but need:
1. ✅ WGSL shader implementations (GPU)
2. ✅ `impl Tensor` integration (Tensor API)
3. ✅ Cross-chip testing & validation
4. ✅ Performance benchmarks vs PyTorch

**Files Found**:
- ✅ `ops/multi_head_attention.rs` - CPU reference (needs WGSL)
- ✅ `ops/scaled_dot_product_attention.rs` - CPU reference (has shader, needs improvement)
- ✅ `ops/sparse_attention.rs` - CPU reference (needs WGSL)
- ✅ `ops/causal_attention.rs` - CPU reference (needs WGSL)
- ✅ `ops/cross_attention.rs` - CPU reference (needs WGSL)
- ✅ `ops/flash_attention.rs` - CPU reference (needs WGSL)
- ✅ `ops/grouped_query_attention.rs` - CPU reference (needs WGSL)
- ✅ `ops/local_attention.rs` - CPU reference (needs WGSL)

**Shaders Found**:
- ⚠️ `shaders/scaled_dot_product_attention.wgsl` - Exists but incomplete

═══════════════════════════════════════════════════════════════

## 📋 **IMPLEMENTATION STRATEGY**

### **Approach**: **Iterative Evolution with Validation**

For each operation:
1. **Analyze** CPU reference implementation (understand algorithm)
2. **Design** WGSL shader (GPU-optimized version)
3. **Implement** shader + Rust wrapper
4. **Integrate** with Tensor API (`impl Tensor`)
5. **Test** numerical correctness vs CPU reference
6. **Benchmark** performance vs PyTorch
7. **Document** API & examples
8. **Validate** deep debt A++ maintained

**Philosophy**: "Make it work, make it right, make it fast"
- Phase 4.1: Work (correct implementation)
- Phase 4.2: Right (cross-chip validation)
- Phase 4.3: Fast (performance optimization)

═══════════════════════════════════════════════════════════════

## 🚀 **PHASE 4 OPERATIONS** (Priority Order)

### **Week 1: Core Attention (2-3 operations)**

#### **1. scaled_dot_product_attention** (⚡ **IMPROVE EXISTING**)

**Priority**: 🔥 **CRITICAL** (foundation for all attention)  
**Status**: ⚠️ Has shader + CPU impl, needs GPU wrapper improvement  
**Effort**: 3-5 days

**Current State**:
- ✅ CPU reference implementation exists
- ✅ WGSL shader exists (`shaders/scaled_dot_product_attention.wgsl`)
- ❌ Not wired to Tensor API
- ❌ Shader is incomplete (single-kernel, needs multi-pass)

**Tasks**:
1. Review existing WGSL shader
2. Improve shader (multi-pass: scores → softmax → apply)
3. Create Rust wrapper with proper buffer management
4. Add `impl Tensor` for API integration
5. Write comprehensive tests
6. Benchmark vs PyTorch

**Success Criteria**:
- ✅ Numerical correctness (< 1e-5 error vs CPU)
- ✅ Works on GPU/CPU via WebGPU
- ✅ Performance within 2x of PyTorch
- ✅ Tests pass for various sizes (small, medium, large)
- ✅ Wired to Tensor API

**Why First**: Foundation for all other attention mechanisms!

---

#### **2. multi_head_attention** (🔥 **MOST CRITICAL**)

**Priority**: 🔥 **CRITICAL** (transformers depend on this)  
**Status**: ❌ CPU-only, needs complete WGSL implementation  
**Effort**: 7-10 days

**Current State**:
- ✅ CPU reference implementation exists
- ❌ No WGSL shader
- ❌ Not wired to Tensor API

**Algorithm**:
```
MultiHead(Q, K, V) = Concat(head_1, ..., head_h) * W^O
where head_i = Attention(Q*W^Q_i, K*W^K_i, V*W^V_i)
```

**Tasks**:
1. Design WGSL shader architecture (4 passes)
   - Pass 1: Project Q, K, V (matmul with weights)
   - Pass 2: Reshape to [batch, heads, seq, head_dim]
   - Pass 3: scaled_dot_product_attention per head
   - Pass 4: Concat heads + output projection
2. Implement Rust wrapper
3. Add `impl Tensor` for API integration
4. Write comprehensive tests
5. Benchmark vs PyTorch

**Challenges**:
- Multiple matrix multiplications (projections)
- Per-head attention computation
- Concatenation & final projection
- Memory management across passes

**Success Criteria**:
- ✅ Numerical correctness (< 1e-4 error vs CPU)
- ✅ Works on GPU/CPU
- ✅ Performance within 2x of PyTorch MHA
- ✅ Supports various head counts (1, 4, 8, 16)
- ✅ Comprehensive tests (edge cases, large inputs)

**Why Critical**: Core transformer building block!

---

#### **3. causal_attention** (🔥 **HIGH PRIORITY**)

**Priority**: 🔥 **HIGH** (GPT-style models)  
**Status**: ❌ CPU-only, needs WGSL  
**Effort**: 3-4 days

**Current State**:
- ✅ CPU reference implementation
- ❌ No WGSL shader

**Algorithm**: Same as scaled_dot_product but with causal mask
```
scores[i, j] = -inf if j > i else QK^T / sqrt(d_k)
```

**Tasks**:
1. Design WGSL shader (extend scaled_dot_product with masking)
2. Implement Rust wrapper
3. Add `impl Tensor` for API integration
4. Test causal masking correctness
5. Benchmark vs PyTorch

**Success Criteria**:
- ✅ Correct causal masking (future positions masked)
- ✅ Numerical correctness
- ✅ Works on GPU/CPU
- ✅ Performance within 2x of PyTorch

**Why Important**: Required for autoregressive models (GPT, etc.)!

---

### **Week 2: Efficiency & Positional Encodings (2-3 operations)**

#### **4. sparse_attention** (⚡ **MEDIUM PRIORITY**)

**Priority**: ⚡ **MEDIUM** (efficiency for long sequences)  
**Status**: ❌ CPU-only, needs WGSL  
**Effort**: 5-7 days

**Current State**:
- ✅ CPU reference implementation
- ❌ No WGSL shader

**Algorithm**: Attention with sparse patterns (windowed, strided, etc.)
```
Only compute attention for subset of positions
Common patterns: local window, strided, global tokens
```

**Tasks**:
1. Design WGSL shader with sparsity patterns
2. Implement common patterns (local, strided, global)
3. Rust wrapper + Tensor integration
4. Test various sparsity patterns
5. Benchmark memory & compute savings

**Success Criteria**:
- ✅ Multiple sparsity patterns supported
- ✅ Significant memory savings for long sequences
- ✅ Performance improvement over dense attention
- ✅ Numerical correctness maintained

**Why Useful**: Enables efficient long-context transformers!

---

#### **5. rotary_embedding** (⚡ **MEDIUM PRIORITY**)

**Priority**: ⚡ **MEDIUM** (modern positional encoding)  
**Status**: ❌ Not implemented, needs WGSL  
**Effort**: 3-4 days

**Algorithm**: Rotary Position Embedding (RoPE)
```
Apply rotation matrix to Q, K based on position
Enables length extrapolation
```

**Tasks**:
1. Design WGSL shader for RoPE
2. Implement rotation computation
3. Rust wrapper + Tensor integration
4. Test position-aware behavior
5. Validate length extrapolation

**Success Criteria**:
- ✅ Correct position encoding
- ✅ Length extrapolation works
- ✅ Fast computation (elementwise)
- ✅ Integrates with attention ops

**Why Important**: Used in modern LLMs (LLaMA, GPT-NeoX, etc.)!

---

### **Week 3: Advanced Variants (2 operations)**

#### **6. cross_attention** (💡 **LOW PRIORITY**)

**Priority**: 💡 **LOW** (encoder-decoder models)  
**Status**: ❌ CPU-only, needs WGSL  
**Effort**: 2-3 days

**Current State**:
- ✅ CPU reference implementation
- ❌ No WGSL shader

**Algorithm**: Attention where Q comes from decoder, K/V from encoder
```
CrossAttention(Q_dec, K_enc, V_enc)
```

**Tasks**:
1. Adapt scaled_dot_product for cross-attention
2. WGSL shader (similar to standard attention)
3. Rust wrapper + Tensor integration
4. Test encoder-decoder scenarios

**Success Criteria**:
- ✅ Correct cross-attention behavior
- ✅ Works with different Q, K/V shapes
- ✅ Numerical correctness
- ✅ Performance acceptable

**Why Useful**: Required for seq2seq models (T5, BART, etc.)!

---

#### **7. alibi_position** (💡 **LOW PRIORITY**)

**Priority**: 💡 **LOW** (alternative positional bias)  
**Status**: ❌ Not implemented, needs WGSL  
**Effort**: 2-3 days

**Algorithm**: Attention with Linear Biases (ALiBi)
```
scores = QK^T / sqrt(d_k) + position_bias
```

**Tasks**:
1. Design WGSL shader for ALiBi bias computation
2. Implement position-dependent bias
3. Rust wrapper + Tensor integration
4. Test length generalization

**Success Criteria**:
- ✅ Correct bias computation
- ✅ Length generalization validated
- ✅ Fast (minimal overhead)
- ✅ Integrates with attention

**Why Interesting**: Simple alternative to absolute/relative position encodings!

═══════════════════════════════════════════════════════════════

## 📋 **IMPLEMENTATION CHECKLIST**

### **For Each Operation**:

**Design Phase**:
- [ ] Analyze CPU reference implementation
- [ ] Design WGSL shader architecture
- [ ] Plan buffer layout & data flow
- [ ] Identify optimization opportunities
- [ ] Document algorithm & approach

**Implementation Phase**:
- [ ] Write WGSL shader
- [ ] Test shader compilation
- [ ] Create Rust wrapper struct
- [ ] Implement `execute()` method
- [ ] Add `impl Tensor` for API integration
- [ ] Handle edge cases (small inputs, large inputs)

**Testing Phase**:
- [ ] Unit tests (basic functionality)
- [ ] Numerical correctness tests (vs CPU reference)
- [ ] Edge case tests (small, large, boundary conditions)
- [ ] Performance tests (vs PyTorch baseline)
- [ ] Cross-chip tests (GPU, CPU)

**Documentation Phase**:
- [ ] API documentation (rustdoc)
- [ ] Usage examples
- [ ] Algorithm explanation
- [ ] Performance characteristics
- [ ] Known limitations

**Validation Phase**:
- [ ] Deep debt A++ maintained (no unsafe, etc.)
- [ ] All tests passing
- [ ] Performance acceptable (within 2x of PyTorch)
- [ ] Code review for quality
- [ ] Commit & push

═══════════════════════════════════════════════════════════════

## 🎯 **SUCCESS CRITERIA FOR PHASE 4**

### **Coverage**:
- ✅ 6-7 operations with complete WGSL implementations
- ✅ All wired to Tensor API (`impl Tensor`)
- ✅ Universal compute coverage: 37.4% → 40%+

### **Quality**:
- ✅ Deep debt A++ maintained (no unsafe, pure Rust, etc.)
- ✅ Numerical correctness (< 1e-4 error vs reference)
- ✅ Comprehensive tests (unit, integration, edge cases)
- ✅ Cross-chip validation (GPU, CPU work correctly)

### **Performance**:
- ✅ Within 2x of PyTorch baseline (acceptable overhead)
- ✅ GPU speedup demonstrated (vs CPU)
- ✅ Memory efficient (no unnecessary copies)

### **Documentation**:
- ✅ API docs complete (rustdoc)
- ✅ Usage examples for each operation
- ✅ Algorithm explanations
- ✅ Integration guide for transformers

### **Impact**:
- ✅ Can run transformer models on any chip!
- ✅ Foundation for LLMs (GPT, BERT, T5, etc.)
- ✅ Enables attention-based architectures universally
- ✅ Clear path to 50% coverage (halfway milestone)

═══════════════════════════════════════════════════════════════

## 🔧 **TECHNICAL APPROACH**

### **WGSL Shader Design Pattern**:

```wgsl
// Pattern for attention operations:

// 1. Parameters struct
struct AttentionParams {
    batch_size: u32,
    num_heads: u32,
    seq_len: u32,
    head_dim: u32,
}

// 2. Bindings (inputs, outputs, params)
@group(0) @binding(0) var<storage, read> query: array<f32>;
@group(0) @binding(1) var<storage, read> key: array<f32>;
@group(0) @binding(2) var<storage, read> value: array<f32>;
@group(0) @binding(3) var<storage, read_write> output: array<f32>;
@group(0) @binding(4) var<uniform> params: AttentionParams;

// 3. Compute shader with workgroup size
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // Algorithm implementation
    // Use global_id to determine which element to compute
}
```

### **Rust Wrapper Pattern**:

```rust
// Pattern for Rust wrapper:

pub struct OperationName {
    query: Tensor,
    key: Tensor,
    value: Tensor,
    // operation-specific params
}

impl OperationName {
    pub fn new(query: Tensor, key: Tensor, value: Tensor) -> Self {
        Self { query, key, value }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/operation_name.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        // 1. Get device
        let device = self.query.device();
        
        // 2. Create output buffer
        let output_buffer = device.create_buffer_f32(output_size)?;
        
        // 3. Create params
        let params = OperationParams { ... };
        
        // 4. Create bind group layout & bind group
        // ... (standard WebGPU boilerplate)
        
        // 5. Compile shader & create pipeline
        let shader = device.compile_shader(Self::wgsl_shader(), ...);
        let pipeline = device.create_compute_pipeline(...);
        
        // 6. Execute
        let mut encoder = device.create_command_encoder(...);
        {
            let mut pass = encoder.begin_compute_pass(...);
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }
        device.queue.submit(Some(encoder.finish()));
        
        // 7. Return Tensor
        Ok(Tensor::from_buffer(output_buffer, shape, device))
    }
}

// 8. Tensor API integration
impl Tensor {
    pub fn operation_name(self, other: &Self) -> Result<Self> {
        OperationName::new(self, other.clone()).execute()
    }
}

// 9. Tests
#[cfg(test)]
mod tests {
    // Comprehensive test suite
}
```

═══════════════════════════════════════════════════════════════

## 📊 **TIMELINE**

### **Week 1** (Feb 3-10):
- Day 1-2: scaled_dot_product_attention improvement
- Day 3-5: multi_head_attention implementation
- Day 5-7: causal_attention implementation

**Milestone**: 3/7 operations complete (39% coverage)

---

### **Week 2** (Feb 10-17):
- Day 1-4: sparse_attention implementation
- Day 4-7: rotary_embedding implementation

**Milestone**: 5/7 operations complete (40% coverage)

---

### **Week 3** (Feb 17-24):
- Day 1-3: cross_attention implementation
- Day 3-5: alibi_position implementation
- Day 5-7: Final testing, benchmarking, documentation

**Milestone**: 7/7 operations complete (40%+ coverage) ✅

═══════════════════════════════════════════════════════════════

## 🎯 **NEXT IMMEDIATE ACTION**

### **START HERE**: scaled_dot_product_attention

**Why**: Foundation for all other attention mechanisms!

**Steps**:
1. ✅ Review existing WGSL shader
2. ✅ Improve shader (multi-pass approach)
3. ✅ Create proper Rust wrapper
4. ✅ Add `impl Tensor` integration
5. ✅ Write comprehensive tests
6. ✅ Benchmark vs PyTorch

**Command to Begin**:
```bash
# 1. Review current implementation
cat crates/barracuda/src/ops/scaled_dot_product_attention.rs
cat crates/barracuda/src/shaders/scaled_dot_product_attention.wgsl

# 2. Start implementation
# (Begin designing improved WGSL shader)
```

═══════════════════════════════════════════════════════════════

**Plan Status**: ✅ **READY TO EXECUTE**  
**First Operation**: scaled_dot_product_attention  
**Target**: 40% universal coverage (103/259 ops)  
**Timeline**: 2-3 weeks  
**Deep Debt**: ✅ A++ maintained throughout

🦀⚡ **Phase 4: Attention Mechanisms - Ready to Build!** ⚡🦀
