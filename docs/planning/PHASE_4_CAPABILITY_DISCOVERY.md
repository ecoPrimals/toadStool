//! # Capability-Based Discovery - Evolution Plan
//!
//! Plan for evolving from hardcoded fallback ports to full capability-based discovery.

## Current State (Phase 1-3: 90% Complete)

### ✅ Phase 1: Centralized Configuration
- All ports centralized in `crates/core/config/src/ports.rs`
- Single source of truth for configuration
- **Status**: COMPLETE

### ✅ Phase 2: Environment Variable Overrides
- `TOADSTOOL_*_PORT` for own services
- `{PRIMAL}_PORT` and `{PRIMAL}_ENDPOINT` for others
- Docker/K8s ready
- **Status**: COMPLETE

### ✅ Phase 3: Songbird Discovery Integration
- Runtime discovery via Songbird
- Capability-based service location
- Fallback to env vars if discovery fails
- **Status**: 95% COMPLETE

### 🟡 Phase 4: Zero-Hardcoding (Pending)
- mDNS/DNS-SD for local networks
- Pure capability-based discovery
- Remove fallback ports entirely
- **Status**: 0% - PLANNED

---

## Phase 4 Implementation Plan

### Architecture Design

```rust
//! Discovery hierarchy (attempt in order):
//!
//! 1. Explicit configuration (env vars, config files)
//! 2. Songbird capability discovery (network orchestrator)
//! 3. mDNS/DNS-SD (local network, zero-config)
//! 4. ERROR - no fallback ports!

pub async fn discover_service(
    capability: &str,
    config: &DiscoveryConfig,
) -> Result<ServiceEndpoint, DiscoveryError> {
    // 1. Check explicit configuration first
    if let Some(endpoint) = check_explicit_config(capability, config)? {
        return Ok(endpoint);
    }
    
    // 2. Try Songbird orchestrator
    if let Ok(endpoint) = discover_via_songbird(capability).await {
        return Ok(endpoint);
    }
    
    // 3. Try mDNS/DNS-SD for local network
    if let Ok(endpoint) = discover_via_mdns(capability).await {
        return Ok(endpoint);
    }
    
    // 4. No fallback - fail explicitly
    Err(DiscoveryError::ServiceNotFound {
        capability: capability.to_string(),
        tried: vec!["config", "songbird", "mdns"],
    })
}
```

### Implementation Steps

#### Step 1: mDNS Discovery (Week 1)

**Dependencies**:
```toml
[dependencies]
mdns-sd = "0.10"  # Pure Rust mDNS implementation
```

**Implementation**:
```rust
// crates/core/config/src/discovery/mdns.rs

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use std::time::Duration;

pub struct MdnsDiscovery {
    daemon: ServiceDaemon,
}

impl MdnsDiscovery {
    pub fn new() -> Result<Self> {
        let daemon = ServiceDaemon::new()?;
        Ok(Self { daemon })
    }
    
    /// Discover service by capability
    pub async fn discover(&self, capability: &str) -> Result<ServiceEndpoint> {
        // Service type: _toadstool-{capability}._tcp.local.
        let service_type = format!("_toadstool-{}._tcp.local.", capability);
        
        let receiver = self.daemon.browse(&service_type)?;
        
        // Wait up to 5 seconds for discovery
        let timeout = tokio::time::timeout(
            Duration::from_secs(5),
            self.wait_for_service(receiver)
        ).await??;
        
        Ok(timeout)
    }
    
    async fn wait_for_service(
        &self,
        mut receiver: mpsc::Receiver<ServiceEvent>,
    ) -> Result<ServiceEndpoint> {
        while let Some(event) = receiver.recv().await {
            match event {
                ServiceEvent::ServiceResolved(info) => {
                    return self.parse_service_info(&info);
                }
                _ => continue,
            }
        }
        
        Err(DiscoveryError::Timeout)
    }
    
    /// Register our own service for discovery
    pub fn register_service(
        &self,
        capability: &str,
        port: u16,
    ) -> Result<()> {
        let service_type = format!("_toadstool-{}._tcp.local.", capability);
        let instance_name = format!("toadstool-{}-{}", capability, uuid::Uuid::new_v4());
        
        let service = ServiceInfo::new(
            &service_type,
            &instance_name,
            &hostname()?,
            "",  // No address (auto-resolved)
            port,
            &[("capability", capability)][..],
        )?;
        
        self.daemon.register(service)?;
        Ok(())
    }
}
```

#### Step 2: DNS-SD Discovery (Week 2)

```rust
// crates/core/config/src/discovery/dns_sd.rs

use trust_dns_resolver::Resolver;

pub struct DnsSdDiscovery {
    resolver: Resolver,
}

impl DnsSdDiscovery {
    pub fn new() -> Result<Self> {
        let resolver = Resolver::from_system_conf()?;
        Ok(Self { resolver })
    }
    
    pub async fn discover(&self, capability: &str) -> Result<ServiceEndpoint> {
        // Query SRV records
        let query = format!("_toadstool-{capability}._tcp.local");
        
        let response = self.resolver
            .srv_lookup(&query)
            .await?;
        
        // Get first available service
        let srv = response.iter().next()
            .ok_or(DiscoveryError::NotFound)?;
        
        Ok(ServiceEndpoint {
            host: srv.target().to_string(),
            port: srv.port(),
            capability: capability.to_string(),
        })
    }
}
```

#### Step 3: Remove Fallback Ports (Week 3)

**Delete**:
```rust
// crates/core/config/src/ports.rs

// DELETE THIS ENTIRE MODULE:
pub mod fallback {
    pub const SONGBIRD: u16 = 8080;  // REMOVE
    pub const SQUIRREL: u16 = 8083;  // REMOVE
    pub const BEARDOG: u16 = 8081;   // REMOVE
    pub const NESTGATE: u16 = 8082;  // REMOVE
}
```

**Replace with**:
```rust
// Force explicit discovery
pub fn get_primal_endpoint(primal: &str) -> Result<String> {
    // Only check env var - no fallback!
    std::env::var(format!("{}_ENDPOINT", primal))
        .map_err(|_| ConfigError::PrimalNotConfigured(primal.to_string()))
}
```

#### Step 4: Integration & Testing (Week 4)

```rust
// crates/core/config/src/discovery/mod.rs

pub async fn discover_service(
    capability: &str,
) -> Result<ServiceEndpoint> {
    // Try all discovery methods
    let discovery = ServiceDiscovery::new()?;
    
    // 1. Explicit config
    if let Some(endpoint) = discovery.from_config(capability)? {
        tracing::info!("Service {} found via explicit config", capability);
        return Ok(endpoint);
    }
    
    // 2. Songbird
    if let Ok(endpoint) = discovery.from_songbird(capability).await {
        tracing::info!("Service {} found via Songbird", capability);
        return Ok(endpoint);
    }
    
    // 3. mDNS
    if let Ok(endpoint) = discovery.from_mdns(capability).await {
        tracing::info!("Service {} found via mDNS", capability);
        return Ok(endpoint);
    }
    
    // 4. DNS-SD
    if let Ok(endpoint) = discovery.from_dns_sd(capability).await {
        tracing::info!("Service {} found via DNS-SD", capability);
        return Ok(endpoint);
    }
    
    // FAIL - no fallback
    Err(DiscoveryError::ServiceNotFound {
        capability: capability.to_string(),
        methods_tried: vec!["config", "songbird", "mdns", "dns-sd"],
    })
}
```

---

## Benefits of Phase 4

### 1. True Self-Knowledge ✅
- ToadStool knows only itself
- Other services discovered at runtime
- No hardcoded assumptions
- **Principle upheld**: 100%

### 2. Zero-Config Local Development
```bash
# Start ToadStool
cargo run --bin toadstool-server

# Start Songbird in another terminal
cd ../songbird && cargo run

# They discover each other automatically via mDNS!
# No configuration needed!
```

### 3. Production Flexibility
```bash
# Production: Use explicit endpoints
export SONGBIRD_ENDPOINT=https://songbird.prod.example.com:8080

# Staging: Use Songbird orchestrator
export SONGBIRD_ENDPOINT=http://songbird-staging:8080

# Development: Use mDNS auto-discovery
# (no config needed)
```

### 4. Failure Modes Are Explicit
```rust
// OLD (with fallbacks):
let endpoint = get_primal_endpoint("SONGBIRD");  // Always succeeds

// NEW (without fallbacks):
let endpoint = discover_service("orchestration").await?;
// ^ Fails explicitly if not configured/discoverable
```

---

## Migration Strategy

### Backward Compatibility Period (1-2 months)

```rust
// Feature flag for migration period
#[cfg(feature = "legacy-fallbacks")]
pub mod fallback {
    // Keep fallback ports temporarily
}

// Emit warnings when fallbacks are used
pub fn get_primal_port(primal: &str, fallback: u16) -> u16 {
    #[cfg(feature = "legacy-fallbacks")]
    {
        tracing::warn!(
            "Using fallback port {} for {}. Configure {} via environment!",
            fallback,
            primal,
            format!("{}_ENDPOINT", primal)
        );
        fallback
    }
    
    #[cfg(not(feature = "legacy-fallbacks"))]
    {
        panic!("Fallback ports disabled. Configure {} via {}",
               primal, format!("{}_ENDPOINT", primal));
    }
}
```

### Deprecation Timeline

**Month 1**: Add warnings
```
WARN: Using fallback port 8080 for SONGBIRD. 
      Configure SONGBIRD_ENDPOINT via environment!
      Fallbacks will be removed in version 0.3.0
```

**Month 2**: Make fallbacks opt-in
```toml
# Require explicit feature flag
[features]
legacy-fallbacks = []  # Disabled by default
```

**Month 3**: Remove fallbacks entirely
```rust
// ports.rs no longer has fallback module
// All discovery must be explicit
```

---

## Testing Strategy

### Unit Tests
```rust
#[tokio::test]
async fn test_mdns_discovery() {
    let discovery = MdnsDiscovery::new().unwrap();
    
    // Register test service
    discovery.register_service("test-capability", 9999).unwrap();
    
    // Discover it
    let endpoint = discovery.discover("test-capability").await.unwrap();
    assert_eq!(endpoint.port, 9999);
}

#[tokio::test]
async fn test_discovery_hierarchy() {
    // Test that discovery tries methods in order
    let discovery = ServiceDiscovery::new().unwrap();
    
    // Should try: config -> songbird -> mdns -> dns-sd -> error
    let result = discovery.discover("nonexistent").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_explicit_config_takes_precedence() {
    std::env::set_var("TEST_ENDPOINT", "http://explicit:1234");
    
    let discovery = ServiceDiscovery::new().unwrap();
    let endpoint = discovery.discover("test").await.unwrap();
    
    assert_eq!(endpoint.host, "explicit");
    assert_eq!(endpoint.port, 1234);
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_mdns_auto_discovery() {
    // Start mock Songbird service with mDNS
    let songbird = MockSongbird::new_with_mdns(8080).await;
    
    // ToadStool should discover it automatically
    let toadstool = ToadStool::new().await.unwrap();
    let endpoint = toadstool.discover_orchestration().await.unwrap();
    
    assert_eq!(endpoint.port, 8080);
}
```

---

## Success Criteria

### Phase 4 Complete When:
- [ ] mDNS discovery implemented and tested
- [ ] DNS-SD discovery implemented and tested
- [ ] Fallback ports removed
- [ ] All discovery methods integrated
- [ ] Tests passing (unit + integration)
- [ ] Documentation updated
- [ ] Migration guide published
- [ ] 100% self-knowledge achieved ✅

### Metrics
- **Self-Knowledge**: 90% → 100%
- **Configuration Flexibility**: High
- **Local Development UX**: Excellent (zero-config)
- **Production Clarity**: Explicit failures
- **Backward Compatibility**: Feature-flagged

---

## Timeline

**Week 1**: mDNS implementation  
**Week 2**: DNS-SD implementation  
**Week 3**: Remove fallbacks, integration  
**Week 4**: Testing, documentation, polish

**Total**: 4 weeks (1 month)

---

## Alternative Approaches Considered

### 1. Keep Fallbacks Forever
**Rejected**: Violates self-knowledge principle

### 2. Remove Fallbacks Immediately
**Rejected**: Too disruptive, need migration period

### 3. Use Consul/etcd for Discovery
**Rejected**: Adds external dependency, mDNS simpler

### 4. Chosen: Incremental Evolution ✅
- Add Phase 4 capabilities
- Deprecate fallbacks gradually
- Remove after migration period
- **Best balance of pragmatism and principles**

---

## Conclusion

Phase 4 discovery will achieve 100% self-knowledge while maintaining pragmatic deployment options. The migration path is clear, the benefits are substantial, and the implementation is straightforward.

**Status**: Ready to implement  
**Priority**: MEDIUM (after test coverage)  
**Risk**: LOW (well-designed)  
**Impact**: HIGH (architectural purity)

🍄 **Evolution complete: From hardcoded to capability-based!**

