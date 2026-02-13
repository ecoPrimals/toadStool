# 🍄 Fractal Composition Infrastructure Roadmap

**Date**: January 13, 2026  
**Status**: Planning  
**Vision**: Infrastructure for infinite workload composition  
**Pattern**: Songbird model - Build for hardest challenge, unlock simpler ones

---

## 🎯 Executive Summary

The biomeOS team has presented a transformative challenge: **build infrastructure that enables unknowable workload compositions**. Following the Songbird pattern (gaming comms → everything), we will design Toadstool to handle the most complex composition scenario imaginable, making all simpler cases trivial.

### The North Star Scenario

```
Gaming Tournament (20ms latency)
    + OpenFold Protein Folding (massive GPU compute)
    + Live Streaming (real-time encoding)
    + AI Training (GPU saturation)
Across:
    - Pop!_OS → biomeOS → SteamOS (nested OS layers)
    - Bare metal → AWS → Azure → back (dynamic migration)
    - 100% sovereignty (zero vendor lock-in)
```

**If Toadstool can compose THIS, it can compose ANYTHING.**

---

## 📊 Current State Assessment

### ✅ What Toadstool Already Has

**Strong Foundation**:
- ✅ **Capability-Based Discovery** (`infant_discovery` module)
  - Runtime primal discovery
  - Zero hardcoded primal names
  - Self-knowledge only

- ✅ **Diverse Workload Types** (`workload_types` module)
  - Native, WASM, Container, GPU, Python
  - AI/ML with intelligent backend selection
  - CUDA compatibility layer

- ✅ **GPU Abstraction** (`barraCUDA`)
  - Vendor-agnostic GPU compute
  - 18 operations proven, 4 more ready
  - Pure Rust, zero vendor lock-in

- ✅ **Distributed Coordination** (`distributed` crate)
  - Multi-primal coordination
  - Workload distribution

- ✅ **Deep Debt Principles**
  - No hardcoding
  - Runtime discovery
  - Self-knowledge
  - Zero mocks in production

### ❌ What's Missing for Fractal Composition

**Gap Analysis**:

1. **Multi-Layer Deployment Detection**
   - ❌ No detection of current layer (bare metal vs VM vs container vs cloud)
   - ❌ No adaptation of capabilities based on layer
   - ❌ No nested OS support (Pop!_OS ↔ biomeOS ↔ SteamOS)

2. **Dynamic Workload Composition**
   - ❌ No constraint-based composition engine
   - ❌ No priority graphs for competing workloads
   - ❌ No dynamic resource allocation across workloads

3. **Fractal Cloud Coordination**
   - ❌ No cloud provider abstraction
   - ❌ No local → cloud spillover
   - ❌ No cloud → local return migration
   - ❌ No multi-cloud failover

4. **Plugin System for Unknown Providers**
   - ❌ No generic `ComputeProvider` trait
   - ❌ No plugin registration system
   - ❌ No support for future providers

---

## 🎯 Four-Phase Roadmap

### Phase 1: Multi-Layer OS Support (Week 1)
**Goal**: Toadstool works at ANY layer (bare metal, middleware, service, container, VM, cloud)

**Deliverables**:
```rust
// 1. Layer detection
pub enum DeploymentLayer {
    BareMetalOS,      // biomeOS IS the OS
    MiddlewareLayer,  // biomeOS on Pop!_OS
    ServiceLayer,     // biomeOS provides to SteamOS
    ContainerLayer,   // biomeOS in Docker/Podman
    VMLayer,          // biomeOS in QEMU/KVM
    CloudLayer,       // biomeOS in EC2/GCE/Azure
}

// 2. Auto-detection
impl Toadstool {
    pub async fn detect_deployment_layer() -> DeploymentLayer;
}

// 3. Layer-aware capabilities
impl Toadstool {
    pub async fn adapt_capabilities_for_layer(
        &self,
        layer: DeploymentLayer
    ) -> Vec<Capability>;
}
```

**Test Scenario**: 
- Pop!_OS (base) → biomeOS (middleware) → SteamOS (service layer)
- Verify GPU capabilities propagate correctly through all layers

**Files to Create**:
- `crates/core/toadstool/src/deployment_layer.rs` (new)
- `crates/core/toadstool/src/layer_detection.rs` (new)
- `crates/core/toadstool/tests/multi_layer_tests.rs` (new)

---

### Phase 2: Dynamic Workload Composition (Week 2)
**Goal**: Compose N unknown workloads with M constraints, finding valid execution plan

**Deliverables**:
```rust
// 1. Composition constraints
pub struct CompositionConstraints {
    hard_latency: Option<Duration>,        // MUST satisfy
    soft_gpu_preference: Option<f32>,      // SHOULD satisfy
    bandwidth_requirement: Option<Bandwidth>,
    fallback_strategy: FallbackStrategy,
}

// 2. Priority graph
pub struct DynamicPriorityGraph {
    workloads: Vec<Workload>,
    constraints: Vec<CompositionConstraints>,
    dependencies: Graph<WorkloadId, Dependency>,
}

// 3. Composition engine
impl Toadstool {
    pub async fn compose_workloads(
        &self,
        workloads: Vec<Workload>,
        constraints: Vec<CompositionConstraints>,
    ) -> Result<CompositionPlan>;
}
```

**Test Scenario**:
- Gaming tournament (hard: <20ms latency, GPU priority)
- OpenFold job (soft: prefer GPU when available)
- Live streaming (hard: 5mbps bandwidth, CPU cores)
- AI commentary (soft: can use CPU fallback)

**Files to Create**:
- `crates/core/toadstool/src/composition/mod.rs` (new)
- `crates/core/toadstool/src/composition/constraints.rs` (new)
- `crates/core/toadstool/src/composition/engine.rs` (new)
- `crates/core/toadstool/src/composition/priority_graph.rs` (new)
- `crates/core/toadstool/tests/composition_tests.rs` (new)

---

### Phase 3: Fractal Cloud Coordination (Week 3)
**Goal**: Seamless local ↔ cloud ↔ local migration with zero hardcoding

**Deliverables**:
```rust
// 1. Generic compute provider trait
#[async_trait]
pub trait ComputeProvider: Send + Sync {
    async fn capabilities(&self) -> Vec<Capability>;
    async fn execute(&self, workload: Workload) -> Result<ExecutionHandle>;
    async fn cost_estimate(&self, workload: Workload) -> Result<Cost>;
    async fn migrate_from(&self, handle: ExecutionHandle) -> Result<()>;
}

// 2. Cloud provider enum (extensible)
pub enum CloudProvider {
    AWS { region: String, credentials: Capability },
    Azure { region: String, credentials: Capability },
    GCP { region: String, credentials: Capability },
    Custom { endpoint: String, auth: Capability },
    Plugin(Box<dyn ComputeProvider>),  // For future providers!
}

// 3. Fractal coordination
pub struct FractalCoordinator {
    local: LocalComputePool,
    regional: Vec<Box<dyn ComputeProvider>>,
    global: Vec<Box<dyn ComputeProvider>>,
}

impl FractalCoordinator {
    pub async fn execute_with_failover(
        &self,
        workload: Workload,
        preferences: ExecutionPreferences,
    ) -> Result<ExecutionHandle>;
}
```

**Test Scenario**:
1. Start workload on local GPU (lowest latency)
2. Saturate local GPU with gaming
3. Automatic spillover to AWS
4. Gaming ends, GPU available
5. Automatic return to local
6. AWS fails → failover to Azure
7. All without hardcoding

**Files to Create**:
- `crates/distributed/src/providers/mod.rs` (new)
- `crates/distributed/src/providers/local.rs` (new)
- `crates/distributed/src/providers/aws.rs` (new)
- `crates/distributed/src/providers/azure.rs` (new)
- `crates/distributed/src/providers/gcp.rs` (new)
- `crates/distributed/src/fractal_coordinator.rs` (new)
- `crates/distributed/tests/fractal_tests.rs` (new)

---

### Phase 4: Plugin System for Unknown Providers (Week 4)
**Goal**: Support compute providers that don't exist yet

**Deliverables**:
```rust
// 1. Plugin system
pub struct ProviderRegistry {
    providers: Vec<Box<dyn ComputeProvider>>,
}

impl ProviderRegistry {
    pub fn register<P: ComputeProvider + 'static>(&mut self, provider: P) {
        self.providers.push(Box::new(provider));
    }
    
    pub async fn discover_providers(&self) -> Vec<&dyn ComputeProvider> {
        self.providers.iter()
            .map(|p| p.as_ref())
            .collect()
    }
}

// 2. Example future provider
pub struct QuantumCloudProvider2027 {
    endpoint: String,
    quantum_api: QuantumAPI,
}

#[async_trait]
impl ComputeProvider for QuantumCloudProvider2027 {
    async fn execute(&self, workload: Workload) -> Result<ExecutionHandle> {
        // Quantum-specific execution
        // Toadstool doesn't need to know about quantum!
        self.quantum_api.submit_circuit(workload).await
    }
}
```

**Test Scenario**:
1. Create mock "FutureProvider2027" plugin
2. Register it with Toadstool
3. Submit workload without knowing provider exists
4. Verify Toadstool discovers and uses it
5. Zero core changes needed

**Files to Create**:
- `crates/distributed/src/providers/plugin.rs` (new)
- `crates/distributed/src/providers/registry.rs` (new)
- `crates/distributed/tests/plugin_tests.rs` (new)
- `examples/custom_provider_plugin.rs` (new)

---

## 🎓 Success Criteria

### You Know It's Working When:

1. **Multi-Layer Test**
   ```
   Pop!_OS → biomeOS → SteamOS
   - All layers detect correctly
   - GPU capabilities propagate
   - No special configuration needed
   ```

2. **Composition Test**
   ```
   Gaming (20ms) + OpenFold (GPU) + Streaming (5mbps) + AI (fallback)
   - All run simultaneously
   - Constraints satisfied
   - Resources allocated dynamically
   ```

3. **Fractal Test**
   ```
   Local GPU → (saturate) → AWS → (available) → Local
   - Seamless migration
   - No data loss
   - No hardcoded providers
   ```

4. **Plugin Test**
   ```
   Unknown provider plugin registered → Used automatically
   - Zero core changes
   - Discovery works
   - Execution succeeds
   ```

---

## 🏗️ Architecture Principles

### 1. Composition Over Code
```
BAD:  if gaming && openfold { special_case(); }
GOOD: compose([gaming, openfold], constraints)
```

### 2. Discovery Over Hardcoding
```
BAD:  if aws { use_aws_api(); }
GOOD: for provider in discover() { try(provider); }
```

### 3. Adaptation Over Assumption
```
BAD:  assert!(bare_metal);
GOOD: layer = detect(); adapt_to(layer);
```

### 4. Constraint Over Prescription
```
BAD:  gaming_gets_gpu(); openfold_gets_cpu();
GOOD: satisfy([gaming.latency < 20ms, openfold.prefer_gpu])
```

---

## 📁 File Structure

```
crates/
├── core/
│   └── toadstool/
│       └── src/
│           ├── deployment_layer.rs       (Phase 1)
│           ├── layer_detection.rs        (Phase 1)
│           └── composition/              (Phase 2)
│               ├── mod.rs
│               ├── constraints.rs
│               ├── engine.rs
│               └── priority_graph.rs
├── distributed/
│   └── src/
│       ├── providers/                    (Phase 3 & 4)
│       │   ├── mod.rs
│       │   ├── local.rs
│       │   ├── aws.rs
│       │   ├── azure.rs
│       │   ├── gcp.rs
│       │   ├── plugin.rs
│       │   └── registry.rs
│       └── fractal_coordinator.rs        (Phase 3)
└── tests/
    ├── multi_layer_tests.rs              (Phase 1)
    ├── composition_tests.rs              (Phase 2)
    ├── fractal_tests.rs                  (Phase 3)
    └── plugin_tests.rs                   (Phase 4)
```

---

## 🎯 Integration with Existing Code

### barraCUDA Integration
- Use barraCUDA for GPU workloads across all layers
- Cloud providers use barraCUDA for GPU operations
- Layer detection influences GPU capability exposure

### Infant Discovery Integration
- Extend infant discovery to discover cloud providers
- Layer-aware capability advertisement
- Dynamic provider registration

### Workload Types Integration
- Composition engine uses existing `WorkloadSpec`
- Add `ComposedWorkload` variant
- Constraints integrate with workload characteristics

---

## 📊 Timeline

| Phase | Duration | Deliverable | Test |
|-------|----------|-------------|------|
| **Phase 1** | Week 1 | Multi-layer detection | Pop!_OS → biomeOS → SteamOS |
| **Phase 2** | Week 2 | Composition engine | Gaming + OpenFold + Streaming |
| **Phase 3** | Week 3 | Fractal coordination | Local → AWS → Local |
| **Phase 4** | Week 4 | Plugin system | Unknown provider works |

**Total**: 4 weeks for complete infrastructure

---

## 🔄 Iterative Approach

### Week 1 (Phase 1)
- Day 1-2: Layer detection (bare metal, VM, container)
- Day 3-4: Capability adaptation per layer
- Day 5: Testing with Pop!_OS/biomeOS/SteamOS stack

### Week 2 (Phase 2)
- Day 1-2: Constraint system
- Day 3: Priority graph
- Day 4: Composition engine
- Day 5: Multi-workload testing

### Week 3 (Phase 3)
- Day 1: ComputeProvider trait
- Day 2-3: AWS/Azure/GCP implementations
- Day 4: Fractal coordinator
- Day 5: Migration testing

### Week 4 (Phase 4)
- Day 1-2: Plugin system
- Day 3: Provider registry
- Day 4-5: Custom provider testing

---

## 🎓 Why This Enables Everything

### The Songbird Parallel

**Songbird**: Gaming (tightest) → Scientific + Security + IoT (simpler)

**Toadstool**: Impossible composition → All simple cases

### Examples That "Fall Out For Free"

If Toadstool handles **Gaming + OpenFold + Streaming + AI** across **nested OS** with **cloud failover**...

Then it **trivially** handles:
- ✅ Just gaming (one workload, simpler)
- ✅ Just science (one workload, simpler)
- ✅ Single cloud (one provider, simpler)
- ✅ Single layer (no nesting, simpler)
- ✅ Known providers (no plugins, simpler)

**Build for complexity → Inherit simplicity**

---

## 🎯 Definition of Done

### When Can We Say This Is Complete?

Someone says:
> "I want gaming tournament + protein folding simultaneously,  
> with local GPUs and cloud failover, on Pop!_OS with SteamOS guests"

Response is:
> "Just describe workloads and constraints. Toadstool composes it."

NOT:
> "Let me write custom code for that scenario."

---

## 📈 Measuring Success

### Quantitative Metrics
- **Composition Tests**: 100% passing for N workloads × M constraints
- **Layer Detection**: 100% accuracy across 6 layers
- **Provider Plugins**: Zero core changes for new provider
- **Migration Time**: <5s for local ↔ cloud

### Qualitative Metrics
- **Unknowable Scenarios**: System composes without hardcoding
- **Developer Experience**: New workload types "just work"
- **Provider Flexibility**: Cloud providers swappable
- **Future-Proof**: 2027 providers work via plugin

---

## 🚀 Next Steps

1. **Review with Team**: Validate approach and priorities
2. **Create Phase 1 Branch**: `feature/multi-layer-detection`
3. **Start Smallest Piece**: Layer detection for bare metal vs VM
4. **Incremental Testing**: Each piece tested before next
5. **Document As We Go**: Architecture decisions captured

---

## 🎯 Bottom Line

**This is not about writing gaming code or science code.**  
**This is about building the substrate that makes ALL compositions possible.**

**Infrastructure that enables unknowable futures.**

---

**"Different orders of the same architecture - composed at runtime, not compile time."** 🍄

---

**Status**: Roadmap Complete  
**Next**: Team review and Phase 1 kickoff  
**Timeline**: 4 weeks  
**Confidence**: HIGH (building on solid barraCUDA foundation)
