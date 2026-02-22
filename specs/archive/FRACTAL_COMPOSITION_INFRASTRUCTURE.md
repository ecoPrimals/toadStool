# Fractal Composition Infrastructure - Technical Specification

**Version**: 1.0  
**Date**: January 13, 2026  
**Status**: Phase 1 In Progress  
**Owner**: Toadstool Team

---

## 🎯 Vision

Enable Toadstool to compose unknowable workload combinations across heterogeneous compute environments (bare metal, VMs, containers, clouds) with nested OS stacks (Pop!_OS ↔ biomeOS ↔ SteamOS), using constraint-based composition rather than hardcoded logic.

**Core Principle**: Build for the hardest challenge, unlock all simpler ones (Songbird pattern).

---

## 📋 Requirements

### Functional Requirements

**FR1: Multi-Layer Detection**
- MUST detect 6 deployment layers: BareMetalOS, MiddlewareLayer, ServiceLayer, ContainerLayer, VMLayer, CloudLayer
- MUST auto-detect environment without configuration
- MUST cache detection results for performance
- SHOULD provide rich metadata (host OS, cloud provider, container runtime, etc.)

**FR2: Capability Adaptation**
- MUST adapt GPU capabilities based on deployment layer
- MUST adapt storage capabilities based on deployment layer
- MUST adapt network capabilities based on deployment layer
- MUST expose appropriate APIs for each layer

**FR3: Dynamic Workload Composition**
- MUST compose N workloads with M constraints
- MUST support hard constraints (MUST satisfy, e.g., <20ms latency)
- MUST support soft constraints (SHOULD satisfy, e.g., prefer GPU)
- MUST compute valid execution plan or fail with clear reason
- MUST handle conflicting constraints gracefully

**FR4: Fractal Cloud Coordination**
- MUST support local ↔ cloud migration
- MUST support multi-cloud failover (AWS → Azure → GCP)
- MUST return to local when capacity available
- MUST NOT hardcode cloud providers
- MUST work with providers that don't exist yet (via plugins)

**FR5: Plugin System**
- MUST allow registration of unknown compute providers
- MUST discover providers at runtime
- MUST work with zero core changes for new providers

### Non-Functional Requirements

**NFR1: Performance**
- Layer detection: <100ms for cached results
- Cloud metadata: <1s timeout per endpoint
- Composition computation: <5s for 10 workloads × 10 constraints
- Migration: <30s for local ↔ cloud

**NFR2: Reliability**
- Layer detection: 99.9% accuracy
- Composition: 100% constraint satisfaction or clear failure
- Migration: Zero data loss
- Failover: <5s detection, <30s recovery

**NFR3: Security**
- Cloud credentials: Never logged or exposed
- Container isolation: Respect namespace boundaries
- VM isolation: No cross-VM resource access
- Plugin sandbox: TBD (future work)

**NFR4: Deep Debt Compliance**
- Zero hardcoded providers
- Zero hardcoded OS names
- Runtime discovery only
- No mocks in production
- Self-knowledge only (no global state)

---

## 🏗️ Architecture

### Component Overview

```
┌─────────────────────────────────────────────────────────┐
│                   Application Layer                      │
│  (Gaming, Science, Streaming, AI workloads)             │
└───────────────────────┬─────────────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────────────┐
│            Composition Engine (Phase 2)                  │
│  - Constraint solver                                     │
│  - Priority graph                                        │
│  - Resource allocation                                   │
└───────────────────────┬─────────────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────────────┐
│         Fractal Coordinator (Phase 3)                    │
│  - Local compute pool                                    │
│  - Cloud provider registry                               │
│  - Migration engine                                      │
└───────────────────────┬─────────────────────────────────┘
                        │
        ┌───────────────┼───────────────┐
        │               │               │
┌───────▼──────┐ ┌─────▼──────┐ ┌─────▼──────┐
│   Local      │ │   Cloud     │ │  Plugin    │
│  Provider    │ │  Providers  │ │ Providers  │
│              │ │ (AWS/GCP/   │ │ (Future)   │
│              │ │  Azure)     │ │            │
└───────┬──────┘ └─────┬──────┘ └─────┬──────┘
        │               │               │
┌───────▼───────────────▼───────────────▼──────┐
│        Layer Detection (Phase 1)              │
│  - DeploymentLayer enum                       │
│  - LayerDetector                              │
│  - Capability adaptation                      │
└───────────────────────────────────────────────┘
```

### Data Flow

**1. Startup Detection**:
```
Application Start
    ↓
LayerDetector.detect()
    ↓
Check Container → Cloud → VM → Middleware → Service
    ↓
Cache Result (DeploymentLayer)
    ↓
Adapt Capabilities
    ↓
Register with Discovery
```

**2. Workload Submission**:
```
User submits: [Gaming, OpenFold, Streaming, AI]
    ↓
CompositionEngine.compose(workloads, constraints)
    ↓
Build priority graph
    ↓
Solve constraints
    ↓
Allocate resources (local vs cloud)
    ↓
Return CompositionPlan
    ↓
Execute via FractalCoordinator
```

**3. Cloud Spillover**:
```
Local GPU saturated
    ↓
FractalCoordinator.execute_with_failover()
    ↓
Try local (ResourceSaturated)
    ↓
Try cloud providers in order
    ↓
Migrate workload to cloud
    ↓
Monitor local availability
    ↓
Return to local when available
```

---

## 📊 Phase Breakdown

### Phase 1: Multi-Layer OS Support (Week 1)

**Status**: 40% Complete

**Components**:

1. **DeploymentLayer enum** ✅ (Done)
   - 6 layer types
   - Rich metadata per layer
   - Display/debug traits

2. **LayerDetector** ✅ (Done)
   - Container detection (Docker/Podman)
   - Cloud metadata (AWS/GCP/Azure)
   - VM detection (QEMU/KVM/VMware)
   - Middleware/Service detection
   - Caching

3. **Capability Adaptation** ⏳ (In Progress)
   - GPU capabilities per layer
   - Storage capabilities per layer
   - Network capabilities per layer
   - Integration with primal discovery

4. **Testing** 📋 (Pending)
   - Unit tests (basic - done)
   - Integration tests (Pop!_OS stack)
   - End-to-end validation

**Files**:
- `crates/core/toadstool/src/deployment_layer.rs` (611 lines) ✅
- `crates/core/toadstool/src/layer_adaptation.rs` (TBD)
- `crates/core/toadstool/tests/multi_layer_tests.rs` (TBD)

**Success Criteria**:
- [x] Layer detection works on all 6 layers
- [ ] Capabilities adapt correctly per layer
- [ ] Pop!_OS → biomeOS → SteamOS test passes
- [ ] GPU propagates through all layers

---

### Phase 2: Dynamic Workload Composition (Week 2)

**Status**: Not Started

**Components**:

1. **Constraint System**
   - Hard constraints (MUST)
   - Soft constraints (SHOULD)
   - Constraint validation
   - Conflict detection

2. **Priority Graph**
   - Workload dependencies
   - Resource requirements
   - Execution ordering
   - Parallel execution paths

3. **Composition Engine**
   - Constraint solver
   - Resource allocation
   - Execution plan generation
   - Failure explanation

4. **Testing**
   - Constraint validation tests
   - Composition correctness tests
   - Gaming + OpenFold + Streaming + AI test
   - Conflict handling tests

**Files**:
- `crates/core/toadstool/src/composition/mod.rs`
- `crates/core/toadstool/src/composition/constraints.rs`
- `crates/core/toadstool/src/composition/engine.rs`
- `crates/core/toadstool/src/composition/priority_graph.rs`
- `crates/core/toadstool/tests/composition_tests.rs`

**Success Criteria**:
- [ ] Compose 4+ workloads with 10+ constraints
- [ ] Gaming (<20ms) + OpenFold (GPU) + Streaming (5mbps) + AI (fallback)
- [ ] Detect and report constraint conflicts
- [ ] <5s composition time for realistic scenarios

---

### Phase 3: Fractal Cloud Coordination (Week 3)

**Status**: Not Started

**Components**:

1. **ComputeProvider Trait**
   - Generic provider interface
   - Capability discovery
   - Execution API
   - Migration API
   - Cost estimation

2. **Cloud Providers**
   - AWS implementation
   - Azure implementation
   - GCP implementation
   - Local provider

3. **Fractal Coordinator**
   - Provider registry
   - Failover logic
   - Migration engine
   - Return-to-local monitoring

4. **Testing**
   - Provider interface tests
   - Local → cloud spillover test
   - Cloud → local return test
   - Multi-cloud failover test

**Files**:
- `crates/distributed/src/providers/mod.rs`
- `crates/distributed/src/providers/trait.rs`
- `crates/distributed/src/providers/local.rs`
- `crates/distributed/src/providers/aws.rs`
- `crates/distributed/src/providers/azure.rs`
- `crates/distributed/src/providers/gcp.rs`
- `crates/distributed/src/fractal_coordinator.rs`
- `crates/distributed/tests/fractal_tests.rs`

**Success Criteria**:
- [ ] Local GPU → AWS spillover automatic
- [ ] AWS → Local return when available
- [ ] AWS fail → Azure failover seamless
- [ ] <30s migration time
- [ ] Zero data loss

---

### Phase 4: Plugin System (Week 4)

**Status**: Not Started

**Components**:

1. **Plugin Registry**
   - Dynamic provider registration
   - Provider discovery
   - Plugin lifecycle management

2. **Plugin API**
   - Stable trait interface
   - Version compatibility
   - Error handling

3. **Example Plugins**
   - Mock "FutureProvider2027"
   - Custom on-prem provider
   - Edge compute provider

4. **Testing**
   - Plugin registration tests
   - Discovery tests
   - Custom provider integration tests

**Files**:
- `crates/distributed/src/providers/plugin.rs`
- `crates/distributed/src/providers/registry.rs`
- `crates/distributed/tests/plugin_tests.rs`
- `examples/custom_provider_plugin.rs`

**Success Criteria**:
- [ ] Register unknown provider at runtime
- [ ] Provider discovered automatically
- [ ] Execution works without core changes
- [ ] Future providers "just work"

---

## 🎯 Key Design Decisions

### Decision 1: Async-First Architecture

**Rationale**: Cloud metadata, migration, and coordination are inherently async operations.

**Impact**: 
- Requires tokio runtime
- All detection methods async
- Better scalability
- Slightly more complex API

**Trade-offs**: Worth it for non-blocking operations

---

### Decision 2: Trait-Based Provider Abstraction

**Rationale**: Enables unknown future providers without core changes.

**Impact**:
- Dynamic dispatch (small perf cost)
- Plugin-ready from day 1
- Future-proof
- Clean separation of concerns

**Trade-offs**: Slight performance overhead acceptable for flexibility

---

### Decision 3: Constraint-Based Composition

**Rationale**: Prescriptive allocation doesn't scale to unknown workload combinations.

**Impact**:
- More complex engine
- Better flexibility
- Handles unknowable futures
- Clear failure modes

**Trade-offs**: Complexity justified by infinite composition capability

---

### Decision 4: Cached Layer Detection

**Rationale**: Environment doesn't change during process lifetime.

**Impact**:
- Fast subsequent calls (<1ms)
- Single detection overhead (~100ms)
- Can reset() if needed
- Simple implementation

**Trade-offs**: Assumes static environment (acceptable)

---

## 🔄 Integration Points

### With Infant Discovery

**Current**: Primals discover each other via mDNS/Consul

**Integration**: 
- Layer detection influences capability advertisement
- Bare metal: Advertise full GPU capabilities
- Middleware: Advertise host-provided capabilities
- Service: Request capabilities from primal discovery
- Cloud: Advertise cloud GPU capabilities

**Changes**:
- Add `deployment_layer` to capability advertisement
- Filter capabilities based on layer
- Enable layer-aware discovery

---

### With barraCuda

**Current**: 18 GPU operations, vendor-agnostic

**Integration**:
- Use barraCuda for all GPU workloads
- Layer influences GPU access method
- Cloud providers use barraCuda with cloud GPUs
- True multi-vendor testing via fractal composition

**Future**:
- Test barraCuda across local → AWS (NVIDIA) → Azure (AMD)
- Verify vendor lock-in breaking via real workload migration
- Benchmark GPU ops across all providers

---

### With Workload Types

**Current**: Native, WASM, Container, GPU, Python, AI/ML, CUDA

**Integration**:
- Add `ComposedWorkload` variant
- Composition engine uses existing `WorkloadSpec`
- Layer influences workload scheduling
- Constraints map to workload characteristics

**Changes**:
- Extend `WorkloadSpec` enum
- Add composition metadata
- Enable constraint specification

---

## 📈 Success Metrics

### Quantitative

**Layer Detection**:
- Accuracy: >99.9%
- Latency: <100ms cached, <1s uncached
- Coverage: 6 layers supported

**Composition**:
- Constraint satisfaction: 100% or clear failure
- Composition time: <5s for 10 workloads × 10 constraints
- Failure explanation quality: Human-readable

**Migration**:
- Time: <30s local ↔ cloud
- Data loss: 0%
- Detection latency: <5s

**Plugin System**:
- Registration: Dynamic at runtime
- Core changes: 0 for new provider
- Discovery time: <1s

### Qualitative

**Unknowable Composition**:
- System handles workload combinations we didn't anticipate
- No special-case code required
- Constraints expressed, not prescribed

**Developer Experience**:
- New workload types "just work"
- Cloud providers swappable
- Clear error messages

**Future-Proof**:
- 2027 providers work via plugin
- New constraint types addable
- Architecture scales to cosmic computing

---

## 🎓 Deep Debt Compliance

### Principle 1: No Hardcoding

**Applied**:
- ✅ No hardcoded cloud providers (trait-based)
- ✅ No hardcoded OS names (runtime detection)
- ✅ No hardcoded layer assumptions (detect and adapt)
- ✅ No special-case composition logic (constraint-based)

### Principle 2: Runtime Discovery

**Applied**:
- ✅ Layer detection at runtime
- ✅ Cloud metadata discovered
- ✅ Provider capabilities discovered
- ✅ Workload requirements discovered

### Principle 3: Self-Knowledge Only

**Applied**:
- ✅ LayerDetector knows only itself
- ✅ Providers know only their capabilities
- ✅ Workloads know only their requirements
- ✅ No global orchestrator state

### Principle 4: No Mocks in Production

**Applied**:
- ✅ Real cloud metadata endpoints
- ✅ Real container detection
- ✅ Real VM indicators
- ✅ Mocks only in tests

---

## 🚀 Timeline

| Week | Phase | Deliverable | Status |
|------|-------|-------------|--------|
| **1** | Multi-Layer | Detection + Adaptation | 40% ✅ |
| **2** | Composition | Constraint engine | 0% 📋 |
| **3** | Fractal | Cloud coordination | 0% 📋 |
| **4** | Plugin | Provider system | 0% 📋 |

**Current**: Week 1, Day 1  
**On Track**: Yes  
**Blockers**: None

---

## 📚 References

- `FRACTAL_COMPOSITION_ROADMAP.md` - Detailed roadmap
- `BIOMEOS_FRACTAL_COMPOSITION_RESPONSE.md` - Formal response
- `FRACTAL_COMPOSITION_KICKOFF_JAN13.md` - Day 1 summary
- Songbird evolution docs (parallel pattern)
- barraCuda achievements (velocity proof)

---

## ✅ Definition of Done

**Each Phase Complete When**:
- [ ] All components implemented
- [ ] Tests passing (unit + integration)
- [ ] Documentation complete
- [ ] Success criteria met
- [ ] Demo to biomeOS team
- [ ] Zero technical debt

**Project Complete When**:
- [ ] All 4 phases complete
- [ ] Gaming + OpenFold + Streaming + AI test passes
- [ ] Local → Cloud → Local migration works
- [ ] Unknown provider plugin works
- [ ] biomeOS team acceptance

---

**Version**: 1.0  
**Date**: January 13, 2026  
**Status**: Phase 1 In Progress (40%)  
**Next Review**: End of Week 1
