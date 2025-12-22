# 🌐 Capability-Based Primal Discovery Pattern

**Created**: December 21, 2025  
**Status**: DESIGN SPECIFICATION  
**Purpose**: Replace hardcoded ports/addresses with runtime capability discovery

---

## 🎯 Design Principles

### 1. **Primal Sovereignty**
- Each primal knows ONLY itself at compile time
- Discovery happens at runtime
- Zero hardcoded coupling between primals

### 2. **Capability-First**
- Request by capability, not by service name
- "Find me something that can do X"
- Not "Connect to Songbird on port 8080"

### 3. **Zero Configuration**
- Works out of the box
- mDNS for local discovery
- Fallback to configured endpoints if needed

---

## 🏗️ Architecture

### Current (Hardcoded) ❌
```rust
// Hardcoded in code
const SONGBIRD_PORT: u16 = 8080;
const BEARDOG_PORT: u16 = 8081;

// Direct connection
let url = format!("http://localhost:{}", SONGBIRD_PORT);
let client = HttpClient::new(url);
```

**Problems**:
- Compile-time coupling
- Violates primal sovereignty
- Can't handle dynamic deployments
- Fails in containerized environments

### New (Discovery-Based) ✅
```rust
// No hardcoding - discover at runtime
let discovery = PrimalDiscovery::new().await?;
let endpoint = discovery.find_capability("orchestration").await?;
let client = HttpClient::new(endpoint.url());
```

**Benefits**:
- Runtime discovery
- Primal sovereignty maintained
- Works in any environment
- Handles dynamic topologies

---

## 📋 Implementation Plan

### Phase 1: Core Discovery Engine

#### File: `crates/core/common/src/primal_discovery.rs`

```rust
//! Primal Discovery via Capabilities
//!
//! Each primal discovers others at runtime based on capabilities,
//! maintaining primal sovereignty with zero compile-time coupling.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Primal endpoint discovered at runtime
#[derive(Clone, Debug)]
pub struct PrimalEndpoint {
    /// Service identifier (e.g., "songbird-main")
    pub service_id: String,
    
    /// Capabilities this endpoint provides
    pub capabilities: Vec<String>,
    
    /// Connection URL (http://, https://, grpc://)
    pub url: String,
    
    /// Trust level (local, verified, unverified)
    pub trust_level: TrustLevel,
    
    /// Discovery method used
    pub discovered_via: DiscoveryMethod,
    
    /// When discovered
    pub discovered_at: Instant,
    
    /// Last successful health check
    pub last_seen: Instant,
    
    /// Average latency (milliseconds)
    pub latency_ms: u64,
}

impl PrimalEndpoint {
    /// Check if endpoint is still fresh
    pub fn is_fresh(&self, max_age: Duration) -> bool {
        self.last_seen.elapsed() < max_age
    }
    
    /// Get connection URL
    pub fn url(&self) -> &str {
        &self.url
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TrustLevel {
    /// Local network (mDNS)
    Local,
    
    /// Verified via cryptographic proof
    Verified,
    
    /// Discovered but not verified
    Unverified,
}

#[derive(Clone, Debug)]
pub enum DiscoveryMethod {
    /// mDNS/DNS-SD local discovery
    MDns,
    
    /// Configured endpoint
    Configuration,
    
    /// Discovered via another primal
    Referral { from: String },
}

/// Primal discovery engine
pub struct PrimalDiscovery {
    /// mDNS discovery backend
    mdns: Arc<crate::mdns::MdnsDiscovery>,
    
    /// Discovered endpoints cache
    cache: Arc<RwLock<HashMap<String, Vec<PrimalEndpoint>>>>,
    
    /// Configuration overrides
    config: Arc<DiscoveryConfig>,
}

#[derive(Clone)]
pub struct DiscoveryConfig {
    /// Cache TTL
    pub cache_ttl: Duration,
    
    /// Health check interval
    pub health_check_interval: Duration,
    
    /// Configured fallback endpoints
    pub fallbacks: HashMap<String, String>,
    
    /// Enable mDNS discovery
    pub enable_mdns: bool,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            cache_ttl: Duration::from_secs(300), // 5 minutes
            health_check_interval: Duration::from_secs(30),
            fallbacks: HashMap::new(),
            enable_mdns: true,
        }
    }
}

impl PrimalDiscovery {
    /// Create new discovery engine
    pub async fn new() -> Result<Self, DiscoveryError> {
        Self::with_config(DiscoveryConfig::default()).await
    }
    
    /// Create with custom configuration
    pub async fn with_config(config: DiscoveryConfig) -> Result<Self, DiscoveryError> {
        let mdns = Arc::new(crate::mdns::MdnsDiscovery::new()?);
        
        Ok(Self {
            mdns,
            cache: Arc::new(RwLock::new(HashMap::new())),
            config: Arc::new(config),
        })
    }
    
    /// Discover service by capability
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use toadstool_common::primal_discovery::PrimalDiscovery;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let discovery = PrimalDiscovery::new().await?;
    /// 
    /// // Find orchestration service (e.g., Songbird)
    /// let endpoint = discovery.find_capability("orchestration").await?;
    /// println!("Found: {}", endpoint.url());
    /// 
    /// // Find security service (e.g., BearDog)
    /// let endpoint = discovery.find_capability("security").await?;
    /// println!("Found: {}", endpoint.url());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn find_capability(&self, capability: &str) -> Result<PrimalEndpoint, DiscoveryError> {
        // 1. Check cache
        if let Some(cached) = self.get_from_cache(capability).await {
            if cached.is_fresh(self.config.cache_ttl) {
                tracing::debug!("Cache hit for capability: {}", capability);
                return Ok(cached);
            }
        }
        
        // 2. Try mDNS discovery
        if self.config.enable_mdns {
            if let Ok(endpoints) = self.discover_via_mdns(capability).await {
                if let Some(best) = self.select_best(&endpoints) {
                    self.cache_endpoint(capability, best.clone()).await;
                    return Ok(best);
                }
            }
        }
        
        // 3. Try configured fallback
        if let Some(url) = self.config.fallbacks.get(capability) {
            let endpoint = PrimalEndpoint {
                service_id: format!("{}-fallback", capability),
                capabilities: vec![capability.to_string()],
                url: url.clone(),
                trust_level: TrustLevel::Local,
                discovered_via: DiscoveryMethod::Configuration,
                discovered_at: Instant::now(),
                last_seen: Instant::now(),
                latency_ms: 0,
            };
            
            self.cache_endpoint(capability, endpoint.clone()).await;
            return Ok(endpoint);
        }
        
        // 4. Not found
        Err(DiscoveryError::NotFound {
            capability: capability.to_string(),
        })
    }
    
    /// Discover all services with capability
    pub async fn find_all_with_capability(&self, capability: &str) -> Result<Vec<PrimalEndpoint>, DiscoveryError> {
        self.discover_via_mdns(capability).await
    }
    
    /// Refresh discovery (force re-scan)
    pub async fn refresh(&self) -> Result<(), DiscoveryError> {
        self.cache.write().await.clear();
        Ok(())
    }
    
    // Internal helpers
    
    async fn get_from_cache(&self, capability: &str) -> Option<PrimalEndpoint> {
        let cache = self.cache.read().await;
        cache.get(capability)
            .and_then(|endpoints| endpoints.first())
            .cloned()
    }
    
    async fn cache_endpoint(&self, capability: &str, endpoint: PrimalEndpoint) {
        let mut cache = self.cache.write().await;
        cache.entry(capability.to_string())
            .or_insert_with(Vec::new)
            .push(endpoint);
    }
    
    async fn discover_via_mdns(&self, capability: &str) -> Result<Vec<PrimalEndpoint>, DiscoveryError> {
        // Query mDNS for services advertising this capability
        let services = self.mdns.discover_services(capability).await?;
        
        let mut endpoints = Vec::new();
        for service in services {
            endpoints.push(PrimalEndpoint {
                service_id: service.service_id,
                capabilities: service.capabilities,
                url: service.url,
                trust_level: TrustLevel::Local, // mDNS is local network
                discovered_via: DiscoveryMethod::MDns,
                discovered_at: Instant::now(),
                last_seen: Instant::now(),
                latency_ms: service.latency_ms.unwrap_or(0),
            });
        }
        
        Ok(endpoints)
    }
    
    fn select_best(&self, endpoints: &[PrimalEndpoint]) -> Option<PrimalEndpoint> {
        // Selection criteria:
        // 1. Prefer verified > local > unverified
        // 2. Prefer lower latency
        // 3. Prefer more recently seen
        
        endpoints.iter()
            .min_by_key(|e| (
                match e.trust_level {
                    TrustLevel::Verified => 0,
                    TrustLevel::Local => 1,
                    TrustLevel::Unverified => 2,
                },
                e.latency_ms,
                e.last_seen.elapsed().as_secs(),
            ))
            .cloned()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("Capability not found: {capability}")]
    NotFound { capability: String },
    
    #[error("mDNS error: {0}")]
    MDnsError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
}
```

---

### Phase 2: Primal-Specific Adapters

#### File: `crates/core/common/src/primal_adapters.rs`

```rust
//! High-level adapters for common primal interactions

use super::primal_discovery::{PrimalDiscovery, PrimalEndpoint};

/// Orchestration service adapter (typically Songbird)
pub struct OrchestrationAdapter {
    discovery: PrimalDiscovery,
    endpoint: Option<PrimalEndpoint>,
}

impl OrchestrationAdapter {
    pub async fn new(discovery: PrimalDiscovery) -> Result<Self, AdapterError> {
        Ok(Self {
            discovery,
            endpoint: None,
        })
    }
    
    /// Get orchestration endpoint
    async fn endpoint(&mut self) -> Result<&PrimalEndpoint, AdapterError> {
        if self.endpoint.is_none() || !self.endpoint.as_ref().unwrap().is_fresh(Duration::from_secs(300)) {
            self.endpoint = Some(self.discovery.find_capability("orchestration").await?);
        }
        Ok(self.endpoint.as_ref().unwrap())
    }
    
    /// Register workload
    pub async fn register_workload(&mut self, spec: WorkloadSpec) -> Result<String, AdapterError> {
        let endpoint = self.endpoint().await?;
        // Use endpoint.url() for HTTP request
        // Implementation here...
        Ok("job-id".to_string())
    }
}

/// Security service adapter (typically BearDog)
pub struct SecurityAdapter {
    discovery: PrimalDiscovery,
    endpoint: Option<PrimalEndpoint>,
}

impl SecurityAdapter {
    pub async fn new(discovery: PrimalDiscovery) -> Result<Self, AdapterError> {
        Ok(Self {
            discovery,
            endpoint: None,
        })
    }
    
    async fn endpoint(&mut self) -> Result<&PrimalEndpoint, AdapterError> {
        if self.endpoint.is_none() || !self.endpoint.as_ref().unwrap().is_fresh(Duration::from_secs(300)) {
            self.endpoint = Some(self.discovery.find_capability("security").await?);
        }
        Ok(self.endpoint.as_ref().unwrap())
    }
    
    /// Request secure tunnel
    pub async fn request_tunnel(&mut self) -> Result<TunnelId, AdapterError> {
        let endpoint = self.endpoint().await?;
        // Use endpoint.url() for request
        // Implementation here...
        Ok(TunnelId::new())
    }
}

/// Storage service adapter (typically NestGate)
pub struct StorageAdapter {
    discovery: PrimalDiscovery,
    endpoint: Option<PrimalEndpoint>,
}

// Similar pattern...

#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("Discovery failed: {0}")]
    DiscoveryFailed(String),
    
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
}
```

---

### Phase 3: Migration Path

#### Step 1: Add Discovery Alongside Hardcoded (Parallel)
```rust
// Both work simultaneously
let url = if let Ok(discovery) = PrimalDiscovery::new().await {
    if let Ok(endpoint) = discovery.find_capability("orchestration").await {
        endpoint.url().to_string()
    } else {
        format!("http://localhost:{}", DEFAULT_SONGBIRD_PORT) // Fallback
    }
} else {
    format!("http://localhost:{}", DEFAULT_SONGBIRD_PORT) // Fallback
};
```

#### Step 2: Make Discovery Primary, Hardcoded Fallback
```rust
let discovery = PrimalDiscovery::new().await?;
let endpoint = discovery.find_capability("orchestration").await
    .or_else(|_| {
        // Fallback to configured default
        Ok(PrimalEndpoint::from_url("http://localhost:8080")?)
    })?;
```

#### Step 3: Pure Discovery (Remove Hardcoding)
```rust
// Only discovery, no hardcoded fallbacks
let discovery = PrimalDiscovery::new().await?;
let endpoint = discovery.find_capability("orchestration").await?;
```

---

## 🧪 Testing Strategy

### Unit Tests
```rust
#[tokio::test]
async fn test_discovery_finds_service() {
    let mut config = DiscoveryConfig::default();
    config.fallbacks.insert(
        "orchestration".to_string(),
        "http://localhost:8080".to_string()
    );
    
    let discovery = PrimalDiscovery::with_config(config).await.unwrap();
    let endpoint = discovery.find_capability("orchestration").await.unwrap();
    
    assert_eq!(endpoint.url(), "http://localhost:8080");
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_mdns_discovery() {
    // Start mock mDNS service
    let _mock = start_mock_mdns_service("orchestration", 8080).await;
    
    let discovery = PrimalDiscovery::new().await.unwrap();
    let endpoint = discovery.find_capability("orchestration").await.unwrap();
    
    assert!(endpoint.url().contains("8080"));
    assert_eq!(endpoint.discovered_via, DiscoveryMethod::MDns);
}
```

---

## 📊 Implementation Phases

### Phase 1: Foundation (1 week)
- ✅ Design complete
- ⬜ Implement `PrimalDiscovery`
- ⬜ Implement `PrimalEndpoint`
- ⬜ Integrate with existing mDNS
- ⬜ Unit tests

### Phase 2: Adapters (1 week)
- ⬜ OrchestrationAdapter
- ⬜ SecurityAdapter
- ⬜ StorageAdapter
- ⬜ Integration tests

### Phase 3: Migration (1-2 weeks)
- ⬜ Add to CLI (parallel with hardcoded)
- ⬜ Add to server (parallel with hardcoded)
- ⬜ Add to distributed (parallel with hardcoded)
- ⬜ Test in all modes
- ⬜ Make discovery primary
- ⬜ Remove hardcoded values

### Phase 4: Production (ongoing)
- ⬜ Monitor discovery latency
- ⬜ Tune cache TTLs
- ⬜ Add metrics
- ⬜ Handle edge cases

---

## 🎯 Success Criteria

1. ✅ Zero hardcoded ports in production code
2. ✅ Zero compile-time primal coupling
3. ✅ Works in development (localhost)
4. ✅ Works in production (mDNS)
5. ✅ Works in containers (environment discovery)
6. ✅ Graceful degradation (fallbacks work)
7. ✅ Fast (<100ms typical discovery)
8. ✅ Reliable (99.9% success rate)

---

## 📝 Configuration Example

```toml
# toadstool.toml
[discovery]
# Enable mDNS discovery
enable_mdns = true

# Cache TTL (seconds)
cache_ttl = 300

# Health check interval (seconds)
health_check_interval = 30

# Fallback endpoints (for development/testing)
[discovery.fallbacks]
orchestration = "http://localhost:8080"
security = "http://localhost:8081"
storage = "http://localhost:8082"
ai_coordination = "http://localhost:8083"
```

---

## 🚀 Benefits

### For Development
- Works out of box on localhost
- Can override with config for testing
- Fast iteration (no rebuilds for endpoints)

### For Production
- Auto-discovers services via mDNS
- Handles dynamic scaling
- Works in any network topology
- Container-friendly

### For Architecture
- True primal sovereignty
- Zero compile-time coupling
- Extensible (new primals auto-discovered)
- Future-proof (new capabilities work immediately)

---

**Status**: READY TO IMPLEMENT  
**Estimated Effort**: 3-4 weeks  
**Priority**: HIGH (enables true sovereignty)  
**Dependencies**: Existing mDNS implementation (already exists!)

---

*Next Step*: Begin Phase 1 implementation

