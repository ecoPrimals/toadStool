# 🦈 BARRACUDA NPU OPERATIONS ROADMAP
## February 1, 2026 - Phase 5: Extended NPU Coverage

**Current Status**: ✅ Core NPU backend complete  
**Next Phase**: Identify & implement NPU-accelerated operations

═══════════════════════════════════════════════════════════════════════════════

## 🎯 NPU-FRIENDLY OPERATIONS

### Already NPU-Native (via existing modules)
✅ **Spike encoding/decoding** - spike_encode, spike_decode  
✅ **LIF neurons** - lif_neuron  
✅ **Temporal pooling** - temporal_pool  
✅ **Sparse matmul** - sparse_matmul_quantized

### Priority 1: ML Inference Primitives (Energy Champion Use Case)

**Based on MNIST NPU breakthrough** (7× energy efficient!):

1. **MatMul Variants** 🔥
   - `matmul` - Core dense matmul
   - `batch_matmul` - Batched operations
   - `matmul_tiled` - Tiled for large matrices
   - **NPU Benefit**: Event-driven sparse computation
   - **Implementation**: Convert weights/activations → events → NPU → reconstruct

2. **Activation Functions** 🔥
   - `relu` - Creates sparsity (perfect for NPU!)
   - `gelu` - Modern ML activation
   - `sigmoid` - Neural saturation
   - `softmax` - Output normalization
   - **NPU Benefit**: Threshold operations align with event generation
   - **Implementation**: Apply in event space

3. **Normalization Layers** 🔥
   - `layer_norm` - Transformer normalization
   - `rmsnorm` - Efficient normalization
   - `batch_norm` - CNN normalization
   - **NPU Benefit**: Sparse after normalization
   - **Implementation**: Normalize → threshold → events

4. **Attention Mechanisms** 🔥
   - `multi_head_attention` - Transformer core
   - `sparse_attention` - Already sparse-aware!
   - `local_attention` - Localized patterns
   - **NPU Benefit**: Attention is naturally sparse
   - **Implementation**: Q/K/V → events → attention → reconstruct

### Priority 2: Sparse Operations (NPU Native)

5. **Sparse Primitives** 🔥
   - `sparse_matmul_quantized` - ✅ Already exists!
   - `sparse_attention` - ✅ Already exists!
   - Could add: `sparse_conv2d`, `sparse_linear`
   - **NPU Benefit**: Native sparse representation

6. **Dropout & Masking** 🔥
   - `dropout` - Creates sparsity
   - `masked_fill` - Selective zeros
   - `masked_select` - Sparse selection
   - **NPU Benefit**: Masking = event suppression

### Priority 3: Genomics Operations (from K-mer validation)

7. **Pattern Matching** 🔥
   - `pattern_match` - ✅ Already exists!
   - `gc_content` - ✅ Already exists!
   - `complexity_filter` - ✅ Already exists!
   - **NPU Benefit**: Sparse k-mer matching
   - **Status**: Need to validate with K-mer NPU results

### Priority 4: Specialized Operations

8. **Reservoir Computing** 🔥
   - `reservoir_init` - ✅ Already exists!
   - `reservoir_update` - ✅ Already exists!
   - `spectral_radius` - ✅ Already exists!
   - **NPU Benefit**: Echo State Networks on NPU
   - **Status**: Ready to test

═══════════════════════════════════════════════════════════════════════════════

## 🏗️ IMPLEMENTATION STRATEGY

### Approach: Transparent NPU Acceleration

**Philosophy**: Operations auto-select NPU when beneficial

```rust
// User code (unchanged)
let z = x.matmul(&y)?;

// Internal (v2.0 magic!)
// 1. WorkloadClassifier: "This is ML matmul"
// 2. SparsityAnalyzer: "Input is 60% sparse"
// 3. DeviceSelector: "NPU optimal (energy priority)"
// 4. Execute on NPU backend
```

### Option 1: Wrapper Approach (Fast, Non-Invasive)

**Pros**: No changes to existing ops  
**Cons**: Duplication of logic

```rust
// crates/barracuda/src/npu/ops/matmul.rs
pub async fn npu_matmul(
    a: &[f32], 
    b: &[f32], 
    shape_a: &[usize],
    shape_b: &[usize],
    npu: &mut NpuMlBackend
) -> Result<Vec<f32>> {
    // 1. Analyze sparsity
    let sparsity = EventCodec::default().measure_sparsity(a);
    
    // 2. Convert to events
    let events_a = EventCodec::default().encode_simple(a);
    let events_b = EventCodec::default().encode_simple(b);
    
    // 3. NPU execution (configure for matmul structure)
    let result = npu.execute_mlp_layer(&events_a, shape_b[1])?;
    
    // 4. Reconstruct
    Ok(result)
}
```

### Option 2: Trait-Based Backend Selection (Clean, Extensible)

**Pros**: Clean architecture, extensible  
**Cons**: Requires refactoring existing ops

```rust
// crates/barracuda/src/ops/backend.rs
pub trait ComputeBackend {
    fn matmul(&mut self, a: &[f32], b: &[f32], ...) -> Result<Vec<f32>>;
    fn relu(&mut self, x: &[f32]) -> Result<Vec<f32>>;
    // ... other ops
}

// CPU backend (existing)
impl ComputeBackend for WgpuDevice { /* ... */ }

// NPU backend (new!)
impl ComputeBackend for NpuMlBackend {
    fn matmul(&mut self, a: &[f32], b: &[f32], ...) -> Result<Vec<f32>> {
        // Event-driven matmul
    }
}

// Auto-selection
pub struct UnifiedBackend {
    selector: DeviceSelector,
    cpu: WgpuDevice,
    npu: Option<NpuMlBackend>,
}

impl UnifiedBackend {
    pub fn matmul(&mut self, a: &[f32], b: &[f32], ...) -> Result<Vec<f32>> {
        let device = self.selector.select(WorkloadType::ML, ...);
        match device {
            ComputeDevice::NPU => self.npu.as_mut()?.matmul(a, b, ...),
            _ => self.cpu.matmul(a, b, ...),
        }
    }
}
```

### Option 3: Hybrid Approach (Pragmatic)

**Recommendation**: Start with wrapper approach for ML ops, refactor if needed

**Phase 5a**: Implement NPU wrappers for top 5 ML ops
- `npu_matmul`
- `npu_relu`
- `npu_layer_norm`
- `npu_attention`
- `npu_softmax`

**Phase 5b**: Integrate into Tensor API
- Add `Tensor::with_device(device)` method
- Auto-selection based on workload

═══════════════════════════════════════════════════════════════════════════════

## 📊 PRIORITY MATRIX

### Immediate (Next Session)

| Operation | NPU Benefit | Effort | Priority |
|-----------|-------------|--------|----------|
| **matmul** | 🔥🔥🔥 High | Medium | **P0** |
| **relu** | 🔥🔥🔥 High | Low | **P0** |
| **layer_norm** | 🔥🔥 Medium | Medium | **P1** |
| **softmax** | 🔥🔥 Medium | Medium | **P1** |
| **dropout** | 🔥 Low | Low | **P2** |

### Short-Term (This Week)

| Operation | NPU Benefit | Effort | Priority |
|-----------|-------------|--------|----------|
| **multi_head_attention** | 🔥🔥🔥 High | High | **P1** |
| **batch_matmul** | 🔥🔥 Medium | Medium | **P1** |
| **rmsnorm** | 🔥🔥 Medium | Low | **P2** |
| **gelu** | 🔥 Low | Low | **P2** |

### Future

- Sparse conv2d for CNNs
- Specialized genomics ops (after K-mer NPU validation)
- Graph neural network ops
- Transformer building blocks

═══════════════════════════════════════════════════════════════════════════════

## 🎯 RECOMMENDED NEXT STEPS

### Option A: Implement Core ML Ops on NPU (2-3 hours)

**Deliverables**:
1. `crates/barracuda/src/npu/ops/` module
2. Implement: matmul, relu, layer_norm, softmax
3. Benchmarks for each operation
4. Integration tests

**Impact**: Enable full MLP inference on NPU

---

### Option B: Validate Existing NPU Ops (1 hour)

**Deliverables**:
1. Test existing neuromorphic ops on NPU
2. Benchmark: spike_encode, lif_neuron, temporal_pool
3. Validate genomics ops (pattern_match, gc_content)

**Impact**: Characterize what's already NPU-ready

---

### Option C: Build Unified Backend API (3-4 hours)

**Deliverables**:
1. `ComputeBackend` trait
2. Implement for WgpuDevice, NpuMlBackend
3. Auto-selection in Tensor API
4. Full integration tests

**Impact**: Complete v2.0 "Tensors Everywhere" vision

═══════════════════════════════════════════════════════════════════════════════

## 🔬 VALIDATION STRATEGY

For each NPU operation implementation:

1. **Unit Test**: Correctness vs CPU/GPU
2. **Benchmark**: Energy, latency, throughput
3. **Decision Matrix Update**: Add to selector
4. **Integration Test**: End-to-end pipeline

**Goal**: Expand validated test count from 88 → 150+

═══════════════════════════════════════════════════════════════════════════════

## 📝 SUMMARY

**v2.0 Core**: ✅ Complete (~1,000 lines)  
**v2.0 Operations**: ⏳ Roadmap defined (5 phases)  
**Recommendation**: Start with Option A (core ML ops)

**Estimated Total Work**:
- Phase 5a (Core ML ops): 2-3 hours
- Phase 5b (Integration): 1-2 hours
- Phase 5c (Validation): 2-3 hours
- **Total v2.0 Complete**: 5-8 hours

**Current Progress**: ~60% of full v2.0 vision  
**Core Backend**: 100% complete ✅  
**Operation Coverage**: 5% (5 neuromorphic ops NPU-ready)  
**Target Coverage**: 30% (top 30 ML ops NPU-ready)

═══════════════════════════════════════════════════════════════════════════════

**Status**: Roadmap complete, ready for Phase 5 implementation  
**Recommendation**: Implement matmul + relu as proof-of-concept  
**Next**: Choose Option A, B, or C and proceed

═══════════════════════════════════════════════════════════════════════════════
