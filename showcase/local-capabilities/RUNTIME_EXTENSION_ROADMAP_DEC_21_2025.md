# ToadStool Runtime Extension Roadmap
## Python, Container, and GPU Runtime Support

**Date**: December 21, 2025  
**Status**: Planning Phase  
**Priority**: Medium (Level 0 completion)

---

## 🎯 Current State

### ✅ What Works Now
- **Native Runtime**: Full support via `UniversalJobType::Native`
- **WASM Runtime**: Full support via `UniversalJobType::Wasm`
- **Primal Runtime**: Delegation via `UniversalJobType::Primal`
- **BiomeOS Runtime**: Orchestration via `UniversalJobType::BiomeOS`

### ⚠️ What's Missing from UniversalJobType
- **Python Runtime**: Engine exists (`toadstool-runtime-python`) but not in `UniversalJobType`
- **Container Runtime**: Engine exists (`toadstool-runtime-container`) but not in `UniversalJobType`
- **GPU Runtime**: Engine exists (`toadstool-runtime-gpu`) but not in `UniversalJobType`

---

## 📋 Extension Plan

### Phase 1: Python Runtime (Highest Priority)
**Goal**: Add Python support to `UniversalJobType`

**Changes Required**:
1. Extend `UniversalJobType` enum in `crates/core/toadstool/src/universal/jobs.rs`:
   ```rust
   pub enum UniversalJobType {
       // ... existing variants ...
       Python {
           source: PythonSource,  // Script, File, or Module
           python_version: Option<String>,
           packages: Vec<String>,
           env: HashMap<String, String>,
       },
   }
   ```

2. Update `UniversalComputePlatform` to route Python jobs to `toadstool-runtime-python`

3. Create Python demo: `demo_python.rs` → `demo-python-execution`

**Estimated Effort**: 4-6 hours  
**Dependencies**: None (runtime already exists)  
**Testing**: Unit tests + integration demo

---

### Phase 2: Container Runtime
**Goal**: Add Container (Docker/Podman) support to `UniversalJobType`

**Changes Required**:
1. Extend `UniversalJobType` enum:
   ```rust
   pub enum UniversalJobType {
       // ... existing variants ...
       Container {
           image: String,
           tag: Option<String>,
           command: Option<Vec<String>>,
           env: HashMap<String, String>,
           volumes: Vec<VolumeMount>,
       },
   }
   ```

2. Update routing logic for container runtime

3. Create container demo

**Estimated Effort**: 6-8 hours  
**Dependencies**: Docker/Podman installed  
**Testing**: Unit tests + integration demo

---

### Phase 3: GPU Runtime
**Goal**: Add GPU execution support to `UniversalJobType`

**Changes Required**:
1. Extend `UniversalJobType` enum:
   ```rust
   pub enum UniversalJobType {
       // ... existing variants ...
       Gpu {
           compute_type: GpuComputeType,  // CUDA, ROCm, Vulkan, etc.
           kernel_source: Vec<u8>,
           args: Vec<GpuKernelArg>,
           device_requirements: GpuDeviceRequirements,
       },
   }
   ```

2. Update routing to GPU runtime

3. Create GPU demo

**Estimated Effort**: 8-12 hours  
**Dependencies**: GPU hardware, CUDA/ROCm drivers  
**Testing**: Unit tests + GPU hardware demo

---

## 🔍 Technical Details

### Current `UniversalJobType` Location
```
File: crates/core/toadstool/src/universal/jobs.rs
Lines: 30-56

pub enum UniversalJobType {
    Native { executable, args, env },
    Wasm { module, args, env },
    Primal { primal_type, endpoint, payload },
    BiomeOS { biome_manifest, team_id },
}
```

### Existing Runtime Engines (Not Yet Exposed)
1. **`toadstool-runtime-python`**:
   - Location: `crates/runtime/python/`
   - Status: Implemented
   - Capability: Execute Python scripts with PyO3

2. **`toadstool-runtime-container`**:
   - Location: `crates/runtime/container/`
   - Status: Implemented (BYOB - Bring Your Own Backend)
   - Capability: Docker, Podman, Kubernetes support

3. **`toadstool-runtime-gpu`**:
   - Location: `crates/runtime/gpu/`
   - Status: Implemented
   - Capability: CUDA, ROCm, Vulkan compute

---

## 🎯 Success Criteria

### Phase 1: Python (Complete when...)
- [  ] `UniversalJobType::Python` variant added
- [  ] Python jobs route to `toadstool-runtime-python`
- [  ] `demo-python-execution` binary works
- [  ] Execution receipts show real Python execution
- [  ] Unit tests passing
- [  ] Documentation updated

### Phase 2: Container (Complete when...)
- [  ] `UniversalJobType::Container` variant added
- [  ] Container jobs route to `toadstool-runtime-container`
- [  ] `demo-container-execution` binary works
- [  ] Can run Docker images
- [  ] Unit tests passing
- [  ] Documentation updated

### Phase 3: GPU (Complete when...)
- [  ] `UniversalJobType::Gpu` variant added
- [  ] GPU jobs route to `toadstool-runtime-gpu`
- [  ] `demo-gpu-execution` binary works
- [  ] Can execute CUDA/ROCm kernels
- [  ] Unit tests passing
- [  ] Documentation updated

---

## 📊 Impact Assessment

### User Experience Impact
**Before**:
- Level 0: 33% complete (2/6 runtimes)
- Limited showcase demonstrations
- Shell script mocks (now removed)

**After Phase 1 (Python)**:
- Level 0: 50% complete (3/6 runtimes)
- Python ML/AI demos possible
- More realistic showcase

**After Phase 2 (Container)**:
- Level 0: 66% complete (4/6 runtimes)
- Container orchestration demos
- Production deployment patterns

**After Phase 3 (GPU)**:
- Level 0: 83% complete (5/6 runtimes)
- GPU acceleration demos
- High-performance computing showcase

---

## 🚀 Recommended Sequence

1. **Phase 1: Python** (Highest ROI)
   - Most requested runtime
   - ML/AI ecosystem integration
   - Relatively quick implementation

2. **Phase 2: Container** (Medium ROI)
   - Production deployment relevance
   - Kubernetes integration potential
   - Moderate complexity

3. **Phase 3: GPU** (Specialized ROI)
   - HPC and ML acceleration
   - Requires hardware
   - Higher complexity

---

## 🔗 Related Work

### Already Complete
- ✅ Native runtime in `UniversalJobType`
- ✅ WASM runtime in `UniversalJobType`
- ✅ `UniversalComputePlatform` API
- ✅ Job submission and tracking
- ✅ Resource management framework

### Needs No Changes
- ✅ Runtime engine implementations (all exist)
- ✅ Resource monitoring
- ✅ Security/sandboxing infrastructure
- ✅ Job scheduling logic

### Only Needs
- ⚠️ Enum variants in `UniversalJobType`
- ⚠️ Routing logic updates
- ⚠️ Demo creation
- ⚠️ Documentation

---

## 💡 Key Insight

**The runtime engines already exist!**

We're not building new runtimes - we're just exposing them through the `UniversalJobType` enum and `UniversalComputePlatform` API.

**Estimated Total Effort**: 18-26 hours for all 3 phases  
**Complexity**: Low-Medium (plumbing, not new features)  
**Risk**: Low (runtimes are tested and working)

---

## 📝 Next Steps

1. **Immediate**: Document this roadmap ✅ (this file)
2. **Short-term**: Implement Phase 1 (Python) - 4-6 hours
3. **Medium-term**: Implement Phase 2 (Container) - 6-8 hours
4. **Long-term**: Implement Phase 3 (GPU) - 8-12 hours

---

*Roadmap Created*: December 21, 2025  
*Priority*: Medium (after core stabilization)  
*Complexity*: Low-Medium  
*Impact*: High (showcase completion)

