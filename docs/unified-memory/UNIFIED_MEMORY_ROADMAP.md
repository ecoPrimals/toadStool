# 🗺️ Universal Unified Memory - Implementation Roadmap

**Date**: January 2, 2026  
**Status**: 📋 PLANNING → EXECUTION  
**Goal**: Zero-copy, vendor-agnostic unified memory for Intel/AMD/NVIDIA

---

## 🎯 Mission

Implement **pure Rust, vendor-agnostic unified memory** that enables:
- ✅ Zero-copy compute (CPU ↔ GPU no transfers)
- ✅ Works on Intel iGPU, AMD APU, NVIDIA GPU
- ✅ Sovereignty-first (WebGPU primary, vendors fallback)
- ✅ Modern async Rust (tokio, no blocking)
- ✅ Zero technical debt (no unwraps, proper errors)

---

## 📊 Current Status: Planning Complete ✅

| Phase | Status | Progress | ETA |
|-------|--------|----------|-----|
| **Specification** | ✅ DONE | 100% | Jan 2 |
| **Phase 1: Core** | 📋 READY | 0% | Week 1 |
| **Phase 2: Backends** | 📋 PLANNED | 0% | Week 1-2 |
| **Phase 3: Integration** | 📋 PLANNED | 0% | Week 2 |
| **Phase 4: Testing** | 📋 PLANNED | 0% | Week 2-3 |
| **Phase 5: Polish** | 📋 PLANNED | 0% | Week 3 |

**Overall Progress**: 10% (Spec Done) → 100% (3 weeks)

---

## 📋 Phase 1: Core Infrastructure (Days 1-2)

**Goal**: Module structure, traits, core types

### Tasks

- [ ] **1.1**: Create module structure
  - [ ] `crates/runtime/gpu/src/unified_memory/mod.rs`
  - [ ] `crates/runtime/gpu/src/unified_memory/manager.rs`
  - [ ] `crates/runtime/gpu/src/unified_memory/buffer.rs`
  - [ ] `crates/runtime/gpu/src/unified_memory/backend.rs`
  - [ ] `crates/runtime/gpu/src/unified_memory/backends/mod.rs`
  - [ ] `crates/runtime/gpu/src/unified_memory/types.rs`
  - **Estimate**: 1 hour
  - **Owner**: TBD
  - **Status**: 📋 Ready

- [ ] **1.2**: Define core types
  - [ ] `BufferId` - Unique buffer identifier
  - [ ] `MemoryFlags` - Allocation flags
  - [ ] `SyncState` - Synchronization state tracking
  - [ ] `UnifiedMemoryCapabilities` - Backend capabilities
  - [ ] `BackendAllocation` - Backend-specific allocation
  - **Estimate**: 2 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

- [ ] **1.3**: Implement `UnifiedMemoryBackend` trait
  - [ ] Trait definition with async methods
  - [ ] Default implementations where possible
  - [ ] Documentation and examples
  - **Estimate**: 2 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

- [ ] **1.4**: Implement `UniversalUnifiedMemory` manager
  - [ ] Backend selection logic
  - [ ] Allocation tracking (DashMap)
  - [ ] Metrics collection
  - [ ] Configuration management
  - **Estimate**: 4 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

- [ ] **1.5**: Implement `UnifiedBuffer`
  - [ ] CPU/GPU pointer management
  - [ ] Async read/write methods
  - [ ] Synchronization state tracking
  - [ ] Type-safe views (TypedView<T>)
  - [ ] Drop implementation (cleanup)
  - **Estimate**: 4 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

**Phase 1 Total**: ~13 hours (2 days)

---

## 🔧 Phase 2: Backend Implementations (Days 3-6)

**Goal**: Implement Vulkan, OpenCL, WebGPU, CPU backends

### 2.1: Vulkan Backend (Days 3-4) - PRIORITY 1

- [ ] **2.1.1**: Vulkan initialization
  - [ ] Create ash instance
  - [ ] Select physical device
  - [ ] Find unified memory type (HOST_VISIBLE + DEVICE_LOCAL)
  - [ ] Create logical device
  - [ ] Query capabilities
  - **Estimate**: 3 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

- [ ] **2.1.2**: Implement UnifiedMemoryBackend for Vulkan
  - [ ] `allocate_unified()` - VkMemoryAllocate + map
  - [ ] `free_unified()` - unmap + VkFreeMemory
  - [ ] `map_cpu_ptr()` - persistent mapping
  - [ ] `get_device_ptr()` - device address
  - [ ] `sync_cpu_to_device()` - flush if needed
  - [ ] `sync_device_to_cpu()` - invalidate if needed
  - **Estimate**: 4 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

- [ ] **2.1.3**: Vulkan memory management
  - [ ] VulkanAllocator for sub-allocation
  - [ ] Memory type fallback logic
  - [ ] Error handling (all paths)
  - **Estimate**: 3 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

- [ ] **2.1.4**: Vulkan tests
  - [ ] Unit tests for allocation/free
  - [ ] CPU write → GPU read test
  - [ ] GPU write → CPU read test
  - [ ] Concurrent allocation test
  - **Estimate**: 2 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

**Vulkan Total**: ~12 hours (1.5 days)

### 2.2: OpenCL SVM Backend (Day 5) - PRIORITY 2

- [ ] **2.2.1**: OpenCL initialization
  - [ ] Query platform and devices
  - [ ] Check OpenCL 2.0+ support
  - [ ] Create context
  - [ ] Query SVM capabilities
  - **Estimate**: 2 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

- [ ] **2.2.2**: Implement UnifiedMemoryBackend for OpenCL
  - [ ] `allocate_unified()` - clSVMAlloc
  - [ ] `free_unified()` - clSVMFree
  - [ ] `map_cpu_ptr()` - direct pointer (already mapped)
  - [ ] `get_device_ptr()` - same pointer
  - [ ] `sync_cpu_to_device()` - clEnqueueSVMMap if needed
  - [ ] `sync_device_to_cpu()` - clEnqueueSVMUnmap if needed
  - **Estimate**: 3 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

- [ ] **2.2.3**: OpenCL tests
  - [ ] Same test suite as Vulkan
  - **Estimate**: 1 hour
  - **Owner**: TBD
  - **Status**: 📋 Ready

**OpenCL Total**: ~6 hours (1 day)

### 2.3: WebGPU Backend (Day 6) - PRIORITY 0 (Future)

- [ ] **2.3.1**: WebGPU initialization
  - [ ] Create wgpu instance
  - [ ] Request adapter
  - [ ] Request device with MAPPABLE_PRIMARY_BUFFERS
  - **Estimate**: 2 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

- [ ] **2.3.2**: Implement UnifiedMemoryBackend for WebGPU
  - [ ] `allocate_unified()` - create_buffer with MAP flags
  - [ ] `map_cpu_ptr()` - buffer.slice().get_mapped_range()
  - [ ] `sync_to_device()` - unmap
  - [ ] `sync_to_cpu()` - map_async
  - **Estimate**: 3 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

- [ ] **2.3.3**: WebGPU tests
  - [ ] Same test suite
  - **Estimate**: 1 hour
  - **Owner**: TBD
  - **Status**: 📋 Ready

**WebGPU Total**: ~6 hours (1 day)

### 2.4: CPU Fallback Backend (Day 6) - ALWAYS AVAILABLE

- [ ] **2.4.1**: Implement UnifiedMemoryBackend for CPU
  - [ ] Use memmap2 for allocations
  - [ ] No-op sync methods (always coherent)
  - [ ] Simple and reliable
  - **Estimate**: 2 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

- [ ] **2.4.2**: CPU tests
  - [ ] Basic allocation/free
  - [ ] Read/write correctness
  - **Estimate**: 1 hour
  - **Owner**: TBD
  - **Status**: 📋 Ready

**CPU Total**: ~3 hours (0.5 day)

**Phase 2 Total**: ~27 hours (3.5 days)

---

## 🔗 Phase 3: Integration (Days 7-9)

**Goal**: Integrate with existing ToadStool architecture

### 3.1: Memory Capabilities Detection (Day 7)

- [ ] **3.1.1**: Create UnifiedMemoryDetector
  - [ ] Detect Vulkan unified memory
  - [ ] Detect OpenCL SVM
  - [ ] Detect WebGPU support
  - [ ] Generate UnifiedMemoryInfo
  - **Estimate**: 3 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

- [ ] **3.1.2**: Update MemoryCapabilities in universal.rs
  - [ ] Call UnifiedMemoryDetector
  - [ ] Set `unified_memory: bool` accurately
  - [ ] Set `zero_copy: bool` accurately
  - **Estimate**: 2 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

**Detection Total**: ~5 hours

### 3.2: Scheduler Integration (Days 7-8)

- [ ] **3.2.1**: Add unified memory path to scheduler
  - [ ] Detect devices with unified memory
  - [ ] Prefer unified memory devices for eligible workloads
  - [ ] Allocate UnifiedBuffer instead of separate buffers
  - **Estimate**: 4 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

- [ ] **3.2.2**: Update workload execution
  - [ ] Use UnifiedBuffer.device_ptr() for kernels
  - [ ] Zero-copy input/output
  - [ ] Track unified memory usage in metrics
  - **Estimate**: 3 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

**Scheduler Total**: ~7 hours

### 3.3: Kernel Execution Integration (Day 8)

- [ ] **3.3.1**: Update GPU backends to accept unified pointers
  - [ ] Vulkan: use device pointer directly
  - [ ] OpenCL: use SVM pointer directly
  - [ ] CUDA: use managed memory pointer
  - **Estimate**: 3 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

### 3.4: Configuration & Strategy (Day 9)

- [ ] **3.4.1**: Add unified memory config
  - [ ] Backend selection strategy
  - [ ] Pool size configuration
  - [ ] Sync strategy (eager/lazy)
  - **Estimate**: 2 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

- [ ] **3.4.2**: Environment variable overrides
  - [ ] `TOADSTOOL_UNIFIED_MEMORY_BACKEND`
  - [ ] `TOADSTOOL_UNIFIED_MEMORY_POOL_SIZE`
  - **Estimate**: 1 hour
  - **Owner**: TBD
  - **Status**: 📋 Ready

**Phase 3 Total**: ~21 hours (3 days)

---

## 🧪 Phase 4: Testing & Validation (Days 10-12)

**Goal**: Comprehensive testing across all backends

### 4.1: Unit Tests (Day 10)

- [ ] **4.1.1**: Backend tests
  - [ ] Vulkan backend unit tests
  - [ ] OpenCL backend unit tests
  - [ ] WebGPU backend unit tests
  - [ ] CPU backend unit tests
  - **Estimate**: 4 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

- [ ] **4.1.2**: Buffer tests
  - [ ] Read/write correctness
  - [ ] Bounds checking
  - [ ] Type safety (TypedView)
  - [ ] Concurrent access
  - **Estimate**: 3 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

### 4.2: Integration Tests (Day 11)

- [ ] **4.2.1**: End-to-end workload tests
  - [ ] Simple compute kernel with unified memory
  - [ ] Multi-buffer workload
  - [ ] CPU write → GPU compute → CPU read
  - **Estimate**: 4 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

- [ ] **4.2.2**: Cross-backend tests
  - [ ] Test all backends on available hardware
  - [ ] Compare performance across backends
  - [ ] Verify functional equivalence
  - **Estimate**: 3 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

### 4.3: Concurrency Tests (Day 12)

- [ ] **4.3.1**: Stress tests
  - [ ] 1000 concurrent allocations
  - [ ] Parallel buffer writes
  - [ ] Rapid alloc/free cycles
  - **Estimate**: 3 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

- [ ] **4.3.2**: Memory leak tests
  - [ ] Verify Drop implementations
  - [ ] Check for leaked allocations
  - [ ] Valgrind/ASAN runs
  - **Estimate**: 2 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

**Phase 4 Total**: ~19 hours (2.5 days)

---

## 🎨 Phase 5: Polish & Documentation (Days 13-15)

**Goal**: Production-ready code with excellent docs

### 5.1: Performance Optimization (Day 13)

- [ ] **5.1.1**: Memory pooling
  - [ ] Implement UnifiedMemoryPool
  - [ ] Size class bucketing
  - [ ] Cache for large allocations
  - **Estimate**: 4 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

- [ ] **5.1.2**: Smart synchronization
  - [ ] Lazy sync (only when needed)
  - [ ] Batch syncs
  - [ ] Conflict resolution
  - **Estimate**: 3 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

### 5.2: Metrics & Monitoring (Day 14)

- [ ] **5.2.1**: Implement UnifiedMemoryMetrics
  - [ ] Allocation stats
  - [ ] Sync stats
  - [ ] Pool hit rate
  - [ ] Backend usage
  - **Estimate**: 3 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

- [ ] **5.2.2**: Performance benchmarks
  - [ ] Compare vs traditional copy
  - [ ] Backend comparison
  - [ ] Overhead measurements
  - **Estimate**: 2 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

### 5.3: Documentation (Day 15)

- [ ] **5.3.1**: API documentation
  - [ ] Rustdoc for all public APIs
  - [ ] Code examples in doc comments
  - [ ] Safety comments for all unsafe
  - **Estimate**: 4 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

- [ ] **5.3.2**: User guides
  - [ ] `docs/guides/unified_memory.md`
  - [ ] Migration guide
  - [ ] Performance tuning guide
  - **Estimate**: 3 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

- [ ] **5.3.3**: Examples
  - [ ] `examples/unified_memory_basic.rs`
  - [ ] `examples/unified_memory_async.rs`
  - [ ] `examples/unified_memory_multi_buffer.rs`
  - [ ] `examples/unified_memory_benchmark.rs`
  - **Estimate**: 4 hours
  - **Owner**: TBD
  - **Status**: 📋 Ready

**Phase 5 Total**: ~23 hours (3 days)

---

## 📊 Timeline Summary

| Week | Phase | Hours | Days | Status |
|------|-------|-------|------|--------|
| **Week 1** | Phase 1: Core | 13 | 2 | 📋 Ready |
| | Phase 2: Backends | 27 | 3.5 | 📋 Ready |
| **Week 2** | Phase 3: Integration | 21 | 3 | 📋 Ready |
| | Phase 4: Testing | 19 | 2.5 | 📋 Ready |
| **Week 3** | Phase 5: Polish | 23 | 3 | 📋 Ready |
| **TOTAL** | | **103 hours** | **14 days** | 📋 Ready |

**With 8-hour days**: ~14 working days = **3 weeks**

---

## ✅ Success Criteria

### Must Have (MVP) - Week 2

- [x] Spec complete ✅
- [ ] Vulkan backend works on Intel, AMD, NVIDIA
- [ ] OpenCL backend works on Intel, AMD, NVIDIA
- [ ] CPU fallback always works
- [ ] Zero production unwraps
- [ ] Async-native API
- [ ] Comprehensive error handling
- [ ] Thread-safe, concurrent operations
- [ ] Unit tests passing
- [ ] Integration tests passing

### Should Have (Production) - Week 3

- [ ] WebGPU backend (pure Rust)
- [ ] Memory pooling optimization
- [ ] Smart synchronization
- [ ] Performance metrics
- [ ] Complete documentation
- [ ] 5+ examples
- [ ] Benchmarks

### Nice to Have (Future)

- [ ] Auto-tuning for workload patterns
- [ ] Multi-GPU unified memory
- [ ] Peer-to-peer GPU transfers
- [ ] NUMA-aware allocation

---

## 🎯 Milestones

### Milestone 1: Core Complete (End of Week 1)
- ✅ Module structure created
- ✅ Traits and types defined
- ✅ Vulkan backend working
- ✅ OpenCL backend working
- ✅ Basic tests passing

### Milestone 2: Integration Complete (End of Week 2)
- ✅ Scheduler integration done
- ✅ Memory detection working
- ✅ E2E tests passing
- ✅ All backends tested on real hardware

### Milestone 3: Production Ready (End of Week 3)
- ✅ All optimizations implemented
- ✅ Comprehensive documentation
- ✅ Examples and benchmarks
- ✅ Ready for merge to main

---

## 📈 Progress Tracking

### Daily Standup Template

```markdown
## Day X - [Date]

**Focus**: [Phase/Task]

**Completed**:
- [ ] Task 1
- [ ] Task 2

**In Progress**:
- [ ] Task 3 (60% complete)

**Blockers**:
- None / [Describe blocker]

**Tomorrow**:
- [ ] Continue Task 3
- [ ] Start Task 4

**Notes**:
- [Any insights, issues, or decisions]
```

### Weekly Review Template

```markdown
## Week X Review

**Planned**: X hours
**Actual**: Y hours
**Variance**: +/- Z hours

**Completed Tasks**: X/Y
**Completion Rate**: Z%

**Highlights**:
- [Major accomplishment]

**Challenges**:
- [Challenge and resolution]

**Next Week Focus**:
- [Key priorities]
```

---

## 🛠️ Development Environment

### Required Tools

```bash
# Rust toolchain
rustup toolchain install stable
rustup component add rustfmt clippy

# Vulkan SDK
# https://vulkan.lunarg.com/sdk/home

# OpenCL development headers
sudo apt install opencl-headers ocl-icd-opencl-dev  # Linux
brew install opencl-headers  # macOS

# GPU drivers (ensure latest)
# Intel: https://www.intel.com/content/www/us/en/download/785597/intel-arc-iris-xe-graphics-windows.html
# AMD: https://www.amd.com/en/support
# NVIDIA: https://www.nvidia.com/drivers
```

### Build Commands

```bash
# Build unified memory module
cargo build -p toadstool-runtime-gpu --features unified-memory

# Run tests
cargo test -p toadstool-runtime-gpu unified_memory

# Run with specific backend
TOADSTOOL_UNIFIED_MEMORY_BACKEND=vulkan cargo test

# Benchmarks
cargo bench --features unified-memory unified_memory_

# Documentation
cargo doc --no-deps --open
```

---

## 🔍 Code Review Checklist

Before marking tasks complete:

- [ ] Zero unwraps in production code
- [ ] All errors properly handled
- [ ] Async methods use `.await`, not blocking
- [ ] Thread-safety verified (Send + Sync)
- [ ] Unsafe code has SAFETY comments
- [ ] Tests cover happy and error paths
- [ ] Documentation is comprehensive
- [ ] Performance is acceptable (benchmarked)
- [ ] No memory leaks (tested with ASAN)
- [ ] Clippy warnings resolved
- [ ] Rustfmt applied

---

## 📝 Communication

### Updates

- **Daily**: Update this roadmap with completed tasks
- **Weekly**: Post summary to team chat/docs
- **Blockers**: Report immediately, don't wait

### Questions/Decisions

- **Architecture**: Document in specs/UNIVERSAL_UNIFIED_MEMORY.md
- **Implementation**: Add code comments or update this roadmap
- **Changes**: Update both spec and roadmap

---

## 🚀 Let's Go!

**Status**: 📋 READY TO START  
**Next Action**: Begin Phase 1, Task 1.1 (Create module structure)  
**Philosophy**: Deep solutions, zero debt, modern async Rust, sovereignty first 🍄

**Let's build world-class unified memory!** 💪

