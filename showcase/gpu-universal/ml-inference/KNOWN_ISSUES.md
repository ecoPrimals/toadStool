# Known Issues - barraCUDA

**Date**: January 12, 2026  
**Status**: Honest accounting of work in progress

---

## Principle: Transparency Over False Completion

Per Deep Debt principles, we document issues honestly rather than shipping broken code or hiding problems.

---

## Operations Requiring Debug/Completion

### 1. Scan (Prefix Sum) - Algorithm Issue

**Status**: WGSL shader implemented, but producing incorrect results

**Issue**: 
- Blelloch algorithm implementation in `scan.wgsl` produces final sum at all positions instead of cumulative values
- Test shows: input `[1,2,3,4,5]` produces `[15,15,15,15,15]` instead of `[1,3,6,10,15]`

**Root Cause**: 
- Likely issue in up-sweep/down-sweep phases of Blelloch algorithm
- Shared memory indexing or barrier synchronization may be incorrect

**Action Required**:
- [ ] Debug WGSL shader step-by-step
- [ ] Validate up-sweep phase (tree reduction)
- [ ] Validate down-sweep phase (distribution)
- [ ] Test with smaller arrays (2, 4, 8 elements) to isolate issue
- [ ] Add intermediate result logging/validation

**Priority**: Medium (complex operation, used by Filter)

**Deep Debt Compliance**: ✅
- Not shipping broken implementation
- Documented honestly
- Will return with proper time to debug

---

### 2. Operations Not Yet Implemented

**Status**: WGSL shaders complete, Rust wrappers pending

The following have complete, validated WGSL shaders but need Rust integration:

1. **LayerNorm** - Multi-pass (stats computation + normalization)
2. **BatchNorm** - Single-pass (pre-computed stats)
3. **MaxPool2D** - Single-pass (sliding window max)
4. **AvgPool2D** - Single-pass (sliding window average)
5. **Filter** - Depends on Scan (stream compaction)
6. **Scatter** - Atomic writes

**Priority**: High (straightforward integrations)

**Estimated Time**: 2-3 hours for all 6 operations

---

## Testing Gaps

### 1. Softmax Multi-Pass
**Status**: Implemented but needs comprehensive testing

**Required Tests**:
- [ ] Numerical stability (large values, small values)
- [ ] Edge cases (all same, all zeros)
- [ ] Large arrays (multi-workgroup)
- [ ] Performance benchmarks
- [ ] Concurrent execution

### 2. Precision Support
**Status**: All operations currently fp32 only

**Required**:
- [ ] Add fp16 support
- [ ] Add fp64 support (where hardware supports)
- [ ] Generic `GpuPrecision` trait
- [ ] Precision-specific tests

### 3. Hierarchical Reduction
**Status**: Current reduce/scan limited to 65536 elements

**Required**:
- [ ] Multi-level reduction for unbounded arrays
- [ ] Recursive GPU passes
- [ ] Performance validation

---

## Performance Optimization Opportunities

### 1. Softmax
- Current: Three separate pipeline dispatches
- Optimal: Single multi-pass pipeline with proper synchronization
- Benefit: Reduced overhead

### 2. LayerNorm
- Current: Not yet implemented
- Optimal: Welford's algorithm in shared memory
- Benefit: Single-pass statistics computation

### 3. Memory Bandwidth
- Current: Separate allocations per operation
- Optimal: Buffer pooling, reuse
- Benefit: Reduced allocation overhead

---

## Commit to Quality

### What We Will NOT Do
❌ Ship broken Scan implementation  
❌ Hide issues in documentation  
❌ Use CPU fallbacks as "temporary" solution  
❌ Skip comprehensive testing  

### What We WILL Do
✅ Document issues honestly  
✅ Debug properly before shipping  
✅ Maintain zero technical debt  
✅ Complete remaining operations correctly  

---

## Next Steps

### Immediate
1. Skip Scan (needs proper debugging time)
2. Implement remaining 5 straightforward operations
3. Add comprehensive tests for Softmax
4. Document completion status

### Short-Term
1. Return to Scan with focused debugging
2. Add hierarchical reduction
3. Implement precision support (fp16, fp64)
4. Expand test coverage

### Long-Term
1. Optimize multi-pass operations
2. Add buffer pooling
3. Distributed multi-GPU support

---

**Principle**: Honest accounting > False completion

**Status**: 11/21 operations proven correct (52%)  
**Next**: Complete 5 straightforward operations → 16/21 (76%)

**Updated**: January 12, 2026
