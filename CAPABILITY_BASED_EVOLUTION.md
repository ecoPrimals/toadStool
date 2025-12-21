# 🚀 Capability-Based Discovery Evolution

## Philosophy: "Know Yourself, Discover Others at Runtime"

### Core Principle
Each primal:
- ✅ **Knows**: Its own capabilities, ports, and identity
- ✅ **Discovers**: Other primals at runtime via capability matching
- ❌ **Never**: Hardcodes knowledge of other primals

## Current State Analysis

### ✅ Good Progress Already Made:
1. **Deprecated primal ports** with clear warnings
2. **RuntimeDiscovery** pattern documented
3. **Mock client** properly feature-gated for testing
4. **Clear migration path** in documentation

### ⚠️ Still Using Hardcoded Values:
```rust
// Found in 40+ locations:
- SONGBIRD_PORT: 8080 (deprecated ✅)
- BEARDOG_PORT: 8081 (deprecated ✅)  
- NESTGATE_PORT: 8082 (deprecated ✅)
- SQUIRREL_PORT: 8083 (deprecated ✅)

// Still referenced in:
- env_config.rs (backwards compatibility)
- config_utils.rs (migration helpers)
- defaults.rs (deprecated constants)
- tests (acceptable)
```

## Target Architecture

### 1. Self-Knowledge Pattern

```rust
// ✅ ToadStool knows itself
pub struct ToadStoolIdentity {
    pub name: "toadstool",
    pub capabilities: vec![
        Capability::UniversalCompute,
        Capability::RuntimeEnvironment,
        Capability::ResourceManagement,
    ],
    pub endpoints: SelfEndpoints {
        api_port: 8084,        // Self-knowledge OK
        metrics_port: 9090,     // Self-knowledge OK
        health_port: 8085,      // Self-knowledge OK
    },
}

// ❌ ToadStool doesn't know others
// No hardcoded: SONGBIRD_PORT, BEARDOG_PORT, etc.
```

### 2. Runtime Discovery Pattern

```rust
// ✅ Discover by capability, not by name/port
pub async fn coordinate_workload(&self) -> Result<()> {
    // Discover coordinator at runtime
    let coordinators = self.discovery
        .find_by_capability(&Capability::Coordination)
        .await?;
    
    let coordinator = coordinators
        .first()
        .ok_or("No coordinator found")?;
    
    // Use discovered endpoint
    coordinator.send_message(msg).await?;
}
```

### 3. Discovery Methods (Priority Order)

```rust
pub enum DiscoveryMethod {
    /// 1. Multicast DNS-SD (mDNS) - Automatic local network
    Multicast { group: "224.0.0.251:5353" },
    
    /// 2. Capability Registry - Centralized (optional)
    Registry { endpoint: discovered_or_env },
    
    /// 3. DNS-SD - Standard service discovery
    DnsSd { domain: "_primal._tcp.local" },
    
    /// 4. Environment Hints - User-provided (not hardcoded!)
    EnvironmentHint { 
        // User can provide: PRIMAL_COORDINATOR_ENDPOINT
        // But code doesn't assume/hardcode
    },
}
```

## Evolution Steps

### Phase 1: Complete Deprecation ✅ (Already Done!)
- [x] Mark all primal ports as deprecated
- [x] Document RuntimeDiscovery pattern
- [x] Feature-gate Mock for testing only

### Phase 2: Implement Full Discovery (In Progress)

#### Step 1: Enhance RuntimeDiscovery
```rust
// crates/core/common/src/runtime_discovery.rs
impl RuntimeDiscovery {
    /// Discover services by capability
    pub async fn discover_capability(
        &self,
        capability: &Capability,
    ) -> Result<Vec<ServiceInfo>> {
        let mut discovered = Vec::new();
        
        // Try multicast first (local network)
        if let Ok(services) = self.multicast_discover(capability).await {
            discovered.extend(services);
        }
        
        // Try DNS-SD (standard service discovery)
        if let Ok(services) = self.dns_sd_discover(capability).await {
            discovered.extend(services);
        }
        
        // Check environment hints (user-provided, not hardcoded)
        if let Ok(service) = self.env_hint_discover(capability).await {
            discovered.push(service);
        }
        
        Ok(discovered)
    }
}
```

#### Step 2: Remove Hardcoded Port Usage
```rust
// ❌ OLD: ecosystem.rs
let songbird_endpoint = format!("http://localhost:{}", SONGBIRD_PORT);

// ✅ NEW: ecosystem.rs
let coordinators = self.discovery
    .discover_capability(&Capability::Coordination)
    .await?;

if coordinators.is_empty() {
    warn!("No coordinator discovered - operating in standalone mode");
    // Graceful degradation
}
```

#### Step 3: Update EcosystemCoordinator
```rust
// crates/core/toadstool/src/ecosystem.rs
pub struct EcosystemCoordinator {
    /// Runtime discovery engine
    discovery: Arc<RuntimeDiscovery>,
    
    /// Discovered primals (dynamic, not hardcoded)
    primals: Arc<RwLock<HashMap<String, PrimalInstance>>>,
    
    /// No hardcoded endpoints!
    // ❌ primal_endpoints: HashMap<String, String>,
}

impl EcosystemCoordinator {
    pub async fn discover_primals(&self) -> Result<()> {
        // Discover by capability
        for capability in &[
            Capability::Coordination,
            Capability::Storage,
            Capability::Security,
            Capability::Intelligence,
        ] {
            match self.discovery.discover_capability(capability).await {
                Ok(services) => {
                    for service in services {
                        self.register_primal(service).await?;
                    }
                }
                Err(e) => {
                    debug!("No {} service found: {}", capability, e);
                    // Graceful: not all capabilities required
                }
            }
        }
        Ok(())
    }
}
```

### Phase 3: Environment Variable Evolution

#### Before:
```bash
# ❌ OLD: Hardcoded assumptions
TOADSTOOL_SONGBIRD_PORT=8080      # Assumes Songbird exists
TOADSTOOL_BEARDOG_PORT=8081       # Assumes BearDog exists
TOADSTOOL_NESTGATE_PORT=8082      # Assumes NestGate exists
```

#### After:
```bash
# ✅ NEW: Optional hints (not requirements)
TOADSTOOL_COORDINATOR_HINT=songbird.local:8080  # Optional user hint
TOADSTOOL_STORAGE_HINT=nestgate.local:8082      # Optional user hint

# ✅ Self-knowledge (always valid)
TOADSTOOL_API_PORT=8084
TOADSTOOL_METRICS_PORT=9090
TOADSTOOL_HEALTH_PORT=8085
```

### Phase 4: Mock Evolution

#### Current: Mock for no-networking
```rust
#[cfg(not(feature = "networking"))]
PrimalClient::Mock
```

#### Target: Mock only in tests
```rust
// Production: Always use real discovery
#[cfg(not(test))]
impl EcosystemCoordinator {
    async fn create_client(&self, endpoint: &str) -> Result<PrimalClient> {
        // Real HTTP/tRPC client
        Ok(PrimalClient::Http(reqwest::Client::new()))
    }
}

// Test: Use mock
#[cfg(test)]
impl EcosystemCoordinator {
    async fn create_client(&self, endpoint: &str) -> Result<PrimalClient> {
        Ok(PrimalClient::Mock(MockPrimalClient::new()))
    }
}
```

## Migration Path for Users

### For Developers:
```rust
// ❌ OLD CODE (deprecated)
use toadstool_config::defaults::network::SONGBIRD_PORT;
let endpoint = format!("http://localhost:{}", SONGBIRD_PORT);

// ✅ NEW CODE (capability-based)
use toadstool_common::{RuntimeDiscovery, Capability};
let discovery = RuntimeDiscovery::new(client);
let services = discovery
    .discover_capability(&Capability::Coordination)
    .await?;
```

### For Operators:
```bash
# ❌ OLD: Required environment variables
export TOADSTOOL_SONGBIRD_PORT=8080
export TOADSTOOL_BEARDOG_PORT=8081

# ✅ NEW: No required variables for discovery
# Automatic multicast discovery works!

# ✅ OPTIONAL: Hints for non-local or firewalled environments
export COORDINATOR_HINT=songbird-prod.company.com:443
export STORAGE_HINT=nestgate-prod.company.com:443
```

## Benefits

### 1. True Agnosticism ✅
- No assumptions about which primals exist
- No assumptions about where they run
- No assumptions about their ports

### 2. Dynamic Topology ✅
- Primals can come and go
- Automatic failover
- Load balancing across discovered instances

### 3. Zero Configuration ✅
- Works out-of-the-box with multicast
- No config files required
- Optional hints for complex networks

### 4. Security ✅
- No hardcoded credentials
- No exposed ports in code
- Discovery can use TLS/mTLS

### 5. Testing ✅
- Mock only in tests
- Production uses real discovery
- No feature flags for basic functionality

## Implementation Checklist

### High Priority:
- [ ] Implement multicast discovery in RuntimeDiscovery
- [ ] Implement DNS-SD discovery
- [ ] Update EcosystemCoordinator to use discovery
- [ ] Remove hardcoded port usage from ecosystem.rs
- [ ] Add graceful degradation for missing services

### Medium Priority:
- [ ] Deprecate old env vars (SONGBIRD_PORT, etc.)
- [ ] Add new hint-based env vars
- [ ] Update documentation
- [ ] Add migration guide

### Low Priority (Backwards Compatibility):
- [ ] Keep deprecated constants for 1-2 releases
- [ ] Log warnings when old constants used
- [ ] Provide automatic migration tool

## Testing Strategy

```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_discovery_multicast() {
        // Test multicast discovery
        let discovery = RuntimeDiscovery::new_with_multicast();
        let services = discovery
            .discover_capability(&Capability::Coordination)
            .await
            .unwrap();
        
        assert!(!services.is_empty());
    }
    
    #[tokio::test]
    async fn test_graceful_no_coordinator() {
        // Test that missing coordinator doesn't fail
        let coordinator = EcosystemCoordinator::new();
        let result = coordinator.discover_primals().await;
        
        // Should succeed even if no coordinator found
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_no_hardcoded_values() {
        // Ensure no hardcoded ports in production code
        let coordinator = EcosystemCoordinator::new();
        
        // Should not have any hardcoded primal knowledge
        assert_eq!(coordinator.hardcoded_endpoints().len(), 0);
    }
}
```

## Success Metrics

### Before Evolution:
- 210 hardcoded port references
- 40 uses of deprecated constants
- Fragile: breaks if primal ports change

### After Evolution:
- 0 hardcoded primal ports in production
- Dynamic discovery working
- Resilient: adapts to topology changes

### Target:
- 100% capability-based discovery
- 0% hardcoded primal knowledge
- 100% graceful degradation

## Timeline

- **Week 1**: Implement full RuntimeDiscovery (multicast + DNS-SD)
- **Week 2**: Migrate EcosystemCoordinator
- **Week 3**: Remove deprecated constants
- **Week 4**: Documentation and testing

## Conclusion

This evolution transforms ToadStool from a system with hardcoded assumptions to a truly agnostic, capability-based platform that:
- Knows only itself
- Discovers others dynamically
- Adapts to changing topology
- Requires zero configuration
- Provides optional hints for complex environments

**Philosophy**: Every primal is sovereign, self-aware, and discovers peers through capabilities, not assumptions.

---

*Created: December 9, 2025*
*Status: Design Complete, Implementation In Progress*

