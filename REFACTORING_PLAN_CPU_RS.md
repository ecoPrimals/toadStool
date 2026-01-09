# CPU.rs Refactoring Plan

**Date**: January 9, 2026  
**Current Size**: 1404 lines  
**Target**: Modular architecture with <400 lines per file  
**Status**: Plan documented, implementation deferred to next session

## Current Structure

The `cpu.rs` file contains:
1. Core struct and discovery (lines 1-130)
2. 23 operation implementations:
   - Basic ops: map, filter, reduce, scan
   - Vector ops: dot_product, elementwise_binary, gather, scatter
   - Transform ops: transpose
   - Activation ops: softmax, relu, gelu, tanh, sigmoid, dropout
   - Normalization ops: layernorm, batchnorm
   - Tensor ops: matmul, conv, maxpool2d, avgpool2d

## Proposed Modular Structure

```
crates/runtime/universal/src/backends/cpu/
├── mod.rs                 (~200 lines) - Core struct, discovery, dispatch
├── basic_ops.rs           (~150 lines) - map, filter, reduce, scan
├── vector_ops.rs          (~250 lines) - dot_product, elementwise, gather, scatter
├── transform_ops.rs       (~100 lines) - transpose, reshape
├── activation_ops.rs      (~300 lines) - relu, gelu, tanh, sigmoid, dropout, softmax
├── normalization_ops.rs   (~200 lines) - layernorm, batchnorm
└── tensor_ops.rs          (~400 lines) - matmul, conv, maxpool2d, avgpool2d
```

## Implementation Strategy

### Phase 1: Create Module Structure (✅ Started)
- [x] Create `cpu/mod.rs` with core struct and dispatch
- [ ] Create empty operation modules

### Phase 2: Extract Operations (Pending)
For each operation module:
1. Extract operation functions from cpu.rs
2. Make them pub(crate) functions taking `&CpuComputeUnit`
3. Update mod.rs dispatch to call module functions
4. Add module-level tests

### Phase 3: Verify and Test (Pending)
1. Run all tests to ensure no regressions
2. Check that all operations still work
3. Verify performance hasn't degraded

### Phase 4: Cleanup (Pending)
1. Remove old cpu.rs file
2. Update module exports
3. Update documentation

## Benefits of This Refactoring

1. **Maintainability**: Each operation category in its own file
2. **Testability**: Easier to test individual operation categories
3. **Readability**: <400 lines per file, easier to navigate
4. **Extensibility**: Easy to add new operations to appropriate module
5. **Compilation**: Parallel compilation of modules

## Risks and Mitigation

**Risk**: Breaking existing functionality  
**Mitigation**: Comprehensive test suite, incremental refactoring

**Risk**: Performance regression  
**Mitigation**: Benchmark before/after, ensure inlining works

**Risk**: Increased compilation time  
**Mitigation**: Use pub(crate) to limit visibility, enable parallel compilation

## Timeline Estimate

- Phase 1: 30 minutes (started)
- Phase 2: 2 hours (extract all operations)
- Phase 3: 30 minutes (testing)
- Phase 4: 15 minutes (cleanup)

**Total**: ~3 hours of focused work

## Deferred Reason

This refactoring, while valuable, is **not critical** for production readiness. The current single-file implementation:
- ✅ Works correctly (all tests pass)
- ✅ Has good performance (uses rayon for parallelism)
- ✅ Is well-documented
- ⚠️ Violates 1000-line guideline (1404 lines)

**Priority**: Medium  
**Impact**: Code organization and maintainability  
**Urgency**: Low (no functional issues)

## Recommendation

**Defer to next session** and focus on:
1. ✅ Test coverage expansion (currently 44.55%, target 60%)
2. ✅ Addressing production TODOs (mDNS, distributed execution)
3. ✅ Hardcoding elimination (ports, constants)

This refactoring can be done in a dedicated code quality session after critical functionality is complete.

