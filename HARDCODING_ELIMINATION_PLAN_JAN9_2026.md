# Hardcoding Elimination Plan - January 9, 2026

**Goal**: Evolve ToadStool to infant discovery pattern - start with zero knowledge, discover everything at runtime

**Philosophy** (from `primal_identity.rs`):
- **Self-Knowledge**: Each primal knows only itself
- **Runtime Discovery**: Other primals discovered via capabilities
- **Zero Hardcoding**: No primal-specific code or configuration
- **Capability-Based**: Services matched by what they can do, not who they are

---

## 🎯 Current State Analysis

### Hardcoding Found
1. **Primal Names**: 10,424 matches across 875 files
   - songbird, squirrel, nestgate, beardog, toadstool (case-insensitive)
2. **Vendor Services**: 699 matches across 99 files
   - kubernetes, k8s, docker, consul, etcd, containerd
3. **Hardcoded Ports**: In `constants/network.rs`
   - SONGBIRD_PORT = 5000
   - BEARDOG_PORT = 6000
   - SQUIRREL_PORT = 7000
   - NESTGATE_PORT = 8000
   - REDIS_PORT = 6379
   - POSTGRES_PORT = 5432
   - MONGODB_PORT = 27017

### Good Patterns Found
From mature primals (songBird):
- ✅ Only defines OWN ports (DEFAULT_HTTP_PORT, DEFAULT_DISCOVERY_PORT)
- ✅ No hardcoded references to other primals' ports
- ✅ Uses capability-based discovery

From ToadStool existing code:
- ✅ `primal_identity.rs` - Already implements self-knowledge philosophy
- ✅ Capability enums defined (Compute, Storage, Auth, Coordination, Discovery)
- ✅ ServiceEndpoint system for discovered services

---

## 📋 Execution Plan

### Phase 1: Constants Evolution (High Priority) ⚡
**Goal**: Remove primal-specific constants, keep only self-knowledge

**Actions**:
1. ✅ Keep ToadStool's own ports (DEFAULT_HTTP_PORT, etc.)
2. ❌ Remove primal-specific ports (SONGBIRD_PORT, BEARDOG_PORT, etc.)
3. ⚠️ Move vendor ports to "fallback defaults" with clear documentation
4. ✅ Create ServiceDiscoveryDefaults struct for runtime discovery

**Impact**: ~50 files, medium complexity

### Phase 2: Primal Name Elimination (High Priority) ⚡
**Goal**: Replace hardcoded primal names with capability queries

**Pattern Transformation**:
```rust
// Before (hardcoded):
let songbird_endpoint = format!("http://localhost:{}", SONGBIRD_PORT);
client.connect(&songbird_endpoint)?;

// After (capability-based):
let coordination_service = discovery
    .find_service_by_capability(Capability::Coordination(
        CoordinationCapability::ServiceDiscovery
    ))
    .await?;
client.connect(&coordination_service.endpoint())?;
```

**Actions**:
1. Audit all primal name references (10,424 matches)
2. Categorize by context:
   - Tests (can use MockDiscovery)
   - Documentation (update to be agnostic)
   - Production code (replace with capability queries)
3. Create capability mapping:
   - "songbird" → Coordination::ServiceDiscovery
   - "nestgate" → Storage::ObjectStorage
   - "beardog" → Authentication::ServiceAuth
   - "squirrel" → Custom("resource-management")
4. Implement replacements systematically

**Impact**: ~875 files, high complexity

### Phase 3: Vendor Abstraction (Medium Priority)
**Goal**: Abstract vendor-specific implementations behind traits

**Pattern Transformation**:
```rust
// Before (vendor-hardcoded):
let k8s_client = KubernetesClient::new()?;
k8s_client.deploy(workload)?;

// After (abstracted):
let orchestrator = ContainerOrchestrator::discover().await?;
// Automatically selects k8s, docker, podman, or whatever is available
orchestrator.deploy(workload)?;
```

**Actions**:
1. Create abstraction traits:
   - `ContainerOrchestrator` (abstracts k8s, docker, podman)
   - `ServiceRegistry` (abstracts consul, etcd, zookeeper)
   - `MessageBroker` (abstracts rabbitmq, kafka, nats)
2. Implement discovery for each abstraction
3. Replace vendor-specific code with trait usage

**Impact**: ~99 files, medium complexity

### Phase 4: Infant Discovery Bootstrap (Low Priority)
**Goal**: Implement zero-knowledge startup sequence

**Bootstrap Sequence**:
```
1. Load minimal self-config (name, version, ports)
2. Start service discovery listeners (mDNS, DNS-SD, etc.)
3. Announce own capabilities
4. Listen for other services
5. Build dynamic routing table
6. Ready for workload execution
```

**Actions**:
1. Create `InfantBootstrap` module
2. Implement discovery protocols (mDNS, DNS-SD, capability broadcast)
3. Create dynamic service registry
4. Update startup sequence to use bootstrap

**Impact**: ~20 new files, high complexity

### Phase 5: Test Infrastructure Update (Medium Priority)
**Goal**: Update tests to use mock discovery instead of hardcoded values

**Actions**:
1. Create `MockDiscoveryService` for tests
2. Update test fixtures to register mock services
3. Replace hardcoded test URLs with discovered endpoints
4. Maintain backward compatibility with `#[cfg(test)]` flags

**Impact**: ~200 test files, low complexity (repetitive)

---

## 🚀 Implementation Priority

### Sprint 1: Foundation (This Session) - 2-3 hours
- [x] Create this plan
- [ ] Phase 1: Constants Evolution
  - [ ] Remove primal ports from `constants/network.rs`
  - [ ] Create `ServiceDiscoveryDefaults` struct
  - [ ] Update dependent code
- [ ] Document patterns for team

### Sprint 2: Core Transformation - 8-12 hours
- [ ] Phase 2: Primal Name Elimination (production code)
  - [ ] Create capability mapping helpers
  - [ ] Replace top 50 high-impact files
  - [ ] Run integration tests
- [ ] Phase 3: Start vendor abstraction
  - [ ] Create container orchestrator trait
  - [ ] Implement k8s + docker backends

### Sprint 3: Complete Migration - 10-15 hours
- [ ] Phase 2: Complete primal name elimination (all files)
- [ ] Phase 3: Complete vendor abstraction
- [ ] Phase 5: Update test infrastructure
- [ ] Full integration testing

### Sprint 4: Infant Discovery - 15-20 hours
- [ ] Phase 4: Implement bootstrap sequence
- [ ] Phase 4: Implement mDNS/DNS-SD discovery
- [ ] Phase 4: Dynamic service registry
- [ ] End-to-end testing of zero-knowledge startup

**Total Estimated Time**: 35-50 hours

---

## 📊 Success Metrics

### Quantitative
- [ ] Primal name references: 10,424 → 0 (in production code)
- [ ] Vendor-specific code: 699 → 0 (abstracted behind traits)
- [ ] Hardcoded ports: 13 primal/vendor ports → 0 (discovery-based)
- [ ] Self-knowledge only: ToadStool constants file = ToadStool ports only

### Qualitative
- [ ] Can start ToadStool with zero configuration
- [ ] Automatically discovers and connects to available services
- [ ] Works with any primal (songbird, alternative coordinators, etc.)
- [ ] Works with any container orchestrator (k8s, docker, podman)
- [ ] Tests run without hardcoded assumptions

---

## 🔍 Detailed Transformation Examples

### Example 1: Songbird Connection

**Before**:
```rust
// crates/integration/protocols/src/client.rs
use toadstool_common::constants::network::SONGBIRD_PORT;

pub async fn connect_to_songbird() -> Result<Client> {
    let endpoint = format!("http://localhost:{}", SONGBIRD_PORT);
    Client::connect(&endpoint).await
}
```

**After**:
```rust
// crates/integration/protocols/src/client.rs
use toadstool_common::discovery::ServiceDiscovery;
use toadstool_common::primal_identity::{Capability, CoordinationCapability};

pub async fn connect_to_coordination_service(
    discovery: &ServiceDiscovery
) -> Result<Client> {
    let service = discovery
        .find_service_by_capability(
            Capability::Coordination(CoordinationCapability::ServiceDiscovery)
        )
        .await?;
    Client::connect(&service.endpoint()).await
}
```

### Example 2: Container Orchestration

**Before**:
```rust
// crates/runtime/container/src/engines.rs
pub enum ContainerEngine {
    Docker,
    Kubernetes,
}

impl ContainerEngine {
    pub fn deploy(&self, spec: &ContainerSpec) -> Result<()> {
        match self {
            Self::Docker => docker::deploy(spec),
            Self::Kubernetes => k8s::deploy(spec),
        }
    }
}
```

**After**:
```rust
// crates/runtime/container/src/orchestrator.rs
pub trait ContainerOrchestrator: Send + Sync {
    async fn deploy(&self, spec: &ContainerSpec) -> Result<DeploymentId>;
    async fn status(&self, id: &DeploymentId) -> Result<DeploymentStatus>;
    async fn stop(&self, id: &DeploymentId) -> Result<()>;
}

// crates/runtime/container/src/discovery.rs
pub async fn discover_orchestrator() -> Result<Box<dyn ContainerOrchestrator>> {
    // Try to discover in order of preference
    if let Ok(orch) = KubernetesOrchestrator::discover().await {
        return Ok(Box::new(orch));
    }
    if let Ok(orch) = DockerOrchestrator::discover().await {
        return Ok(Box::new(orch));
    }
    if let Ok(orch) = PodmanOrchestrator::discover().await {
        return Ok(Box::new(orch));
    }
    Err(Error::NoOrchestratorAvailable)
}
```

### Example 3: NestGate Storage

**Before**:
```rust
// crates/integration/nestgate/src/client.rs
use toadstool_common::constants::network::NESTGATE_PORT;

pub async fn store_data(data: &[u8]) -> Result<()> {
    let nestgate_url = format!("http://localhost:{}", NESTGATE_PORT);
    let client = NestGateClient::new(&nestgate_url)?;
    client.store(data).await
}
```

**After**:
```rust
// crates/integration/storage/src/client.rs
use toadstool_common::discovery::ServiceDiscovery;
use toadstool_common::primal_identity::{Capability, StorageCapability};

pub async fn store_data(
    discovery: &ServiceDiscovery,
    data: &[u8]
) -> Result<()> {
    let storage_service = discovery
        .find_service_by_capability(
            Capability::Storage(StorageCapability::ObjectStorage)
        )
        .await?;
    
    let client = StorageClient::connect(&storage_service)?;
    client.store(data).await
}
```

---

## 🧪 Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use toadstool_testing::discovery::MockDiscoveryService;

    #[tokio::test]
    async fn test_coordination_service_connection() {
        let mut mock_discovery = MockDiscoveryService::new();
        mock_discovery.register_service(
            DiscoveredService {
                name: "test-coordinator".to_string(),
                capability: Capability::Coordination(
                    CoordinationCapability::ServiceDiscovery
                ),
                endpoint: ServiceEndpoint::http("localhost", 9000),
                metadata: HashMap::new(),
            }
        );

        let result = connect_to_coordination_service(&mock_discovery).await;
        assert!(result.is_ok());
    }
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_infant_discovery_bootstrap() {
    // Start ToadStool with zero configuration
    let toadstool = ToadStool::new_with_discovery().await?;
    
    // Should discover self
    assert!(toadstool.knows_self());
    
    // Should NOT know about other services yet
    assert_eq!(toadstool.known_services().len(), 0);
    
    // Start mock services
    let mock_coordinator = MockCoordinator::start().await?;
    let mock_storage = MockStorage::start().await?;
    
    // Wait for discovery
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Should have discovered services
    assert!(toadstool.known_services().len() >= 2);
    
    // Should be able to use discovered services
    let result = toadstool.execute_workload(test_workload).await;
    assert!(result.is_ok());
}
```

---

## 📚 Documentation Updates

### Files to Update
1. `README.md` - Add "Infant Discovery" section
2. `docs/ARCHITECTURE.md` - Document capability-based discovery
3. `specs/INTEGRATION.md` - Update integration patterns
4. Examples in `examples/` - Show discovery-based usage
5. `QUICK_START.md` - Zero-config startup guide

### New Documentation
1. `docs/INFANT_DISCOVERY.md` - Deep dive on discovery pattern
2. `docs/CAPABILITY_MAPPING.md` - How to map services to capabilities
3. `docs/VENDOR_ABSTRACTION.md` - Using trait-based abstraction
4. `examples/zero_config_startup.rs` - Minimal example

---

## ⚠️ Migration Considerations

### Backward Compatibility
- Keep deprecated constants with `#[deprecated]` attribute for 1 release
- Provide environment variable overrides for specific use cases
- Document migration path in CHANGELOG

### Performance
- Discovery has overhead - cache discovered services
- Implement TTL for service entries
- Use connection pooling for discovered endpoints

### Security
- Validate discovered service identities
- Use TLS for service communication
- Implement service authentication tokens

### Failure Handling
- Graceful degradation if discovery fails
- Fallback to localhost defaults in development
- Clear error messages for missing capabilities

---

## 🎯 This Session Goals (Sprint 1)

### Immediate Actions (Next 2-3 hours)

1. **Phase 1: Constants Evolution** ⚡
   - Remove primal ports from `constants/network.rs`
   - Create `discovery_defaults.rs` with fallback values
   - Update `primal_identity.rs` with ToadStool's self-knowledge
   - Document the pattern

2. **Create Discovery Infrastructure**
   - Implement `ServiceDiscovery` trait
   - Implement `find_service_by_capability()` helper
   - Create mock discovery for testing

3. **Pilot Migration**
   - Choose 5-10 high-impact files
   - Transform from hardcoded to discovery-based
   - Verify tests pass
   - Document transformations

**Success Criteria for This Session**:
- [ ] Constants file cleaned (no primal ports)
- [ ] Discovery infrastructure in place
- [ ] 5-10 files successfully migrated
- [ ] All tests passing
- [ ] Pattern documented for team

---

**Status**: Ready to execute Sprint 1

**Next Step**: Remove primal ports from `constants/network.rs`

